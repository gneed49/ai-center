#!/usr/bin/env python3
"""Prepare and validate the disposable Supabase target without reading .env files."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib


class TargetError(Exception):
    pass


def project_id(repo: Path) -> str:
    return "ai-center-ci-" + hashlib.sha256(str(repo.resolve()).encode()).hexdigest()[:12]


def workdir(repo: Path, env: dict[str, str]) -> Path:
    expected = repo.resolve() / ".run" / "integration-stack"
    actual = Path(env.get("AI_CENTER_INTEGRATION_WORKDIR", str(expected)))
    if actual != expected or actual.resolve() != expected:
        raise TargetError("le workdir doit être .run/integration-stack dans ce checkout, sans lien symbolique")
    return actual


def config_text(repo: Path) -> str:
    text = (repo / "supabase/config.integration.toml").read_text()
    if text.count('project_id = "AI_CENTER_CI_PROJECT_ID"') != 1:
        raise TargetError("identité du template CI invalide")
    text = text.replace('project_id = "AI_CENTER_CI_PROJECT_ID"', f'project_id = "{project_id(repo)}"')
    config = tomllib.loads(text)
    development = tomllib.loads((repo / "supabase/config.toml").read_text())

    def ports(value: dict) -> set[int]:
        found: set[int] = set()
        for key, item in value.items():
            if isinstance(item, dict):
                found.update(ports(item))
            elif key == "port" or key.endswith("_port"):
                found.add(item)
        return found

    if config["project_id"] == development["project_id"] or ports(config) & ports(development):
        raise TargetError("l’identité et tous les ports CI doivent différer du développement")
    if config["db"]["port"] != 55322 or config["api"]["port"] != 55321:
        raise TargetError("ports du contrat CI inattendus")
    return text


def validate_env(env: dict[str, str]) -> None:
    for name, user in (("AI_CENTER_ADMIN_DATABASE_URL", "postgres"),
                       ("AI_CENTER_RUNTIME_DATABASE_URL", "ai_center_runtime"),
                       ("DATABASE_URL", "ai_center_runtime")):
        value = env.get(name)
        if value is not None and not re.fullmatch(
            rf"postgresql://{user}:[A-Za-z0-9_.~%-]+@127\.0\.0\.1:55322/postgres", value
        ):
            raise TargetError(f"{name} doit désigner le rôle prévu sur la base CI loopback 55322/postgres")
    expected = {
        "AI_CENTER_LOCAL_POSTGRES_PORT": "55322",
        "AI_CENTER_AUTH_SMOKE_BIND": "127.0.0.1:4618",
        "AI_CENTER_REAL_E2E_API_URL": "http://127.0.0.1:4617",
        "AI_CENTER_REAL_E2E_WEB_URL": "http://127.0.0.1:5183",
        "AI_CENTER_AGENT_MODE": "deterministic",
    }
    for name, value in expected.items():
        if name in env and env[name] != value:
            raise TargetError(f"{name} ne correspond pas au contrat CI isolé")


def guard(repo: Path, env: dict[str, str]) -> Path:
    target = workdir(repo, env)
    validate_env(env)
    expected_marker = {"version": 1, "repo": str(repo.resolve()), "project_id": project_id(repo)}
    for path in (target / "target.json", target / "supabase/config.toml"):
        if path.is_symlink() or path.resolve() != path:
            raise TargetError("la configuration CI ne doit contenir aucun lien symbolique")
    if json.loads((target / "target.json").read_text()) != expected_marker:
        raise TargetError("le marqueur d’appartenance CI est invalide")
    if (target / "supabase/config.toml").read_text() != config_text(repo):
        raise TargetError("la configuration CI a changé ; reset/stop refusé")
    if env.get("AI_CENTER_SUPABASE_DB_CONTAINER", f"supabase_db_{project_id(repo)}") != f"supabase_db_{project_id(repo)}":
        raise TargetError("le conteneur PostgreSQL n’appartient pas à cette stack CI")
    return target


def prepare(repo: Path, env: dict[str, str]) -> Path:
    validate_env(env)
    target = workdir(repo, env)
    text = config_text(repo)
    if target.exists():
        guard(repo, env)
    target.mkdir(parents=True, exist_ok=True, mode=0o700)
    destination = target / "supabase"
    destination.mkdir(exist_ok=True, mode=0o700)
    # Copy only versioned database inputs. In particular never copy .env, .temp,
    # linked-project metadata or the development config.
    for name in ("roles.sql", "seed.sql", "migrations", "schemas", "tests"):
        source = repo / "supabase" / name
        copied = destination / name
        paths = [source, *source.rglob("*")] if source.is_dir() else [source]
        if any(path.is_symlink() for path in paths):
            raise TargetError("un input Supabase contient un lien symbolique")
        if copied.is_symlink() or copied.resolve() != copied:
            raise TargetError("un input CI pointe hors de la stack")
        if source.is_dir():
            if copied.exists():
                if any(path.is_symlink() for path in copied.rglob("*")):
                    raise TargetError("un input CI contient un lien symbolique")
                shutil.rmtree(copied)
            shutil.copytree(source, copied)
        else:
            shutil.copyfile(source, copied)
    (destination / "config.toml").write_text(text)
    (target / "target.json").write_text(json.dumps({
        "version": 1, "repo": str(repo.resolve()), "project_id": project_id(repo),
    }) + "\n")
    # Stop dotenv's upward search when the API runs from this workdir.
    for name in (".env", ".env.local"):
        path = target / name
        if path.is_symlink():
            raise TargetError("un fichier dotenv CI est un lien symbolique")
        path.write_text("")
    return guard(repo, env)


def validate_status(status: dict) -> None:
    # CLI output is private; report only the field name on failure.
    if not isinstance(status, dict):
        raise TargetError("le statut CLI doit être un objet")
    for name, expected in {
        "DB_URL": "postgresql://postgres:postgres@127.0.0.1:55322/postgres",
        "API_URL": "http://127.0.0.1:55321",
    }.items():
        if status.get(name) != expected:
            raise TargetError(f"le statut de la stack ne confirme pas {name} attendu")


def cleanup_podman(repo: Path, env: dict[str, str]) -> None:
    """Recover only the known Podman prune incompatibility, on the same socket."""
    directory = guard(repo, env)
    errors = []
    for line in (directory / "stop-cli.log").read_text().splitlines():
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if isinstance(record, dict) and isinstance(record.get("error"), dict):
            errors.append(record["error"].get("code"))
    if errors != ["LegacyStopVolumePruneError"]:
        raise TargetError("l’erreur d’arrêt n’est pas le défaut de prune Podman connu")
    host = env.get("DOCKER_HOST", "")
    if not re.fullmatch(r"unix:///[^\s]+", host):
        raise TargetError("le fallback Podman exige le même socket Unix explicite")
    prefix = ["podman", "--remote", "--url", host]

    def run(*args: str) -> str:
        result = subprocess.run([*prefix, *args], capture_output=True, text=True, check=False)
        if result.returncode:
            raise TargetError("le nettoyage Podman ciblé a échoué")
        return result.stdout.strip()

    # This is Podman's native remote API; a Docker endpoint cannot satisfy it.
    if not run("info", "--format", "{{.Version.Version}}"):
        raise TargetError("le moteur de ce socket n’est pas confirmé comme Podman")
    identity = project_id(repo)
    label = f"com.supabase.cli.project={identity}"
    if run("ps", "--all", "--filter", f"label={label}", "--format", "{{.ID}}"):
        raise TargetError("des conteneurs CI subsistent ; aucun volume ne sera supprimé")
    names = run("volume", "ls", "--filter", f"label={label}", "--format", "{{.Name}}").splitlines()
    allowed_names = {f"supabase_db_{identity}", f"supabase_storage_{identity}"}
    if len(names) != len(set(names)) or not set(names).issubset(allowed_names):
        raise TargetError("un volume ne correspond pas aux noms CI autorisés")
    # Validate every candidate before deleting any; never use prune or --force.
    for name in names:
        inspected = json.loads(run("volume", "inspect", name))
        if (not isinstance(inspected, list) or len(inspected) != 1
                or not isinstance(inspected[0], dict)
                or inspected[0].get("Name") != name
                or not isinstance(inspected[0].get("Labels"), dict)
                or inspected[0]["Labels"].get("com.supabase.cli.project") != identity):
            raise TargetError("un volume ne porte pas le marqueur CI attendu")
        if run("ps", "--all", "--filter", f"volume={name}", "--format", "{{.ID}}"):
            raise TargetError("un volume CI est encore attaché")
    for name in names:
        run("volume", "rm", name)
    if run("volume", "ls", "--filter", f"label={label}", "--format", "{{.Name}}"):
        raise TargetError("des volumes CI subsistent après nettoyage")


def main() -> int:
    repo = Path(__file__).resolve().parents[1]
    action = sys.argv[1] if len(sys.argv) == 2 else "help"
    try:
        if action == "prepare":
            prepare(repo, dict(os.environ))
        elif action == "guard":
            guard(repo, dict(os.environ))
        elif action == "environment":
            workdir(repo, dict(os.environ))
            validate_env(dict(os.environ))
            if os.environ.get("AI_CENTER_SUPABASE_DB_CONTAINER", f"supabase_db_{project_id(repo)}") != f"supabase_db_{project_id(repo)}":
                raise TargetError("conteneur CI inattendu")
        elif action == "project-id":
            print(project_id(repo))
        elif action == "status":
            guard(repo, dict(os.environ))
            validate_status(json.load(sys.stdin))
        elif action == "podman-cleanup":
            cleanup_podman(repo, dict(os.environ))
        else:
            print("Usage: integration-target.py prepare|guard|environment|project-id|status")
            return 2
    except TargetError as error:
        print(f"Refus : {error}.", file=sys.stderr)
        return 1
    except (OSError, ValueError, KeyError, TypeError):
        # Do not echo parser exceptions: they may contain a connection URL.
        print("Refus : cible CI absente ou non conforme ; lancer integration-stack.sh prepare et vérifier la configuration/les variables CI.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
