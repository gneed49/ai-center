#!/usr/bin/env python3
"""Prepare and validate the disposable Supabase target without reading .env files."""

from __future__ import annotations

import hashlib
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib


POSTGRES_VERSION = "17.11.0.002"
POSTGRES_SERVER_VERSION = "170011"
POSTGRES_INDEX_DIGEST = "sha256:0450166354dc9c1d25f0322ac8b580774d4fb0184d2b087f6e4fe9499c66cf53"
POSTGRES_PLATFORM_DIGESTS = {
    "amd64": "sha256:4bfbe2e6d7909bd386b1b774683899deb12efa04a3eebaa141f164ffd04ada1d",
    "arm64": "sha256:31fa7517ea6b1891cb4822b61dfd8cc390cc46c33deb9f1019fbe159ef7f0689",
}
POSTGRES_REPOSITORIES = ("supabase/postgres", "docker.io/supabase/postgres",
                         "registry-1.docker.io/supabase/postgres", "public.ecr.aws/supabase/postgres")


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
    if (config["db"]["major_version"] != 17
            or config.get("experimental", {}).get("orioledb_version")):
        raise TargetError("la stack CI exige PostgreSQL 17 standard")
    return text


def validate_env(env: dict[str, str]) -> None:
    for name in ("SUPABASE_CLI_BINARY_OVERRIDE", "SUPABASE_INTERNAL_IMAGE_REGISTRY",
                 "SUPABASE_EXPERIMENTAL_ORIOLEDB_VERSION", "SUPABASE_EXPERIMENTAL_STACK",
                 "SUPABASE_USE_SLIM_IMAGES"):
        if env.get(name):
            raise TargetError(f"{name} ne peut pas remplacer la cible CI vérifiée")
    if env.get("SUPABASE_DB_MAJOR_VERSION", "17") != "17":
        raise TargetError("SUPABASE_DB_MAJOR_VERSION doit conserver le major 17")
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


def marker(repo: Path, version: int = 2) -> dict:
    value = {"version": version, "repo": str(repo.resolve()), "project_id": project_id(repo)}
    if version == 2:
        value.update(postgres_version=POSTGRES_VERSION, postgres_index_digest=POSTGRES_INDEX_DIGEST)
    return value


def guard(repo: Path, env: dict[str, str], *, allow_legacy: bool = False) -> Path:
    target = workdir(repo, env)
    validate_env(env)
    pin = target / "supabase/.temp/postgres-version"
    for path in (target / "target.json", target / "supabase/config.toml", pin):
        if path.is_symlink() or path.resolve() != path:
            raise TargetError("la configuration CI ne doit contenir aucun lien symbolique")
    recorded = json.loads((target / "target.json").read_text())
    legacy = allow_legacy and recorded == marker(repo, 1)
    if recorded != marker(repo) and not legacy:
        raise TargetError("le marqueur d’appartenance CI est invalide")
    if (target / "supabase/config.toml").read_text() != config_text(repo):
        raise TargetError("la configuration CI a changé ; reset/stop refusé")
    if env.get("AI_CENTER_SUPABASE_DB_CONTAINER", f"supabase_db_{project_id(repo)}") != f"supabase_db_{project_id(repo)}":
        raise TargetError("le conteneur PostgreSQL n’appartient pas à cette stack CI")
    # CLI-generated pins may omit the final newline; no other whitespace or
    # extra Docker reference is accepted.
    expected_pins = {POSTGRES_VERSION, POSTGRES_VERSION + "\n"}
    if legacy:
        expected_pins.update(("17.6.1.158", "17.6.1.158\n"))
    if pin.exists():
        if pin.read_text() not in expected_pins:
            raise TargetError("le pin PostgreSQL CI a changé")
    elif not legacy:
        raise TargetError("le pin PostgreSQL CI manque")
    for name in ("project-ref", "pooler-url", "linked-project.json"):
        if (pin.parent / name).exists() or (pin.parent / name).is_symlink():
            raise TargetError("la stack CI ne peut pas contenir de lien vers un projet distant")
    return target


def prepare(repo: Path, env: dict[str, str]) -> Path:
    validate_env(env)
    target = workdir(repo, env)
    text = config_text(repo)
    if target.exists():
        # Only an intact former marker may be upgraded. A missing pin on a new
        # marker is tampering, not permission to silently repair the target.
        guard(repo, env, allow_legacy=True)
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
    (destination / ".temp").mkdir(exist_ok=True, mode=0o700)
    (destination / ".temp/postgres-version").write_text(POSTGRES_VERSION + "\n")
    (target / "target.json").write_text(json.dumps(marker(repo)) + "\n")
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


