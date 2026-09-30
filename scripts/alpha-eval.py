#!/usr/bin/env python3
"""Prepare and aggregate the private Alpha Context Proof evaluation campaign.

This utility deliberately performs no network call. It accepts only pseudonymous
identifiers and numerical metrics; raw prompts and model outputs stay outside the
tracked repository and outside the aggregate report.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import statistics
import sys
import tempfile
from collections import Counter, defaultdict
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "1.0"
LABELS = ("contradiction", "compatible", "ambiguous")
CONDITIONS = ("context_pack", "full_dump")
STAGES = ("calibration", "main", "reserve")
FORBIDDEN_RAW_FIELDS = {
    "content",
    "diff",
    "email",
    "file_content",
    "patch",
    "prompt",
    "repository_url",
    "response",
    "statement",
}
SHA256_RE = re.compile(r"^[a-f0-9]{64}$")


class ValidationError(Exception):
    """Raised when an input cannot be used safely by the harness."""


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise ValidationError(f"{path}: la racine JSON doit être un objet")
    return value


def read_jsonl(path: Path, *, allow_missing: bool = False) -> list[dict[str, Any]]:
    if allow_missing and not path.exists():
        return []
    records: list[dict[str, Any]] = []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise ValidationError(f"{path}: {error}") from error
    for line_number, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValidationError(f"{path}:{line_number}: {error}") from error
        if not isinstance(record, dict):
            raise ValidationError(f"{path}:{line_number}: chaque ligne doit être un objet")
        record["__line__"] = line_number
        records.append(record)
    return records


def write_json_atomic(path: Path, value: dict[str, Any], *, overwrite: bool) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and not overwrite:
        raise ValidationError(f"{path} existe déjà; utilisez --force pour le remplacer")
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, ensure_ascii=False, indent=2, sort_keys=True)
            handle.write("\n")
        os.replace(temporary_name, path)
    except BaseException:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass
        raise


def decimal_value(value: Any, label: str) -> Decimal:
    if isinstance(value, bool):
        raise ValidationError(f"{label}: nombre attendu")
    try:
        result = Decimal(str(value))
    except (InvalidOperation, ValueError) as error:
        raise ValidationError(f"{label}: nombre attendu") from error
    if not result.is_finite() or result < 0:
        raise ValidationError(f"{label}: nombre positif fini attendu")
    return result


def ratio(numerator: float, denominator: float) -> float:
    return numerator / denominator if denominator else 0.0


def rounded(value: float | Decimal) -> float:
    return round(float(value), 6)


def find_forbidden_fields(value: Any, location: str = "$") -> list[str]:
    errors: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            if key in FORBIDDEN_RAW_FIELDS:
                errors.append(f"{location}.{key}: champ de donnée brute interdit")
            errors.extend(find_forbidden_fields(child, f"{location}.{key}"))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            errors.extend(find_forbidden_fields(child, f"{location}[{index}]"))
    return errors


def campaign_template() -> dict[str, Any]:
    projects = [
        {
            "project_ref": f"project-{index:02d}",
            "corpus_role": "ai_center" if index == 1 else "comparison",
            "corpus_fingerprint": f"REPLACE_SHA256_PROJECT_{index:02d}",
            "consent_confirmed": False,
        }
        for index in range(1, 4)
    ]
    handoff_tasks = []
    for index in range(1, 13):
        project_index = ((index - 1) % 3) + 1
        handoff_tasks.append(
            {
                "task_id": f"handoff-{index:02d}",
                "project_ref": f"project-{project_index:02d}",
                "critical_fact_ids": [f"REPLACE_FACT_{index:02d}_01"],
                "relevant_item_ids": [f"REPLACE_RELEVANT_{index:02d}_01"],
                "irrelevant_item_ids": [f"REPLACE_IRRELEVANT_{index:02d}_01"],
            }
        )
    contradiction_pairs = []
    for index in range(1, 61):
        project_index = ((index - 1) % 3) + 1
        expected_label = LABELS[(index - 1) // 20]
        contradiction_pairs.append(
            {
                "pair_id": f"pair-{index:03d}",
                "project_ref": f"project-{project_index:02d}",
                "left_version_ref": f"REPLACE_LEFT_VERSION_{index:03d}",
                "right_version_ref": f"REPLACE_RIGHT_VERSION_{index:03d}",
                "expected_label": expected_label,
            }
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "campaign_id": "alpha-context-proof-001",
        "budget_usd": {
            "calibration": 10,
            "main": 70,
            "reserve": 20,
            "total": 100,
        },
        "privacy": {
            "contains_secrets": False,
            "contains_personal_data": False,
            "contains_sensitive_data": False,
        },
        "projects": projects,
        "handoff_tasks": handoff_tasks,
        "contradiction_pairs": contradiction_pairs,
    }


def validate_manifest(manifest: dict[str, Any], *, allow_placeholders: bool) -> None:
    errors = find_forbidden_fields(manifest)
    allowed_manifest_fields = {
        "schema_version", "campaign_id", "budget_usd", "privacy", "projects",
        "handoff_tasks", "contradiction_pairs",
    }
    unknown_manifest_fields = set(manifest) - allowed_manifest_fields
    if unknown_manifest_fields:
        errors.append(f"champs manifest inconnus: {', '.join(sorted(unknown_manifest_fields))}")
    if manifest.get("schema_version") != SCHEMA_VERSION:
        errors.append("schema_version doit valoir 1.0")

    campaign_id = manifest.get("campaign_id")
    if not isinstance(campaign_id, str) or not re.fullmatch(r"[a-z0-9][a-z0-9-]{2,63}", campaign_id):
        errors.append("campaign_id doit être un slug de 3 à 64 caractères")

    budget = manifest.get("budget_usd")
    expected_budget = {"calibration": 10, "main": 70, "reserve": 20, "total": 100}
    if budget != expected_budget:
        errors.append("budget_usd doit être exactement 10 + 70 + 20 = 100 USD")

    privacy = manifest.get("privacy")
    if not isinstance(privacy, dict) or any(
        privacy.get(key) is not False
        for key in ("contains_secrets", "contains_personal_data", "contains_sensitive_data")
    ):
        errors.append("les trois indicateurs privacy doivent être false")

    projects = manifest.get("projects")
    project_refs: set[str] = set()
    corpus_fingerprints: set[str] = set()
    corpus_roles: Counter[str] = Counter()
    if not isinstance(projects, list) or len(projects) != 3:
        errors.append("projects doit contenir exactement 3 projets")
        projects = []
    for index, project in enumerate(projects):
        if not isinstance(project, dict):
            errors.append(f"projects[{index}] doit être un objet")
            continue
        project_ref = project.get("project_ref")
        unknown_project_fields = set(project) - {
            "project_ref", "corpus_role", "corpus_fingerprint", "consent_confirmed"
        }
        if unknown_project_fields:
            errors.append(f"projects[{index}]: champs inconnus: {', '.join(sorted(unknown_project_fields))}")
        if not isinstance(project_ref, str) or not re.fullmatch(r"project-[0-9]{2}", project_ref):
            errors.append(f"projects[{index}].project_ref invalide")
        elif project_ref in project_refs:
            errors.append(f"project_ref dupliqué: {project_ref}")
        else:
            project_refs.add(project_ref)
        fingerprint = project.get("corpus_fingerprint")
        placeholder_fingerprint = isinstance(fingerprint, str) and fingerprint.startswith("REPLACE_SHA256_")
        if not (isinstance(fingerprint, str) and (SHA256_RE.fullmatch(fingerprint) or placeholder_fingerprint)):
            errors.append(f"projects[{index}].corpus_fingerprint invalide")
        elif fingerprint in corpus_fingerprints:
            errors.append(f"projects[{index}]: corpus_fingerprint dupliqué")
        else:
            corpus_fingerprints.add(fingerprint)
        corpus_role = project.get("corpus_role")
        if corpus_role not in ("ai_center", "comparison"):
            errors.append(f"projects[{index}].corpus_role invalide")
        else:
            corpus_roles[corpus_role] += 1
        if not allow_placeholders:
            if placeholder_fingerprint:
                errors.append(f"projects[{index}]: fingerprint placeholder interdit")
            if project.get("consent_confirmed") is not True:
                errors.append(f"projects[{index}]: consent_confirmed doit être true")
    if corpus_roles["ai_center"] != 1 or corpus_roles["comparison"] != 2:
        errors.append("projects doit contenir un corpus ai_center et deux corpus comparison")

    tasks = manifest.get("handoff_tasks")
    task_ids: set[str] = set()
    referenced_projects: set[str] = set()
    if not isinstance(tasks, list) or len(tasks) < 12:
        errors.append("handoff_tasks doit contenir au moins 12 tâches")
        tasks = []
    for index, task in enumerate(tasks):
        if not isinstance(task, dict):
            errors.append(f"handoff_tasks[{index}] doit être un objet")
            continue
        unknown_task_fields = set(task) - {
            "task_id", "project_ref", "critical_fact_ids", "relevant_item_ids",
            "irrelevant_item_ids",
        }
        if unknown_task_fields:
            errors.append(f"handoff_tasks[{index}]: champs inconnus: {', '.join(sorted(unknown_task_fields))}")
        task_id = task.get("task_id")
        if not isinstance(task_id, str) or not re.fullmatch(r"handoff-[0-9]{2}", task_id):
            errors.append(f"handoff_tasks[{index}].task_id invalide")
        elif task_id in task_ids:
            errors.append(f"task_id dupliqué: {task_id}")
        else:
            task_ids.add(task_id)
        project_ref = task.get("project_ref")
        if project_ref not in project_refs:
            errors.append(f"handoff_tasks[{index}]: project_ref inconnu")
        else:
            referenced_projects.add(project_ref)
        for field in ("critical_fact_ids", "relevant_item_ids", "irrelevant_item_ids"):
            values = task.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(item, str) and item for item in values):
                errors.append(f"handoff_tasks[{index}].{field} doit être une liste non vide")
            elif len(values) != len(set(values)):
                errors.append(
                    f"handoff_tasks[{index}].{field} contient des annotations dupliquées"
                )
        relevant = task.get("relevant_item_ids")
        irrelevant = task.get("irrelevant_item_ids")
        if (
            isinstance(relevant, list)
            and isinstance(irrelevant, list)
            and any(item in irrelevant for item in relevant)
        ):
            errors.append(
                f"handoff_tasks[{index}]: annotations pertinentes et hors sujet doivent être disjointes"
            )

    pairs = manifest.get("contradiction_pairs")
    pair_ids: set[str] = set()
    label_counts: Counter[str] = Counter()
    if not isinstance(pairs, list) or len(pairs) < 60:
        errors.append("contradiction_pairs doit contenir au moins 60 paires")
        pairs = []
    for index, pair in enumerate(pairs):
        if not isinstance(pair, dict):
            errors.append(f"contradiction_pairs[{index}] doit être un objet")
            continue
        unknown_pair_fields = set(pair) - {
            "pair_id", "project_ref", "left_version_ref", "right_version_ref",
            "expected_label",
        }
        if unknown_pair_fields:
            errors.append(f"contradiction_pairs[{index}]: champs inconnus: {', '.join(sorted(unknown_pair_fields))}")
        pair_id = pair.get("pair_id")
        if not isinstance(pair_id, str) or not re.fullmatch(r"pair-[0-9]{3}", pair_id):
            errors.append(f"contradiction_pairs[{index}].pair_id invalide")
        elif pair_id in pair_ids:
            errors.append(f"pair_id dupliqué: {pair_id}")
        else:
            pair_ids.add(pair_id)
        project_ref = pair.get("project_ref")
        if project_ref not in project_refs:
            errors.append(f"contradiction_pairs[{index}]: project_ref inconnu")
        else:
            referenced_projects.add(project_ref)
        expected_label = pair.get("expected_label")
        if expected_label not in LABELS:
            errors.append(f"contradiction_pairs[{index}].expected_label invalide")
        else:
            label_counts[expected_label] += 1
        for field in ("left_version_ref", "right_version_ref"):
            if not isinstance(pair.get(field), str) or not pair[field]:
                errors.append(f"contradiction_pairs[{index}].{field} requis")
        if pair.get("left_version_ref") == pair.get("right_version_ref"):
            errors.append(
                f"contradiction_pairs[{index}]: deux versions distinctes sont requises"
            )

    for label in LABELS:
        if label_counts[label] < 20:
            errors.append(f"au moins 20 paires {label} sont requises")
    if project_refs and referenced_projects != project_refs:
        errors.append("chaque projet doit être représenté par une tâche ou une paire")

    if not allow_placeholders:
        def walk(value: Any) -> bool:
            if isinstance(value, str):
                return value.startswith("REPLACE_")
            if isinstance(value, dict):
                return any(walk(item) for item in value.values())
            if isinstance(value, list):
                return any(walk(item) for item in value)
            return False

        if walk(manifest):
            errors.append("le manifest contient encore des valeurs REPLACE_…")

    if errors:
        raise ValidationError("manifest invalide:\n- " + "\n- ".join(errors))


def validate_runs(
    runs: list[dict[str, Any]], manifest: dict[str, Any]
) -> dict[str, dict[str, Any]]:
    errors: list[str] = []
    tasks = {item["task_id"]: item for item in manifest["handoff_tasks"]}
    pairs = {item["pair_id"]: item for item in manifest["contradiction_pairs"]}
    run_by_id: dict[str, dict[str, Any]] = {}

    required = {
        "schema_version", "run_id", "case_kind", "case_id", "project_ref",
        "stage", "condition", "repetition", "model", "prompt_version",
        "output_schema_version", "input_tokens", "latency_ms", "cost_usd",
        "structured_output_valid", "source_ids_valid",
    }
    allowed = required | {
        "context_pack_hash",
        "critical_facts_expected",
        "critical_facts_present",
        "relevant_items_expected",
        "relevant_items_present",
        "selected_items_total",
        "irrelevant_items_selected",
        "expected_label",
        "predicted_label",
        "output_tokens",
        "attempts",
        "provider_status",
        "provider_response_ref",
        "request_hash",
        "runner_version",
        "input_evidence_hash",
    }
    for record in runs:
        line = record.pop("__line__", "?")
        location = f"runs:{line}"
        errors.extend(f"{location} {error}" for error in find_forbidden_fields(record))
        missing = required - record.keys()
        unknown = set(record) - allowed
        if unknown:
            errors.append(f"{location}: champs inconnus: {', '.join(sorted(unknown))}")
        if missing:
            errors.append(f"{location}: champs absents: {', '.join(sorted(missing))}")
            continue
        if record["schema_version"] != SCHEMA_VERSION:
            errors.append(f"{location}: schema_version invalide")
        run_id = record["run_id"]
        if not isinstance(run_id, str) or not run_id:
            errors.append(f"{location}: run_id invalide")
        elif run_id in run_by_id:
            errors.append(f"{location}: run_id dupliqué {run_id}")
        else:
            run_by_id[run_id] = record
        if record["stage"] not in STAGES:
            errors.append(f"{location}: stage invalide")
        if record["condition"] not in CONDITIONS:
            errors.append(f"{location}: condition invalide")
        if not isinstance(record["repetition"], int) or isinstance(record["repetition"], bool) or not 1 <= record["repetition"] <= 5:
            errors.append(f"{location}: repetition doit être comprise entre 1 et 5")
        elif record["repetition"] not in {
            "calibration": (1,),
            "main": (1, 2, 3),
            "reserve": (4, 5),
        }.get(record["stage"], ()):
            errors.append(f"{location}: répétition incompatible avec le stage")
        for field in ("model", "prompt_version", "output_schema_version"):
            if not isinstance(record[field], str) or not record[field]:
                errors.append(f"{location}: {field} requis")
        for field in ("input_tokens", "latency_ms"):
            if not isinstance(record[field], int) or isinstance(record[field], bool) or record[field] < 0:
                errors.append(f"{location}: {field} doit être un entier positif")
        if record["input_tokens"] == 0:
            errors.append(f"{location}: input_tokens doit être supérieur à zéro")
        output_tokens = record.get("output_tokens")
        if output_tokens is not None and (
            not isinstance(output_tokens, int)
            or isinstance(output_tokens, bool)
            or output_tokens < 0
        ):
            errors.append(f"{location}: output_tokens doit être un entier positif")
        attempts = record.get("attempts")
        if attempts is not None and (
            not isinstance(attempts, int) or isinstance(attempts, bool) or attempts != 1
        ):
            errors.append(f"{location}: le live eval interdit les retries automatiques")
        for field in ("provider_status", "provider_response_ref"):
            value = record.get(field)
            if value is not None and (not isinstance(value, str) or not value):
                errors.append(f"{location}: {field} doit être une référence expurgée")
        request_hash = record.get("request_hash")
        if request_hash is not None and (
            not isinstance(request_hash, str) or not SHA256_RE.fullmatch(request_hash)
        ):
            errors.append(f"{location}: request_hash SHA-256 invalide")
        try:
            decimal_value(record["cost_usd"], f"{location}.cost_usd")
        except ValidationError as error:
            errors.append(str(error))
        for field in ("structured_output_valid", "source_ids_valid"):
            if not isinstance(record[field], bool):
                errors.append(f"{location}: {field} doit être booléen")

        kind = record["case_kind"]
        case_id = record["case_id"]
        if kind == "handoff":
            task = tasks.get(case_id)
            if task is None:
                errors.append(f"{location}: tâche inconnue {case_id}")
                continue
            if record["project_ref"] != task["project_ref"]:
                errors.append(f"{location}: mauvais project_ref")
            for field in (
                "critical_facts_expected", "critical_facts_present",
                "relevant_items_expected", "relevant_items_present",
                "selected_items_total", "irrelevant_items_selected",
            ):
                value = record.get(field)
                if not isinstance(value, int) or isinstance(value, bool) or value < 0:
                    errors.append(f"{location}: {field} entier positif requis")
            expected = record.get("critical_facts_expected")
            present = record.get("critical_facts_present")
            if isinstance(expected, int) and expected != len(task["critical_fact_ids"]):
                errors.append(f"{location}: critical_facts_expected ne correspond pas au manifest")
            if isinstance(expected, int) and isinstance(present, int) and present > expected:
                errors.append(f"{location}: trop de faits critiques présents")
            relevant_expected = record.get("relevant_items_expected")
            relevant_present = record.get("relevant_items_present")
            if (
                isinstance(relevant_expected, int)
                and relevant_expected != len(task["relevant_item_ids"])
            ):
                errors.append(
                    f"{location}: relevant_items_expected ne correspond pas au manifest"
                )
            if (
                isinstance(relevant_expected, int)
                and isinstance(relevant_present, int)
                and relevant_present > relevant_expected
            ):
                errors.append(f"{location}: trop d’éléments pertinents présents")
            selected = record.get("selected_items_total")
            irrelevant = record.get("irrelevant_items_selected")
            if isinstance(selected, int) and isinstance(irrelevant, int) and irrelevant > selected:
                errors.append(f"{location}: irrelevant_items_selected dépasse selected_items_total")
        elif kind == "contradiction":
            pair = pairs.get(case_id)
            if pair is None:
                errors.append(f"{location}: paire inconnue {case_id}")
                continue
            if record["project_ref"] != pair["project_ref"]:
                errors.append(f"{location}: mauvais project_ref")
            if record["condition"] != "context_pack":
                errors.append(f"{location}: une paire utilise seulement context_pack")
            if record.get("expected_label") != pair["expected_label"]:
                errors.append(f"{location}: expected_label ne correspond pas au manifest")
            if record.get("predicted_label") not in LABELS:
                errors.append(f"{location}: predicted_label invalide")
        else:
            errors.append(f"{location}: case_kind invalide")

        pack_hash = record.get("context_pack_hash")
        if record["condition"] == "context_pack":
            if not isinstance(pack_hash, str) or not SHA256_RE.fullmatch(pack_hash):
                errors.append(f"{location}: context_pack_hash SHA-256 requis")
        elif pack_hash is not None:
            errors.append(f"{location}: full_dump doit avoir context_pack_hash=null")

    production_runs = [record for record in runs if record.get("stage") != "calibration"]
    production_keys = Counter(
        (record.get("case_id"), record.get("condition"), record.get("repetition"))
        for record in production_runs
    )
    duplicated_keys = [key for key, count in production_keys.items() if count > 1]
    if duplicated_keys:
        errors.append(
            "runs: un seul enregistrement est permis par cas, condition et répétition"
        )
    frozen_contracts = {
        (record.get("model"), record.get("prompt_version"), record.get("output_schema_version"))
        for record in production_runs
    }
    if len(frozen_contracts) > 1:
        errors.append("runs: modèle, prompt et schéma doivent être gelés après calibration")

    if errors:
        raise ValidationError("runs invalides:\n- " + "\n- ".join(errors))
    return run_by_id


def validate_evaluations(
    evaluations: list[dict[str, Any]],
    run_by_id: dict[str, dict[str, Any]],
) -> None:
    errors: list[str] = []
    evaluation_ids: set[str] = set()
    role_per_comparison: set[tuple[str, str]] = set()
    role_per_case: set[tuple[str, int, str]] = set()
    comparison_pairs: dict[str, tuple[str, str]] = {}
    pair_comparisons: dict[tuple[str, str], str] = {}
    evaluators: dict[str, set[str]] = {"owner": set(), "secondary": set()}
    required = {
        "schema_version", "evaluation_id", "comparison_id", "task_id",
        "repetition", "context_pack_run_id", "full_dump_run_id",
        "evaluator_ref", "evaluator_role", "blind_preference",
        "context_pack_without_major_reformulation",
    }
    allowed = required
    for record in evaluations:
        line = record.pop("__line__", "?")
        location = f"evaluations:{line}"
        errors.extend(f"{location} {error}" for error in find_forbidden_fields(record))
        missing = required - record.keys()
        unknown = set(record) - allowed
        if unknown:
            errors.append(f"{location}: champs inconnus: {', '.join(sorted(unknown))}")
        if missing:
            errors.append(f"{location}: champs absents: {', '.join(sorted(missing))}")
            continue
        if record["schema_version"] != SCHEMA_VERSION:
            errors.append(f"{location}: schema_version invalide")
        identifiers = (
            "evaluation_id",
            "comparison_id",
            "task_id",
            "context_pack_run_id",
            "full_dump_run_id",
            "evaluator_ref",
        )
        if any(
            not isinstance(record[key], str) or len(record[key].strip()) < 3
            for key in identifiers
        ):
            errors.append(f"{location}: identifiant ou référence évaluateur invalide")
            continue
        if type(record["repetition"]) is not int or not 1 <= record["repetition"] <= 5:
            errors.append(f"{location}: répétition invalide")
            continue
        evaluation_id = record["evaluation_id"]
        if not isinstance(evaluation_id, str) or not evaluation_id:
            errors.append(f"{location}: evaluation_id invalide")
        elif evaluation_id in evaluation_ids:
            errors.append(f"{location}: evaluation_id dupliqué")
        else:
            evaluation_ids.add(evaluation_id)
        role = record["evaluator_role"]
        if role not in ("owner", "secondary"):
            errors.append(f"{location}: evaluator_role invalide")
            continue
        evaluators[role].add(record["evaluator_ref"].strip().casefold())
        pair = (record["context_pack_run_id"], record["full_dump_run_id"])
        comparison_id = record["comparison_id"]
        if comparison_pairs.setdefault(comparison_id, pair) != pair:
            errors.append(f"{location}: comparison_id associé à des runs différents")
        if pair_comparisons.setdefault(pair, comparison_id) != comparison_id:
            errors.append(f"{location}: runs associés à plusieurs comparison_id")
        key = (record["comparison_id"], role)
        if key in role_per_comparison:
            errors.append(f"{location}: rôle dupliqué pour cette comparaison")
        role_per_comparison.add(key)
        case_key = (record["task_id"], record["repetition"], role)
        if case_key in role_per_case:
            errors.append(f"{location}: rôle dupliqué pour cette tâche et répétition")
        role_per_case.add(case_key)
        if record["blind_preference"] not in ("context_pack", "full_dump", "tie"):
            errors.append(f"{location}: blind_preference invalide")
        if not isinstance(record["context_pack_without_major_reformulation"], bool):
            errors.append(f"{location}: résultat de reformulation booléen requis")

        pack_run = run_by_id.get(record["context_pack_run_id"])
        dump_run = run_by_id.get(record["full_dump_run_id"])
        if pack_run is None or dump_run is None:
            errors.append(f"{location}: run de comparaison inconnu")
            continue
        if pack_run.get("condition") != "context_pack" or dump_run.get("condition") != "full_dump":
            errors.append(f"{location}: conditions des runs inversées")
        for run in (pack_run, dump_run):
            if run.get("stage") not in ("main", "reserve"):
                errors.append(
                    f"{location}: une évaluation finale ne peut noter la calibration"
                )
            if run.get("case_kind") != "handoff":
                errors.append(f"{location}: seuls les handoffs sont comparés")
            if run.get("case_id") != record["task_id"] or run.get("repetition") != record["repetition"]:
                errors.append(f"{location}: tâche ou répétition incohérente")
        for field in ("stage", "model", "prompt_version", "output_schema_version"):
            if pack_run.get(field) != dump_run.get(field):
                errors.append(
                    f"{location}: contrat des conditions incohérent ({field})"
                )

    if evaluators["owner"] & evaluators["secondary"]:
        errors.append(
            "évaluations: le second évaluateur doit être une personne distincte du propriétaire"
        )

    if errors:
        raise ValidationError("évaluations invalides:\n- " + "\n- ".join(errors))


def metric(
    value: float,
    threshold: float,
    comparator: str,
    numerator: float,
    denominator: float,
) -> dict[str, Any]:
    if comparator == ">=":
        passed = denominator > 0 and value >= threshold
    elif comparator == "<=":
        passed = denominator > 0 and value <= threshold
    elif comparator == "=":
        passed = denominator > 0 and abs(value - threshold) < 1e-9
    else:
        raise AssertionError(f"comparateur inconnu: {comparator}")
    return {
        "value": rounded(value),
        "threshold": threshold,
        "comparator": comparator,
        "numerator": rounded(numerator),
        "denominator": rounded(denominator),
        "passed": passed,
    }


def build_report(
    manifest: dict[str, Any],
    runs: list[dict[str, Any]],
    evaluations: list[dict[str, Any]],
    reserve_case_ids: set[str] | None = None,
) -> dict[str, Any]:
    reserve_case_ids = set(reserve_case_ids or ()) | {
        record["case_id"] for record in runs if record["stage"] == "reserve"
    }
    production_runs = [record for record in runs if record["stage"] != "calibration"]
    pack_runs = [record for record in production_runs if record["condition"] == "context_pack"]
    handoff_pack = [record for record in pack_runs if record["case_kind"] == "handoff"]
    contradiction_runs = [record for record in pack_runs if record["case_kind"] == "contradiction"]
    owner_evaluations = [record for record in evaluations if record["evaluator_role"] == "owner"]
    secondary_comparisons = {
        record["comparison_id"] for record in evaluations if record["evaluator_role"] == "secondary"
    }
    owner_comparisons = {record["comparison_id"] for record in owner_evaluations}

    source_numerator = sum(1 for record in pack_runs if record["source_ids_valid"])
    source_denominator = len(pack_runs)
    facts_expected = sum(record["critical_facts_expected"] for record in handoff_pack)
    facts_present = sum(record["critical_facts_present"] for record in handoff_pack)
    relevant_expected = sum(record["relevant_items_expected"] for record in handoff_pack)
    relevant_present = sum(record["relevant_items_present"] for record in handoff_pack)
    selected_total = sum(record["selected_items_total"] for record in handoff_pack)
    irrelevant_selected = sum(record["irrelevant_items_selected"] for record in handoff_pack)

    run_index = {
        (record["case_id"], record["repetition"], record["condition"]): record
        for record in production_runs
        if record["case_kind"] == "handoff"
    }
    token_reductions: list[float] = []
    for task in manifest["handoff_tasks"]:
        for repetition in range(1, 6):
            pack = run_index.get((task["task_id"], repetition, "context_pack"))
            dump = run_index.get((task["task_id"], repetition, "full_dump"))
            if pack and dump and dump["input_tokens"] > 0:
                token_reductions.append(1 - (pack["input_tokens"] / dump["input_tokens"]))
    median_reduction = statistics.median(token_reductions) if token_reductions else 0.0

    no_reformulation = sum(
        1 for record in owner_evaluations if record["context_pack_without_major_reformulation"]
    )
    preferred_pack = sum(
        1 for record in owner_evaluations if record["blind_preference"] == "context_pack"
    )
    true_positive = sum(
        1
        for record in contradiction_runs
        if record["expected_label"] == "contradiction" and record["predicted_label"] == "contradiction"
    )
    false_positive = sum(
        1
        for record in contradiction_runs
        if record["expected_label"] != "contradiction" and record["predicted_label"] == "contradiction"
    )
    false_negative = sum(
        1
        for record in contradiction_runs
        if record["expected_label"] == "contradiction" and record["predicted_label"] != "contradiction"
    )
    valid_structured = sum(1 for record in production_runs if record["structured_output_valid"])

    metrics = {
        "source_validity": metric(
            ratio(source_numerator, source_denominator), 1.0, "=", source_numerator, source_denominator
        ),
        "critical_fact_recall": metric(
            ratio(facts_present, facts_expected), 1.0, "=", facts_present, facts_expected
        ),
        "relevant_context_recall": metric(
            ratio(relevant_present, relevant_expected),
            0.85,
            ">=",
            relevant_present,
            relevant_expected,
        ),
        "irrelevant_item_ratio": metric(
            ratio(irrelevant_selected, selected_total), 0.20, "<=", irrelevant_selected, selected_total
        ),
        "median_token_reduction": metric(
            median_reduction, 0.30, ">=", median_reduction, 1 if token_reductions else 0
        ),
        "handoff_without_major_reformulation": metric(
            ratio(no_reformulation, len(owner_evaluations)),
            0.80,
            ">=",
            no_reformulation,
            len(owner_evaluations),
        ),
        "blind_context_pack_preference": metric(
            ratio(preferred_pack, len(owner_evaluations)),
            0.70,
            ">=",
            preferred_pack,
            len(owner_evaluations),
        ),
        "contradiction_precision": metric(
            ratio(true_positive, true_positive + false_positive),
            0.80,
            ">=",
            true_positive,
            true_positive + false_positive,
        ),
        "contradiction_recall": metric(
            ratio(true_positive, true_positive + false_negative),
            0.70,
            ">=",
            true_positive,
            true_positive + false_negative,
        ),
        "structured_output_validity": metric(
            ratio(valid_structured, len(production_runs)),
            0.95,
            ">=",
            valid_structured,
            len(production_runs),
        ),
    }

    stage_costs: dict[str, Decimal] = {stage: Decimal("0") for stage in STAGES}
    for record in runs:
        stage_costs[record["stage"]] += decimal_value(record["cost_usd"], "cost_usd")
    total_cost = sum(stage_costs.values(), Decimal("0"))
    budget_limits = manifest["budget_usd"]
    budget_passed = all(
        stage_costs[stage] <= Decimal(str(budget_limits[stage])) for stage in STAGES
    ) and total_cost <= Decimal(str(budget_limits["total"]))

    missing: list[str] = []
    production_index: Counter[tuple[str, str, int]] = Counter(
        (record["case_id"], record["condition"], record["repetition"])
        for record in production_runs
    )
    for task in manifest["handoff_tasks"]:
        expected_repetitions = set(
            range(1, 6 if task["task_id"] in reserve_case_ids else 4)
        )
        for condition in CONDITIONS:
            repetitions = {
                repetition
                for repetition in range(1, 6)
                if production_index[(task["task_id"], condition, repetition)]
            }
            if repetitions != expected_repetitions:
                missing.append(
                    f"{task['task_id']}:{condition}:{len(expected_repetitions)} répétitions"
                )
    for pair in manifest["contradiction_pairs"]:
        expected_repetitions = set(
            range(1, 6 if pair["pair_id"] in reserve_case_ids else 4)
        )
        repetitions = {
            repetition
            for repetition in range(1, 6)
            if production_index[(pair["pair_id"], "context_pack", repetition)]
        }
        if repetitions != expected_repetitions:
            missing.append(f"{pair['pair_id']}:{len(expected_repetitions)} répétitions")
    expected_comparisons = sum(
        5 if task["task_id"] in reserve_case_ids else 3
        for task in manifest["handoff_tasks"]
    )
    owner_case_keys = {
        (record["task_id"], record["repetition"]) for record in owner_evaluations
    }
    for task in manifest["handoff_tasks"]:
        for repetition in range(1, 6 if task["task_id"] in reserve_case_ids else 4):
            if (task["task_id"], repetition) not in owner_case_keys:
                missing.append(f"évaluation owner:{task['task_id']}:répétition {repetition}")
    if len(owner_comparisons) != expected_comparisons:
        missing.append(f"comparaisons owner: {len(owner_comparisons)}/{expected_comparisons}")
    secondary_coverage = ratio(len(secondary_comparisons & owner_comparisons), len(owner_comparisons))
    if secondary_coverage < 0.25:
        missing.append("couverture second évaluateur < 25 %")

    sample_complete = not missing
    gate_passed = (
        sample_complete
        and budget_passed
        and all(item["passed"] for item in metrics.values())
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "campaign_id": manifest["campaign_id"],
        "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "sample": {
            "projects": len(manifest["projects"]),
            "handoff_tasks": len(manifest["handoff_tasks"]),
            "contradiction_pairs": len(manifest["contradiction_pairs"]),
            "model_runs": len(runs),
            "owner_comparisons": len(owner_comparisons),
            "secondary_coverage": rounded(secondary_coverage),
            "complete": sample_complete,
            "missing": missing,
        },
        "budget": {
            "calibration_usd": rounded(stage_costs["calibration"]),
            "main_usd": rounded(stage_costs["main"]),
            "reserve_usd": rounded(stage_costs["reserve"]),
            "total_usd": rounded(total_cost),
            "passed": budget_passed,
        },
        "metrics": metrics,
        "gate_passed": gate_passed,
    }


def command_init(args: argparse.Namespace) -> int:
    write_json_atomic(args.output, campaign_template(), overwrite=args.force)
    print(f"Manifest initialisé: {args.output}")
    return 0


def command_validate(args: argparse.Namespace) -> int:
    manifest = read_json(args.manifest)
    validate_manifest(manifest, allow_placeholders=args.allow_placeholders)
    print(
        json.dumps(
            {
                "valid": True,
                "projects": len(manifest["projects"]),
                "handoff_tasks": len(manifest["handoff_tasks"]),
                "contradiction_pairs": len(manifest["contradiction_pairs"]),
                "placeholders_allowed": args.allow_placeholders,
            },
            separators=(",", ":"),
        )
    )
    return 0


def command_budget(args: argparse.Namespace) -> int:
    manifest = read_json(args.manifest)
    validate_manifest(manifest, allow_placeholders=True)
    runs = read_jsonl(args.runs, allow_missing=True)
    stage_costs: dict[str, Decimal] = {stage: Decimal("0") for stage in STAGES}
    for record in runs:
        record.pop("__line__", None)
        stage = record.get("stage")
        if stage not in STAGES:
            raise ValidationError(f"run avec stage invalide: {stage}")
        stage_costs[stage] += decimal_value(record.get("cost_usd"), "cost_usd")
    next_cost = decimal_value(args.next_max_cost_usd, "next-max-cost-usd")
    total = sum(stage_costs.values(), Decimal("0"))
    stage_limit = Decimal(str(manifest["budget_usd"][args.stage]))
    total_limit = Decimal(str(manifest["budget_usd"]["total"]))
    allowed = stage_costs[args.stage] + next_cost <= stage_limit and total + next_cost <= total_limit
    print(
        json.dumps(
            {
                "allowed": allowed,
                "stage": args.stage,
                "stage_spent_usd": rounded(stage_costs[args.stage]),
                "stage_limit_usd": rounded(stage_limit),
                "total_spent_usd": rounded(total),
                "total_limit_usd": rounded(total_limit),
                "next_max_cost_usd": rounded(next_cost),
            },
            separators=(",", ":"),
        )
    )
    return 0 if allowed else 3


def command_summarize(args: argparse.Namespace) -> int:
    manifest = read_json(args.manifest)
    validate_manifest(manifest, allow_placeholders=False)
    runs = read_jsonl(args.runs)
    evaluations = read_jsonl(args.evaluations)
    run_by_id = validate_runs(runs, manifest)
    validate_evaluations(evaluations, run_by_id)
    spec = importlib.util.spec_from_file_location(
        "alpha_live_eval", Path(__file__).with_name("alpha-live-eval.py")
    )
    assert spec is not None and spec.loader is not None
    protocol = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(protocol)
    config = read_json(args.config)
    try:
        cases = read_json(protocol.ensure_private_path(args.cases))
        protocol.validate_private_cases(cases, manifest)
        protocol.validate_frozen_evidence(config, manifest, runs, cases)
        protocol.validate_reserve_evidence(config, runs)
    except protocol.LiveEvalError as error:
        raise ValidationError(str(error)) from error
    for evaluation in evaluations:
        run = run_by_id[evaluation["context_pack_run_id"]]
        expected_comparison = f"cmp-{run['case_id']}-{run['repetition']}-{protocol.opaque_ref(run['model'])}"
        if evaluation["comparison_id"] != expected_comparison:
            raise ValidationError(
                "comparison_id ne correspond pas au comparatif aveugle planifié"
            )
    report = build_report(manifest, runs, evaluations, set(config["reserve_case_ids"]))
    write_json_atomic(args.output, report, overwrite=args.force)
    print(
        json.dumps(
            {
                "report": str(args.output),
                "gate_passed": report["gate_passed"],
                "total_cost_usd": report["budget"]["total_usd"],
            },
            separators=(",", ":"),
        )
    )
    return 0 if report["gate_passed"] else 4


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Harness local et sans réseau pour la campagne Alpha Context Proof."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    init_parser = subparsers.add_parser("init", help="Créer un manifest privé prérempli.")
    init_parser.add_argument("--output", type=Path, required=True)
    init_parser.add_argument("--force", action="store_true")
    init_parser.set_defaults(handler=command_init)

    validate_parser = subparsers.add_parser("validate", help="Valider le manifest de campagne.")
    validate_parser.add_argument("--manifest", type=Path, required=True)
    validate_parser.add_argument("--allow-placeholders", action="store_true")
    validate_parser.set_defaults(handler=command_validate)

    budget_parser = subparsers.add_parser("budget", help="Autoriser ou bloquer le prochain appel.")
    budget_parser.add_argument("--manifest", type=Path, required=True)
    budget_parser.add_argument("--runs", type=Path, required=True)
    budget_parser.add_argument("--stage", choices=STAGES, required=True)
    budget_parser.add_argument("--next-max-cost-usd", required=True)
    budget_parser.set_defaults(handler=command_budget)

    summary_parser = subparsers.add_parser("summarize", help="Produire le rapport agrégé.")
    summary_parser.add_argument("--manifest", type=Path, required=True)
    summary_parser.add_argument("--config", type=Path, required=True)
    summary_parser.add_argument("--cases", type=Path, required=True)
    summary_parser.add_argument("--runs", type=Path, required=True)
    summary_parser.add_argument("--evaluations", type=Path, required=True)
    summary_parser.add_argument("--output", type=Path, required=True)
    summary_parser.add_argument("--force", action="store_true")
    summary_parser.set_defaults(handler=command_summarize)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        return int(args.handler(args))
    except ValidationError as error:
        print(f"Erreur de validation: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