def verify_postgres(repo: Path, env: dict[str, str]) -> dict:
    """Attest the running image without exposing container environment or URLs."""
    directory = guard(repo, env)
    proof_path = directory / "postgres-image.json"
    if proof_path.is_symlink() or proof_path.resolve() != proof_path:
        raise TargetError("le reçu de version PostgreSQL CI est un lien symbolique")
    # A failed recheck must not leave an earlier success looking current.
    proof_path.unlink(missing_ok=True)
    if shutil.which("docker", path=env.get("PATH")):
        runtime = ["docker"]
    else:
        host = env.get("DOCKER_HOST", "")
        if not re.fullmatch(r"unix:///[^\s]+", host):
            raise TargetError("la vérification Podman exige le même socket Unix explicite")
        runtime = ["podman", "--remote", "--url", host]
    container = "supabase_db_" + project_id(repo)

    def run(*args: str) -> str:
        try:
            result = subprocess.run([*runtime, *args], capture_output=True, text=True,
                                    timeout=20, env=env, check=False)
        except (OSError, subprocess.TimeoutExpired):
            raise TargetError("la vérification PostgreSQL CI est indisponible") from None
        if result.returncode:
            raise TargetError("la vérification PostgreSQL CI a échoué")
        return result.stdout.strip()

    # Do not request Env, mounts, port bindings or the complete inspect document.
    container_format = ('{"name":{{json .Name}},"image_id":{{json .Image}},'
                        '"image":{{json .Config.Image}},"labels":{{json .Config.Labels}},'
                        '"running":{{json .State.Running}}}')
    detail = json.loads(run("container", "inspect", "--format", container_format, container))
    if (not isinstance(detail, dict) or not isinstance(detail.get("name"), str)
            or detail["name"].lstrip("/") != container
            or detail.get("running") is not True
            or not isinstance(detail.get("labels"), dict)
            or detail["labels"].get("com.supabase.cli.project") != project_id(repo)):
        raise TargetError("identité du conteneur PostgreSQL CI non confirmée")
    image = detail.get("image")
    if not isinstance(image, str) or image not in {f"{name}:{POSTGRES_VERSION}" for name in POSTGRES_REPOSITORIES}:
        raise TargetError("le conteneur CI utilise une image PostgreSQL inattendue")
    image_id = detail.get("image_id", "")
    if not isinstance(image_id, str) or not re.fullmatch(r"(?:sha256:)?[a-f0-9]{64}", image_id):
        raise TargetError("identité de l’image PostgreSQL CI invalide")
    image_format = ('{"id":{{json .Id}},"architecture":{{json .Architecture}},'
                    '"os":{{json .Os}},"digests":{{json .RepoDigests}}}')
    metadata = json.loads(run("image", "inspect", "--format", image_format, image_id))
    if (not isinstance(metadata, dict) or not isinstance(metadata.get("id"), str)
            or metadata["id"].removeprefix("sha256:") != image_id.removeprefix("sha256:")
            or metadata.get("os") != "linux"
            or not isinstance(metadata.get("architecture"), str)
            or metadata.get("architecture") not in POSTGRES_PLATFORM_DIGESTS
            or not isinstance(metadata.get("digests"), list)):
        raise TargetError("métadonnées de l’image PostgreSQL CI non conformes")
    architecture = metadata["architecture"]
    allowed = {f"{name}@{digest}" for name in POSTGRES_REPOSITORIES
               for digest in (POSTGRES_INDEX_DIGEST, POSTGRES_PLATFORM_DIGESTS[architecture])}
    attested = sorted(value for value in metadata["digests"] if isinstance(value, str) and value in allowed)
    if not attested:
        raise TargetError("digest officiel de l’image PostgreSQL CI non confirmé")
    server_version = run("exec", container, "psql", "-X", "--no-password", "-U", "postgres",
                         "-d", "postgres", "-Atq", "-c", "show server_version_num")
    if server_version != POSTGRES_SERVER_VERSION:
        raise TargetError("version effective du serveur PostgreSQL CI inattendue")
    proof = {"checked_at": datetime.now(timezone.utc).isoformat(),
             "project_id": project_id(repo), "image": image, "image_id": image_id,
             "architecture": architecture, "repo_digest": attested[0],
             "index_digest": POSTGRES_INDEX_DIGEST, "server_version_num": server_version}
    proof_path.write_text(json.dumps(proof, indent=2) + "\n")
    proof_path.chmod(0o600)
    return proof


def cleanup_podman(repo: Path, env: dict[str, str]) -> None:
    """Recover only the known Podman prune incompatibility, on the same socket."""
    directory = guard(repo, env, allow_legacy=True)
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
        elif action == "guard-cleanup":
            guard(repo, dict(os.environ), allow_legacy=True)
        elif action == "verify-postgres":
            proof = verify_postgres(repo, dict(os.environ))
            print(f"PostgreSQL CI vérifié : {proof['image']} ; {proof['architecture']} ; "
                  f"{proof['repo_digest']} ; version serveur {proof['server_version_num']}.")
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
            print("Usage: integration-target.py prepare|guard|guard-cleanup|environment|project-id|status|verify-postgres|podman-cleanup")
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
