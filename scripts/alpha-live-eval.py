#!/usr/bin/env python3
"""Run the private Alpha Context Proof live evaluation safely.

The default execution path is a dry run. A billable request requires both
``--execute`` and an exact confirmation token. This module never reads dotenv
files, never prints provider payloads, and never retries an ambiguous request.
All private inputs and outputs are constrained to ``.run/alpha-context-proof``.
"""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import random
import re
import secrets
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from collections import defaultdict
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation, ROUND_CEILING
from pathlib import Path
from typing import Any, BinaryIO


RUNNER_VERSION = "1.2.0"
FORMAT_VERSION = "1.0"
PROMPT_VERSION = "alpha-context-proof-live-v2"
OUTPUT_SCHEMA_VERSION = "alpha-context-proof-live-v2"
RESPONSES_API_URL = "https://api.openai.com/v1/responses"
API_KEY_ENV = "OPENAI_API_KEY"
LIVE_CONFIRMATION = "RUN_OPENAI_ALPHA_CONTEXT_PROOF"
STAGES = ("calibration", "main", "reserve")
CONDITIONS = ("context_pack", "full_dump")
STAGE_LIMITS = {
    "calibration": Decimal("10"),
    "main": Decimal("70"),
    "reserve": Decimal("20"),
}
TOTAL_LIMIT = Decimal("100")
MONEY_QUANTUM = Decimal("0.000001")
SAFE_EVENT_FIELDS = {
    "event",
    "run_id",
    "job_id",
    "stage",
    "case_kind",
    "case_id",
    "model_ref",
    "status",
    "http_status",
    "request_hash",
    "provider_response_ref",
    "input_tokens",
    "output_tokens",
    "cost_usd",
    "reserved_usd",
    "reason_code",
}
SECRET_PATTERNS = (
    re.compile(r"\bsk-[A-Za-z0-9_-]{12,}\b"),
    re.compile(r"\b(?:ghp|github_pat)_[A-Za-z0-9_]{12,}\b"),
    re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
    re.compile(r"\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b", re.IGNORECASE),
)
SENSITIVE_KEYS = {
    "access_token",
    "api_key",
    "authorization",
    "email",
    "password",
    "private_key",
    "refresh_token",
    "secret",
}


class LiveEvalError(Exception):
    """Expected, sanitized runner failure."""


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def sha256_json(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def opaque_ref(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()[:16]


def decimal_value(value: Any, label: str) -> Decimal:
    if isinstance(value, bool):
        raise LiveEvalError(f"{label}: finite non-negative number required")
    try:
        parsed = Decimal(str(value))
    except (InvalidOperation, ValueError) as error:
        raise LiveEvalError(f"{label}: finite non-negative number required") from error
    if not parsed.is_finite() or parsed < 0:
        raise LiveEvalError(f"{label}: finite non-negative number required")
    return parsed


def money(value: Decimal) -> Decimal:
    return value.quantize(MONEY_QUANTUM, rounding=ROUND_CEILING)


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def default_private_root() -> Path:
    return repo_root() / ".run" / "alpha-context-proof"


def ensure_private_path(path: Path, private_root: Path | None = None) -> Path:
    root = (private_root or default_private_root()).resolve()
    resolved = path.resolve()
    try:
        resolved.relative_to(root)
    except ValueError as error:
        raise LiveEvalError(
            "live evaluation files must stay under .run/alpha-context-proof"
        ) from error
    return resolved


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise LiveEvalError(f"cannot read JSON file {path.name}") from error
    if not isinstance(value, dict):
        raise LiveEvalError(f"{path.name}: JSON root must be an object")
    return value


def read_jsonl(path: Path, *, allow_missing: bool = True) -> list[dict[str, Any]]:
    if allow_missing and not path.exists():
        return []
    records: list[dict[str, Any]] = []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise LiveEvalError(f"cannot read JSONL file {path.name}") from error
    for line_number, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise LiveEvalError(
                f"{path.name}:{line_number}: invalid JSONL record"
            ) from error
        if not isinstance(value, dict):
            raise LiveEvalError(f"{path.name}:{line_number}: object required")
        records.append(value)
    return records


def write_json_private(path: Path, value: dict[str, Any], *, overwrite: bool) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and not overwrite:
        raise LiveEvalError(f"{path.name} already exists; use --force")
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        os.fchmod(descriptor, 0o600)
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, ensure_ascii=False, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_name, path)
        path.chmod(0o600)
    except BaseException:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass
        raise


def append_jsonl_locked(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_APPEND | os.O_CREAT | os.O_WRONLY, 0o600)
    try:
        with os.fdopen(descriptor, "a", encoding="utf-8") as handle:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            handle.write(canonical_json(record).decode("utf-8") + "\n")
            handle.flush()
            os.fsync(handle.fileno())
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
    except BaseException:
        raise


def safe_event(**fields: Any) -> dict[str, Any]:
    return {key: fields[key] for key in SAFE_EVENT_FIELDS if key in fields}


def emit_event(events_path: Path | None = None, **fields: Any) -> None:
    event = safe_event(**fields)
    if events_path is not None:
        append_jsonl_locked(events_path, {"at": utc_now(), **event})
    print(json.dumps(event, separators=(",", ":"), sort_keys=True))


def find_private_data_risks(value: Any, location: str = "$") -> list[str]:
    risks: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            if key.lower() in SENSITIVE_KEYS:
                risks.append(f"{location}.{key}: sensitive field name")
            risks.extend(find_private_data_risks(child, f"{location}.{key}"))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            risks.extend(find_private_data_risks(child, f"{location}[{index}]"))
    elif isinstance(value, str):
        if any(pattern.search(value) for pattern in SECRET_PATTERNS):
            risks.append(f"{location}: possible secret or personal data")
    return risks


def has_placeholders(value: Any) -> bool:
    if isinstance(value, dict):
        return any(has_placeholders(child) for child in value.values())
    if isinstance(value, list):
        return any(has_placeholders(child) for child in value)
    return isinstance(value, str) and value.startswith("REPLACE_")


def aggregate_validator() -> Any:
    spec = importlib.util.spec_from_file_location(
        "alpha_eval", Path(__file__).with_name("alpha-eval.py")
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_campaign(campaign: dict[str, Any], *, allow_placeholders: bool = False) -> None:
    validator = aggregate_validator()
    try:
        validator.validate_manifest(campaign, allow_placeholders=allow_placeholders)
    except validator.ValidationError as error:
        raise LiveEvalError(str(error)) from error


def model_index(config: dict[str, Any]) -> dict[str, dict[str, Any]]:
    models = config.get("models")
    if not isinstance(models, list) or not 1 <= len(models) <= 2:
        raise LiveEvalError("live config must define one or two calibration models")
    indexed: dict[str, dict[str, Any]] = {}
    for index, model in enumerate(models):
        if not isinstance(model, dict):
            raise LiveEvalError(f"models[{index}] must be an object")
        allowed = {"id", "input_usd_per_million", "output_usd_per_million"}
        if set(model) != allowed:
            raise LiveEvalError(f"models[{index}] has an invalid contract")
        model_id = model.get("id")
        if not isinstance(model_id, str) or not model_id or model_id.startswith("REPLACE_"):
            raise LiveEvalError(f"models[{index}].id must be frozen")
        if model_id in indexed:
            raise LiveEvalError("model IDs must be unique")
        input_price = decimal_value(model.get("input_usd_per_million"), "input price")
        output_price = decimal_value(model.get("output_usd_per_million"), "output price")
        if input_price <= 0 or output_price <= 0:
            raise LiveEvalError("model prices must be strictly positive")
        indexed[model_id] = model
    return indexed


def validate_live_config(
    config: dict[str, Any], campaign: dict[str, Any], *, allow_placeholders: bool = False
) -> None:
    required = {
        "schema_version",
        "campaign_id",
        "pricing_version",
        "pricing_verified_at",
        "models",
        "selected_model",
        "calibration_case_ids",
        "reserve_case_ids",
        "instability_rules",
        "reserve_justifications",
        "max_output_tokens",
        "request_overhead_tokens",
        "timeout_seconds",
        "frozen_contract",
    }
    if set(config) != required:
        raise LiveEvalError("live config fields do not match the versioned contract")
    if config.get("schema_version") != FORMAT_VERSION:
        raise LiveEvalError("live config schema_version must be 1.0")
    if config.get("campaign_id") != campaign.get("campaign_id"):
        raise LiveEvalError("live config campaign_id mismatch")
    for field in ("pricing_version", "pricing_verified_at"):
        value = config.get(field)
        if not isinstance(value, str) or not value or (
            not allow_placeholders and value.startswith("REPLACE_")
        ):
            raise LiveEvalError(f"{field} must be frozen")
    if allow_placeholders and has_placeholders(config.get("models")):
        models = config.get("models")
        if not isinstance(models, list) or not 1 <= len(models) <= 2:
            raise LiveEvalError("live config must contain at most two models")
    else:
        models = model_index(config)
        selected = config.get("selected_model")
        if selected is not None and selected not in models:
            raise LiveEvalError("selected_model is not one of the calibrated models")
    known_case_ids = {
        item["task_id"] for item in campaign["handoff_tasks"]
    } | {item["pair_id"] for item in campaign["contradiction_pairs"]}
    calibration_ids = config.get("calibration_case_ids")
    reserve_ids = config.get("reserve_case_ids")
    if not isinstance(calibration_ids, list) or not calibration_ids:
        raise LiveEvalError("calibration_case_ids must be a non-empty list")
    for field, values in (
        ("calibration_case_ids", calibration_ids),
        ("reserve_case_ids", reserve_ids),
    ):
        if (
            not isinstance(values, list)
            or not all(isinstance(value, str) for value in values)
            or len(values) != len(set(values))
        ):
            raise LiveEvalError(f"{field} must contain unique case IDs")
        if not all(value in known_case_ids for value in values):
            raise LiveEvalError(f"{field} contains an unknown case ID")
    if not any(value.startswith("handoff-") for value in calibration_ids) or not any(
        value.startswith("pair-") for value in calibration_ids
    ):
        raise LiveEvalError(
            "calibration must include a handoff and a contradiction pair"
        )
    rules = config["instability_rules"]
    if (
        not isinstance(rules, list)
        or not all(isinstance(rule, str) for rule in rules)
        or len(rules) != len(set(rules))
        or not set(rules) <= {"quality_metrics_vary", "predicted_labels_vary"}
    ):
        raise LiveEvalError("instability_rules must contain unique supported rules")
    justifications = config["reserve_justifications"]
    if not isinstance(justifications, dict) or set(justifications) != set(reserve_ids):
        raise LiveEvalError("every reserve case requires exactly one justification")
    for justification in justifications.values():
        if (
            not isinstance(justification, dict)
            or set(justification) != {"rule", "main_runs_hash"}
            or justification.get("rule") not in rules
            or not isinstance(justification.get("main_runs_hash"), str)
            or not re.fullmatch(r"[a-f0-9]{64}", justification["main_runs_hash"])
        ):
            raise LiveEvalError(
                "reserve justification requires a registered rule and main runs hash"
            )
    for field, minimum, maximum in (
        ("max_output_tokens", 64, 20_000),
        ("request_overhead_tokens", 1_024, 20_000),
        ("timeout_seconds", 10, 300),
    ):
        value = config.get(field)
        if not isinstance(value, int) or isinstance(value, bool) or not minimum <= value <= maximum:
            raise LiveEvalError(f"{field} must be between {minimum} and {maximum}")
    frozen = config.get("frozen_contract")
    if frozen is not None:
        expected_fields = {
            "selected_model",
            "runner_version",
            "prompt_version",
            "output_schema_version",
            "calibration_runs_hash",
            "campaign_hash",
            "execution_config_hash",
            "private_cases_hash",
            "frozen_at",
        }
        if not isinstance(frozen, dict) or set(frozen) != expected_fields:
            raise LiveEvalError("frozen_contract has an invalid shape")
        if frozen.get("selected_model") != config.get("selected_model"):
            raise LiveEvalError("frozen selected model mismatch")
        if frozen.get("runner_version") != RUNNER_VERSION:
            raise LiveEvalError("frozen runner version mismatch")
        if frozen.get("prompt_version") != PROMPT_VERSION:
            raise LiveEvalError("frozen prompt version mismatch")
        if frozen.get("output_schema_version") != OUTPUT_SCHEMA_VERSION:
            raise LiveEvalError("frozen output schema version mismatch")
        if config.get("selected_model") is None:
            raise LiveEvalError("frozen contract requires a selected model")
        for field in (
            "calibration_runs_hash",
            "campaign_hash",
            "execution_config_hash",
            "private_cases_hash",
        ):
            if not isinstance(frozen.get(field), str) or not re.fullmatch(
                r"[a-f0-9]{64}", frozen[field]
            ):
                raise LiveEvalError(f"frozen {field} is invalid")
        try:
            frozen_at = datetime.fromisoformat(
                frozen["frozen_at"].replace("Z", "+00:00")
            )
            if frozen_at.tzinfo is None:
                raise ValueError("timezone required")
        except (TypeError, ValueError, AttributeError) as error:
            raise LiveEvalError(
                "frozen_at must be a timestamp with timezone"
            ) from error


def validate_private_cases(cases: dict[str, Any], campaign: dict[str, Any]) -> None:
    if cases.get("schema_version") != FORMAT_VERSION:
        raise LiveEvalError("private cases schema_version must be 1.0")
    if cases.get("campaign_id") != campaign.get("campaign_id"):
        raise LiveEvalError("private cases campaign_id mismatch")
    if has_placeholders(cases):
        raise LiveEvalError("private cases still contain REPLACE_ placeholders")
    risks = find_private_data_risks(cases)
    if risks:
        raise LiveEvalError("private cases failed secret/personal-data scan: " + risks[0])
    handoffs = cases.get("handoffs")
    contradictions = cases.get("contradictions")
    if not isinstance(handoffs, dict) or not isinstance(contradictions, dict):
        raise LiveEvalError("private cases must contain handoffs and contradictions")
    expected_handoffs = {item["task_id"] for item in campaign["handoff_tasks"]}
    expected_pairs = {item["pair_id"] for item in campaign["contradiction_pairs"]}
    if set(handoffs) != expected_handoffs or set(contradictions) != expected_pairs:
        raise LiveEvalError("private case IDs do not exactly match the campaign")
    task_manifest = {item["task_id"]: item for item in campaign["handoff_tasks"]}
    for case_id, case in handoffs.items():
        validated_handoff_inputs(case, task_manifest[case_id])
    pair_manifest = {item["pair_id"]: item for item in campaign["contradiction_pairs"]}
    for case_id, case in contradictions.items():
        if not isinstance(case, dict) or set(case) != {"left", "right"}:
            raise LiveEvalError(f"{case_id}: invalid contradiction case contract")
        expected = pair_manifest[case_id]
        for side in ("left", "right"):
            value = case[side]
            if not isinstance(value, dict) or set(value) != {"version_id", "text"}:
                raise LiveEvalError(f"{case_id}:{side}: invalid source contract")
            if value["version_id"] != expected[f"{side}_version_ref"]:
                raise LiveEvalError(f"{case_id}:{side}: version mismatch")
            try:
                uuid.UUID(value["version_id"])
            except (AttributeError, ValueError, TypeError) as error:
                raise LiveEvalError(f"{case_id}:{side}: source UUID required") from error
            if not isinstance(value["text"], str) or not value["text"].strip():
                raise LiveEvalError(f"{case_id}:{side}: text is required")


def validated_handoff_inputs(
    case: dict[str, Any], task: dict[str, Any]
) -> dict[str, Any]:
    """Validate captured exports and derive input metrics from reviewed source mappings."""
    if not isinstance(case, dict) or set(case) != {
        "task_text",
        "project_ref",
        "conditions",
        "annotations",
    }:
        raise LiveEvalError(
            "handoff requires captured exports and reviewed annotations"
        )
    if (
        case["project_ref"] != task["project_ref"]
        or not isinstance(case["task_text"], str)
        or not case["task_text"].strip()
    ):
        raise LiveEvalError("handoff task/project binding is invalid")
    conditions = case["conditions"]
    if not isinstance(conditions, dict) or set(conditions) != set(CONDITIONS):
        raise LiveEvalError("both captured A/B conditions are required")
    for condition in conditions.values():
        if not isinstance(condition, dict) or set(condition) != {
            "payload",
            "allowed_source_ids",
            "context_pack_hash",
        }:
            raise LiveEvalError("invalid captured condition contract")
        validate_uuid_list(condition["allowed_source_ids"], "captured condition")
    pack = conditions["context_pack"]["payload"]
    snapshot = conditions["full_dump"]["payload"]
    if not isinstance(pack, dict) or not isinstance(snapshot, dict):
        raise LiveEvalError(
            "structured ContextPack export and ProjectSnapshot required"
        )
    required_pack_fields = {
        "public_id",
        "version",
        "status",
        "source_graph_version",
        "compiler_version",
        "selection_mode",
        "content_hash",
        "token_budget",
        "token_count",
        "compiled_at",
        "invalidated_at",
        "stale_reason",
        "content",
        "selection_items",
    }
    if set(pack) != required_pack_fields:
        raise LiveEvalError("complete ContextPackSummary JSON export required")
    for field in ("version", "token_budget", "token_count"):
        if (
            not isinstance(pack[field], int)
            or isinstance(pack[field], bool)
            or pack[field] <= 0
        ):
            raise LiveEvalError("invalid ContextPack version or token accounting")
    if pack["token_count"] > pack["token_budget"]:
        raise LiveEvalError("ContextPack exceeds its captured token budget")
    try:
        if (
            datetime.fromisoformat(pack["compiled_at"].replace("Z", "+00:00")).tzinfo
            is None
        ):
            raise ValueError()
    except (TypeError, ValueError, AttributeError) as error:
        raise LiveEvalError("ContextPack compilation timestamp required") from error
    project = snapshot.get("project", {})
    content = pack.get("content", {})
    if not isinstance(project, dict) or not isinstance(content, dict):
        raise LiveEvalError("invalid captured project/content")
    try:
        uuid.UUID(project["public_id"])
        uuid.UUID(pack["public_id"])
    except (KeyError, TypeError, ValueError, AttributeError) as error:
        raise LiveEvalError("captured project and pack UUIDs required") from error
    graph = project.get("graph_version")
    if (
        not isinstance(graph, int)
        or isinstance(graph, bool)
        or graph < 0
        or pack.get("source_graph_version") != graph
        or content.get("graph_version") != graph
    ):
        raise LiveEvalError("captured graph versions differ")
    if (
        pack.get("status") != "current"
        or pack.get("invalidated_at") is not None
        or pack.get("stale_reason") is not None
    ):
        raise LiveEvalError("ContextPack was not current at capture")
    if (
        not isinstance(pack.get("compiler_version"), str)
        or not pack["compiler_version"].strip()
        or pack.get("selection_mode") not in {"hybrid", "deterministic"}
    ):
        raise LiveEvalError("ContextPack compiler provenance is missing")
    expected_content = {
        "objective",
        "project_summary",
        "graph_version",
        "contract",
        "knowledge",
        "provenance",
    }
    if set(content) != expected_content or not isinstance(content["contract"], dict):
        raise LiveEvalError("invalid ContextPack content contract")
    if content["objective"] != project.get("objective") or content[
        "project_summary"
    ] != project.get("summary"):
        raise LiveEvalError("ContextPack project content differs from snapshot")
    try:
        # Non-finite numbers are not valid serde_json values.
        encoded = json.dumps(
            content,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError) as error:
        raise LiveEvalError("invalid ContextPack JSON") from error
    pack_hash = hashlib.sha256(encoded).hexdigest()
    if (
        pack.get("content_hash") != pack_hash
        or conditions["context_pack"]["context_pack_hash"] != pack_hash
    ):
        raise LiveEvalError("ContextPack content hash mismatch")
    if conditions["full_dump"]["context_pack_hash"] is not None:
        raise LiveEvalError("full_dump ContextPack hash must be null")
    source_fields = {
        "knowledge_public_id",
        "version_public_id",
        "version_number",
        "entry_type",
        "title",
        "statement",
        "rationale",
        "node_key",
    }
    full = snapshot.get("knowledge")
    selected = content.get("knowledge")
    if not isinstance(full, list) or not full or not isinstance(selected, list):
        raise LiveEvalError("captured knowledge arrays required")
    normalized = [
        {
            "knowledge_public_id": row.get("public_id"),
            **{key: row.get(key) for key in source_fields - {"knowledge_public_id"}},
        }
        for row in full
        if isinstance(row, dict)
    ]

    def sources_by_id(rows: list[Any]) -> dict[str, Any]:
        result = {}
        for row in rows:
            if not isinstance(row, dict) or set(row) != source_fields:
                raise LiveEvalError("invalid captured knowledge version")
            validate_uuid_list([row["version_public_id"]], "knowledge version")
            validate_uuid_list([row["knowledge_public_id"]], "knowledge entry")
            if (
                not isinstance(row["version_number"], int)
                or isinstance(row["version_number"], bool)
                or row["version_number"] < 1
                or not all(
                    isinstance(row[key], str)
                    for key in (
                        "entry_type",
                        "title",
                        "statement",
                        "rationale",
                        "node_key",
                    )
                )
            ):
                raise LiveEvalError("invalid knowledge version fields")
            if row["version_public_id"] in result:
                raise LiveEvalError("duplicate captured knowledge version")
            result[row["version_public_id"]] = row
        return result

    full_sources, pack_sources = sources_by_id(normalized), sources_by_id(selected)
    if (
        len(normalized) != len(full)
        or not pack_sources
        or any(full_sources.get(key) != row for key, row in pack_sources.items())
    ):
        raise LiveEvalError(
            "ContextPack sources differ from the complete captured versions"
        )
    provenance = content.get("provenance")
    ledger = pack.get("selection_items")
    if not isinstance(provenance, list) or not isinstance(ledger, list):
        raise LiveEvalError("ContextPack provenance and selection ledger required")
    provenance_ids = [
        row.get("version_public_id") for row in provenance if isinstance(row, dict)
    ]
    ledger_ids = [
        row.get("candidate_public_id") for row in ledger if isinstance(row, dict)
    ]
    if (
        len(provenance_ids) != len(provenance)
        or len(set(provenance_ids)) != len(provenance_ids)
        or set(provenance_ids) != set(pack_sources)
    ):
        raise LiveEvalError("ContextPack provenance differs from selected versions")
    for row in provenance:
        if (
            row.get("knowledge_public_id")
            != pack_sources[row["version_public_id"]]["knowledge_public_id"]
            or not row.get("reason_code")
            or not row.get("explanation")
        ):
            raise LiveEvalError("invalid ContextPack source provenance")
    if (
        len(ledger_ids) != len(ledger)
        or len(set(ledger_ids)) != len(ledger_ids)
        or set(ledger_ids) != set(full_sources)
    ):
        raise LiveEvalError("ContextPack selection ledger differs from captured corpus")
    if any(row.get("decision") not in {"included", "excluded"} for row in ledger) or {
        row["candidate_public_id"] for row in ledger if row["decision"] == "included"
    } != set(pack_sources):
        raise LiveEvalError("ContextPack included ledger differs from payload")
    annotations = case["annotations"]
    if (
        not isinstance(annotations, dict)
        or set(annotations) != {"reviewer_ref", "reviewed_at", "facts", "items"}
        or not isinstance(annotations["reviewer_ref"], str)
        or not annotations["reviewer_ref"].strip()
    ):
        raise LiveEvalError("human corpus review provenance required")
    try:
        if (
            datetime.fromisoformat(
                annotations["reviewed_at"].replace("Z", "+00:00")
            ).tzinfo
            is None
        ):
            raise ValueError()
    except (ValueError, TypeError, AttributeError) as error:
        raise LiveEvalError("human corpus review timestamp required") from error
    facts, items = annotations["facts"], annotations["items"]
    expected_items = set(task["relevant_item_ids"]) | set(task["irrelevant_item_ids"])
    if (
        not isinstance(facts, dict)
        or set(facts) != set(task["critical_fact_ids"])
        or not isinstance(items, dict)
        or set(items) != expected_items
    ):
        raise LiveEvalError(
            "annotations must cover the frozen facts and relevance labels"
        )
    if (
        not all(isinstance(value, str) for value in items.values())
        or len(set(items.values())) != len(items)
        or set(items.values()) != set(full_sources)
    ):
        raise LiveEvalError(
            "every captured knowledge unit needs one relevance annotation"
        )
    for fact in facts.values():
        if isinstance(fact, dict) and set(fact) == {"source_version_ids"}:
            validate_uuid_list(fact["source_version_ids"], "critical fact annotation")
            if not set(fact["source_version_ids"]).issubset(full_sources):
                raise LiveEvalError(
                    "critical fact annotation points outside the captured corpus"
                )
        elif isinstance(fact, dict) and set(fact) == {
            "content_pointer",
            "content_hash",
        }:
            if fact["content_pointer"] not in {
                "/objective",
                "/project_summary",
                "/contract",
            } or fact["content_hash"] != sha256_json(
                content[fact["content_pointer"][1:]]
            ):
                raise LiveEvalError(
                    "critical fact content anchor differs from reviewed input"
                )
        else:
            raise LiveEvalError(
                "critical fact requires reviewed versions or content anchor"
            )
    dump_content = {
        "objective": project["objective"],
        "project_summary": project["summary"],
        "graph_version": graph,
        "knowledge": normalized,
    }
    result = {}
    for condition_name, source_map, payload in (
        ("context_pack", pack_sources, content),
        ("full_dump", full_sources, dump_content),
    ):
        condition = conditions[condition_name]
        if set(condition["allowed_source_ids"]) != set(source_map):
            raise LiveEvalError("allowed source IDs differ from captured input")
        fact_count = sum(
            (
                set(fact["source_version_ids"]).issubset(source_map)
                if "source_version_ids" in fact
                else fact["content_pointer"][1:] in payload
                and sha256_json(payload[fact["content_pointer"][1:]])
                == fact["content_hash"]
            )
            for fact in facts.values()
        )
        chosen_items = {
            item_id for item_id, version_id in items.items() if version_id in source_map
        }
        result[condition_name] = {
            "payload": payload,
            "allowed_source_ids": sorted(source_map),
            "context_pack_hash": condition["context_pack_hash"],
            "input_evidence_hash": sha256_json(
                {
                    "project_ref": case["project_ref"],
                    "payload": payload,
                    "annotations": annotations,
                }
            ),
            "critical_facts_expected": len(facts),
            "critical_facts_present": fact_count,
            "relevant_items_expected": len(task["relevant_item_ids"]),
            "relevant_items_present": len(
                chosen_items & set(task["relevant_item_ids"])
            ),
            "selected_items_total": len(source_map),
            "irrelevant_items_selected": len(
                chosen_items & set(task["irrelevant_item_ids"])
            ),
        }
    return result

INPUT_METRIC_FIELDS = (
    "critical_facts_expected", "critical_facts_present", "relevant_items_expected",
    "relevant_items_present", "selected_items_total", "irrelevant_items_selected", "input_evidence_hash",
)


def validate_uuid_list(value: Any, location: str) -> None:
    if (
        not isinstance(value, list)
        or not value
        or not all(isinstance(item, str) for item in value)
        or len(value) != len(set(value))
    ):
        raise LiveEvalError(f"{location}: non-empty unique source UUIDs required")
    try:
        for item in value:
            uuid.UUID(item)
    except (AttributeError, ValueError, TypeError) as error:
        raise LiveEvalError(f"{location}: invalid source UUID") from error


def config_template(campaign: dict[str, Any]) -> dict[str, Any]:
    first_task = campaign["handoff_tasks"][0]["task_id"]
    first_pair = campaign["contradiction_pairs"][0]["pair_id"]
    return {
        "schema_version": FORMAT_VERSION,
        "campaign_id": campaign["campaign_id"],
        "pricing_version": "REPLACE_PRICING_SNAPSHOT_DATE",
        "pricing_verified_at": "REPLACE_ISO_DATE",
        "models": [
            {
                "id": "REPLACE_MODEL_1",
                "input_usd_per_million": "REPLACE_INPUT_PRICE",
                "output_usd_per_million": "REPLACE_OUTPUT_PRICE",
            },
            {
                "id": "REPLACE_MODEL_2",
                "input_usd_per_million": "REPLACE_INPUT_PRICE",
                "output_usd_per_million": "REPLACE_OUTPUT_PRICE",
            },
        ],
        "selected_model": None,
        "calibration_case_ids": [first_task, first_pair],
        "reserve_case_ids": [],
        "instability_rules": ["quality_metrics_vary", "predicted_labels_vary"],
        "reserve_justifications": {},
        "max_output_tokens": 2_000,
        "request_overhead_tokens": 1_024,
        "timeout_seconds": 90,
        "frozen_contract": None,
    }


def private_cases_template(campaign: dict[str, Any]) -> dict[str, Any]:
    handoffs: dict[str, Any] = {}
    for task in campaign["handoff_tasks"]:
        handoffs[task["task_id"]] = {
            "task_text": "REPLACE_PRIVATE_TASK",
            "project_ref": task["project_ref"],
            "annotations": {
                "reviewer_ref": "REPLACE_PSEUDONYMOUS_REVIEWER",
                "reviewed_at": "REPLACE_ISO_TIMESTAMP",
                "facts": {
                    fact: {"source_version_ids": ["REPLACE_SOURCE_UUID"]}
                    for fact in task["critical_fact_ids"]
                },
                "items": {
                    item: "REPLACE_SOURCE_UUID"
                    for item in task["relevant_item_ids"] + task["irrelevant_item_ids"]
                },
            },
            "conditions": {
                "context_pack": {
                    "payload": {"replace": "REPLACE_STRUCTURED_CONTEXT_PACK_EXPORT"},
                    "allowed_source_ids": ["REPLACE_SOURCE_UUID"],
                    "context_pack_hash": "REPLACE_SHA256_CONTEXT_PACK",
                },
                "full_dump": {
                    "payload": {"replace": "REPLACE_STRUCTURED_PROJECT_SNAPSHOT"},
                    "allowed_source_ids": ["REPLACE_SOURCE_UUID"],
                    "context_pack_hash": None,
                },
            },
        }
    contradictions = {
        pair["pair_id"]: {
            "left": {
                "version_id": pair["left_version_ref"],
                "text": "REPLACE_PRIVATE_LEFT_TEXT",
            },
            "right": {
                "version_id": pair["right_version_ref"],
                "text": "REPLACE_PRIVATE_RIGHT_TEXT",
            },
        }
        for pair in campaign["contradiction_pairs"]
    }
    return {
        "schema_version": FORMAT_VERSION,
        "campaign_id": campaign["campaign_id"],
        "handoffs": handoffs,
        "contradictions": contradictions,
    }

def case_kind(case_id: str) -> str:
    return "handoff" if case_id.startswith("handoff-") else "contradiction"


def build_plan(
    campaign: dict[str, Any],
    config: dict[str, Any],
    cases: dict[str, Any],
    stage: str,
    seed: str,
) -> dict[str, Any]:
    if stage not in STAGES:
        raise LiveEvalError("unknown stage")
    models = list(model_index(config))
    if stage == "calibration":
        selected_models = models
        selected_cases = config["calibration_case_ids"]
        repetitions = (1,)
    else:
        frozen = config.get("frozen_contract")
        if not isinstance(frozen, dict) or config.get("selected_model") not in models:
            raise LiveEvalError("freeze a calibrated model before production planning")
        selected_models = [config["selected_model"]]
        if stage == "main":
            selected_cases = [item["task_id"] for item in campaign["handoff_tasks"]]
            selected_cases += [item["pair_id"] for item in campaign["contradiction_pairs"]]
            repetitions = (1, 2, 3)
        else:
            selected_cases = config["reserve_case_ids"]
            repetitions = (4, 5)
            if not selected_cases:
                raise LiveEvalError("reserve_case_ids is empty")

    try:
        randomizer = random.Random(int(seed, 16))
    except ValueError as error:
        raise LiveEvalError("randomization seed must be hexadecimal") from error
    groups: list[list[dict[str, Any]]] = []
    for model in selected_models:
        model_ref = opaque_ref(model)
        for selected_case in selected_cases:
            kind = case_kind(selected_case)
            for repetition in repetitions:
                comparison_id = (
                    f"cmp-{selected_case}-{repetition}-{model_ref}"
                    if kind == "handoff"
                    else None
                )
                if kind == "handoff":
                    labels = ["A", "B"]
                    randomizer.shuffle(labels)
                    condition_jobs = []
                    for condition, blind_label in zip(CONDITIONS, labels, strict=True):
                        condition_jobs.append(
                            plan_job(
                                stage,
                                kind,
                                selected_case,
                                condition,
                                repetition,
                                model,
                                blind_label,
                                comparison_id,
                            )
                        )
                    randomizer.shuffle(condition_jobs)
                    groups.append(condition_jobs)
                else:
                    groups.append(
                        [
                            plan_job(
                                stage,
                                kind,
                                selected_case,
                                "context_pack",
                                repetition,
                                model,
                                "single",
                                None,
                            )
                        ]
                    )
    randomizer.shuffle(groups)
    jobs = [job for group in groups for job in group]
    for sequence, job in enumerate(jobs, start=1):
        job["sequence"] = sequence
    return {
        "schema_version": FORMAT_VERSION,
        "runner_version": RUNNER_VERSION,
        "prompt_version": PROMPT_VERSION,
        "output_schema_version": OUTPUT_SCHEMA_VERSION,
        "campaign_id": campaign["campaign_id"],
        "stage": stage,
        "created_at": utc_now(),
        "randomization_seed": seed,
        "randomization_commitment": hashlib.sha256(seed.encode("ascii")).hexdigest(),
        "campaign_hash": sha256_json(campaign),
        "config_hash": sha256_json(config),
        "private_cases_hash": sha256_json(cases),
        "jobs": jobs,
    }


def plan_job(
    stage: str,
    kind: str,
    case_id: str,
    condition: str,
    repetition: int,
    model: str,
    blind_label: str,
    comparison_id: str | None,
) -> dict[str, Any]:
    identity = {
        "stage": stage,
        "case_kind": kind,
        "case_id": case_id,
        "condition": condition,
        "repetition": repetition,
        "model": model,
    }
    digest = sha256_json(identity)[:20]
    return {
        "job_id": f"job-{digest}",
        "run_id": f"run-{digest}",
        **identity,
        "model_ref": opaque_ref(model),
        "blind_label": blind_label,
        "comparison_id": comparison_id,
    }


def validate_plan(
    plan: dict[str, Any], campaign: dict[str, Any], config: dict[str, Any], cases: dict[str, Any]
) -> None:
    expected = {
        "schema_version": FORMAT_VERSION,
        "runner_version": RUNNER_VERSION,
        "prompt_version": PROMPT_VERSION,
        "output_schema_version": OUTPUT_SCHEMA_VERSION,
        "campaign_id": campaign["campaign_id"],
        "campaign_hash": sha256_json(campaign),
        "config_hash": sha256_json(config),
        "private_cases_hash": sha256_json(cases),
    }
    if any(plan.get(key) != value for key, value in expected.items()):
        raise LiveEvalError("plan contract or source hashes no longer match")
    seed = plan.get("randomization_seed")
    if not isinstance(seed, str):
        raise LiveEvalError("plan randomization seed is missing")
    try:
        commitment = hashlib.sha256(seed.encode("ascii")).hexdigest()
    except UnicodeEncodeError as error:
        raise LiveEvalError("plan randomization seed must be hexadecimal ASCII") from error
    if plan.get("randomization_commitment") != commitment:
        raise LiveEvalError("plan randomization commitment mismatch")
    rebuilt = build_plan(campaign, config, cases, str(plan.get("stage")), seed)
    if plan.get("jobs") != rebuilt["jobs"]:
        raise LiveEvalError("plan jobs were modified after randomization")
    jobs = plan.get("jobs")
    if not isinstance(jobs, list) or not jobs:
        raise LiveEvalError("plan has no jobs")
    job_ids = [job.get("job_id") for job in jobs if isinstance(job, dict)]
    run_ids = [job.get("run_id") for job in jobs if isinstance(job, dict)]
    if len(job_ids) != len(jobs) or len(set(job_ids)) != len(job_ids):
        raise LiveEvalError("plan job IDs are invalid or duplicated")
    if len(set(run_ids)) != len(run_ids):
        raise LiveEvalError("plan run IDs are duplicated")
    comparison_labels: dict[str, set[str]] = defaultdict(set)
    for job in jobs:
        if job.get("case_kind") == "handoff":
            comparison_labels[job["comparison_id"]].add(job.get("blind_label"))
    if any(labels != {"A", "B"} for labels in comparison_labels.values()):
        raise LiveEvalError("every handoff comparison must contain blinded A/B labels")


HANDOFF_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "required": [
        "source_version_ids",
        "deliverable",
    ],
    "properties": {
        "source_version_ids": {"type": "array", "items": {"type": "string"}, "uniqueItems": True},
        "deliverable": {
            "type": "object",
            "additionalProperties": False,
            "required": ["summary", "actions", "risks"],
            "properties": {
                "summary": {"type": "string"},
                "actions": {"type": "array", "items": {"type": "string"}},
                "risks": {"type": "array", "items": {"type": "string"}},
            },
        },
    },
}

CONTRADICTION_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "required": ["source_version_ids", "predicted_label", "confidence", "explanation"],
    "properties": {
        "source_version_ids": {
            "type": "array",
            "minItems": 2,
            "maxItems": 2,
            "items": {"type": "string", "format": "uuid"},
            "uniqueItems": True,
        },
        "predicted_label": {"enum": ["contradiction", "compatible", "ambiguous"]},
        "confidence": {"type": "number", "minimum": 0, "maximum": 1},
        "explanation": {"type": "string"},
    },
}


def request_for_job(
    job: dict[str, Any],
    campaign: dict[str, Any],
    config: dict[str, Any],
    cases: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    if job["case_kind"] == "handoff":
        private_case = cases["handoffs"][job["case_id"]]
        task = next(
            task
            for task in campaign["handoff_tasks"]
            if task["task_id"] == job["case_id"]
        )
        condition = validated_handoff_inputs(private_case, task)[job["condition"]]
        input_value = {
            "task": private_case["task_text"],
            "context": condition["payload"],
            "allowed_source_ids": condition["allowed_source_ids"],
        }
        schema = HANDOFF_SCHEMA
        instructions = (
            "Produce a task handoff using only the supplied context. Cite only supplied source IDs. "
            "Do not claim that code or evidence exists. The response language must match the task."
        )
        scoring = {
            "allowed_source_ids": condition["allowed_source_ids"],
            "context_pack_hash": condition["context_pack_hash"],
            **{field: condition[field] for field in INPUT_METRIC_FIELDS},
        }
    else:
        private_case = cases["contradictions"][job["case_id"]]
        input_value = private_case
        schema = CONTRADICTION_SCHEMA
        instructions = (
            "Classify the supplied pair as contradiction, compatible, or ambiguous. "
            "Cite exactly the two supplied version IDs and do not invent sources."
        )
        scoring = {
            "allowed_source_ids": [
                private_case["left"]["version_id"],
                private_case["right"]["version_id"],
            ],
            "context_pack_hash": sha256_json(private_case),
        }
    body = {
        "model": job["model"],
        "store": False,
        "instructions": instructions,
        "input": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": canonical_json(input_value).decode("utf-8"),
                    }
                ],
            }
        ],
        "max_output_tokens": config["max_output_tokens"],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "alpha_context_proof_live_output",
                "strict": True,
                "schema": schema,
            }
        },
        "metadata": {
            "campaign": opaque_ref(campaign["campaign_id"]),
            "run": job["run_id"],
            "runner_version": RUNNER_VERSION,
            "prompt_version": PROMPT_VERSION,
            "schema_version": OUTPUT_SCHEMA_VERSION,
        },
    }
    return body, scoring

def request_upper_bound_cost(
    body: dict[str, Any], config: dict[str, Any], model: dict[str, Any]
) -> tuple[int, Decimal]:
    input_token_upper_bound = len(canonical_json(body)) * 2 + config["request_overhead_tokens"]
    input_price = decimal_value(model["input_usd_per_million"], "input price")
    output_price = decimal_value(model["output_usd_per_million"], "output price")
    maximum = (
        Decimal(input_token_upper_bound) * input_price
        + Decimal(config["max_output_tokens"]) * output_price
    ) / Decimal(1_000_000)
    return input_token_upper_bound, money(maximum)


class BudgetLedger:
    """Append-only, process-safe reservations for the 10/70/20 USD budget."""

    def __init__(self, path: Path):
        self.path = path
        self.lock_path = path.with_suffix(path.suffix + ".lock")

    def _locked(self) -> BinaryIO:
        self.lock_path.parent.mkdir(parents=True, exist_ok=True)
        handle = open(self.lock_path, "a+b")  # noqa: SIM115 - returned locked handle
        os.chmod(self.lock_path, 0o600)
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        return handle

    def _state(self) -> tuple[dict[str, dict[str, Any]], dict[str, Decimal]]:
        reservations: dict[str, dict[str, Any]] = {}
        charges = {stage: Decimal("0") for stage in STAGES}
        for event in read_jsonl(self.path):
            kind = event.get("event")
            reservation_id = event.get("reservation_id")
            if kind == "reserve":
                stage = event.get("stage")
                run_id = event.get("run_id")
                if (
                    not isinstance(reservation_id, str)
                    or not isinstance(run_id, str)
                    or stage not in STAGES
                ):
                    raise LiveEvalError("budget ledger contains an invalid reservation")
                if reservation_id in reservations:
                    raise LiveEvalError("budget ledger contains a duplicate reservation")
                reservations[reservation_id] = {
                    "run_id": run_id,
                    "stage": stage,
                    "reserved": decimal_value(event.get("max_cost_usd"), "reserved cost"),
                    "status": "pending",
                }
            elif kind == "settle":
                if not isinstance(reservation_id, str):
                    raise LiveEvalError("budget ledger contains an invalid settlement")
                reservation = reservations.get(reservation_id)
                if reservation is None or reservation["status"] != "pending":
                    raise LiveEvalError("budget ledger contains an invalid settlement")
                reservation["actual"] = decimal_value(event.get("actual_cost_usd"), "actual cost")
                reservation["status"] = "settled"
            else:
                raise LiveEvalError("budget ledger contains an unknown event")
        for reservation in reservations.values():
            charge = (
                reservation["reserved"]
                if reservation["status"] == "pending"
                else reservation["actual"]
            )
            charges[reservation["stage"]] += charge
        return reservations, charges

    def reserve(self, reservation_id: str, stage: str, max_cost: Decimal, run_id: str) -> None:
        if stage not in STAGES:
            raise LiveEvalError("invalid budget stage")
        handle = self._locked()
        try:
            reservations, charges = self._state()
            if reservation_id in reservations:
                raise LiveEvalError("this run already has a budget reservation")
            total = sum(charges.values(), Decimal("0"))
            if charges[stage] + max_cost > STAGE_LIMITS[stage] or total + max_cost > TOTAL_LIMIT:
                raise LiveEvalError("hard budget stop: next request exceeds its stage or total cap")
            append_jsonl_locked(
                self.path,
                {
                    "schema_version": FORMAT_VERSION,
                    "event": "reserve",
                    "reservation_id": reservation_id,
                    "run_id": run_id,
                    "stage": stage,
                    "max_cost_usd": str(max_cost),
                    "at": utc_now(),
                },
            )
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
            handle.close()

    def settle(self, reservation_id: str, actual_cost: Decimal) -> None:
        handle = self._locked()
        try:
            reservations, _ = self._state()
            reservation = reservations.get(reservation_id)
            if reservation is None or reservation["status"] != "pending":
                raise LiveEvalError("budget reservation is missing or already settled")
            if actual_cost > reservation["reserved"]:
                raise LiveEvalError("actual cost exceeded the conservative reservation")
            append_jsonl_locked(
                self.path,
                {
                    "schema_version": FORMAT_VERSION,
                    "event": "settle",
                    "reservation_id": reservation_id,
                    "actual_cost_usd": str(actual_cost),
                    "at": utc_now(),
                },
            )
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
            handle.close()

    def snapshot(self) -> dict[str, Any]:
        handle = self._locked()
        try:
            reservations, charges = self._state()
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
            handle.close()
        pending = sum(1 for item in reservations.values() if item["status"] == "pending")
        return {
            "stage_charged_usd": {stage: float(charges[stage]) for stage in STAGES},
            "total_charged_usd": float(sum(charges.values(), Decimal("0"))),
            "pending_reservations": pending,
        }

    def reconcile(self, runs: list[dict[str, Any]]) -> int:
        """Settle crash-left reservations only from already persisted run metrics."""

        runs_by_id = {record.get("run_id"): record for record in runs}
        handle = self._locked()
        settled = 0
        try:
            reservations, _ = self._state()
            for reservation_id, reservation in reservations.items():
                if reservation["status"] != "pending":
                    continue
                record = runs_by_id.get(reservation["run_id"])
                if record is None:
                    continue
                actual_cost = decimal_value(record.get("cost_usd"), "run cost")
                if actual_cost > reservation["reserved"]:
                    raise LiveEvalError("persisted run cost exceeds its reservation")
                append_jsonl_locked(
                    self.path,
                    {
                        "schema_version": FORMAT_VERSION,
                        "event": "settle",
                        "reservation_id": reservation_id,
                        "actual_cost_usd": str(actual_cost),
                        "at": utc_now(),
                        "reconciled": True,
                    },
                )
                settled += 1
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
            handle.close()
        return settled


class ResponsesClient:
    def __init__(self, opener: Any | None = None):
        self.opener = opener or urllib.request.urlopen

    def create(self, body: dict[str, Any], api_key: str, timeout_seconds: int) -> dict[str, Any]:
        request = urllib.request.Request(
            RESPONSES_API_URL,
            data=canonical_json(body),
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
                "User-Agent": f"ai-center-alpha-live-eval/{RUNNER_VERSION}",
            },
            method="POST",
        )
        try:
            with self.opener(request, timeout=timeout_seconds) as response:
                raw = response.read()
        except urllib.error.HTTPError as error:
            raise LiveEvalError(f"provider_http_{error.code}") from error
        except urllib.error.URLError as error:
            raise LiveEvalError("provider_transport_failure") from error
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as error:
            raise LiveEvalError("provider_unreadable_response") from error
        if not isinstance(value, dict):
            raise LiveEvalError("provider_invalid_response_shape")
        return value


def extract_structured_output(response: dict[str, Any]) -> dict[str, Any] | None:
    output_text = response.get("output_text")
    if not isinstance(output_text, str):
        for item in response.get("output", []):
            if not isinstance(item, dict) or item.get("type") != "message":
                continue
            for content in item.get("content", []):
                if isinstance(content, dict) and content.get("type") == "output_text":
                    output_text = content.get("text")
                    break
    if not isinstance(output_text, str):
        return None
    try:
        value = json.loads(output_text)
    except json.JSONDecodeError:
        return None
    return value if isinstance(value, dict) else None


def validate_model_output(kind: str, output: dict[str, Any] | None) -> bool:
    if output is None:
        return False
    if kind == "handoff":
        if set(output) != {
            "source_version_ids",
            "deliverable",
        }:
            return False
        if not all(
            isinstance(output[field], list)
            and all(isinstance(item, str) for item in output[field])
            and len(output[field]) == len(set(output[field]))
            for field in ("source_version_ids",)
        ):
            return False
        deliverable = output["deliverable"]
        return (
            isinstance(deliverable, dict)
            and set(deliverable) == {"summary", "actions", "risks"}
            and isinstance(deliverable["summary"], str)
            and bool(deliverable["summary"].strip())
            and bool(deliverable["actions"])
            and all(
                isinstance(deliverable[field], list)
                and all(
                    isinstance(item, str) and item.strip()
                    for item in deliverable[field]
                )
                for field in ("actions", "risks")
            )
        )
    if set(output) != {
        "source_version_ids",
        "predicted_label",
        "confidence",
        "explanation",
    }:
        return False
    confidence = output["confidence"]
    return (
        isinstance(output["source_version_ids"], list)
        and len(output["source_version_ids"]) == 2
        and len(set(output["source_version_ids"])) == 2
        and output["predicted_label"] in {"contradiction", "compatible", "ambiguous"}
        and isinstance(confidence, (int, float))
        and not isinstance(confidence, bool)
        and 0 <= confidence <= 1
        and isinstance(output["explanation"], str)
    )

def usage_and_cost(response: dict[str, Any], model: dict[str, Any]) -> tuple[int, int, Decimal]:
    usage = response.get("usage")
    if not isinstance(usage, dict):
        raise LiveEvalError("provider response has no billable usage; reservation remains pending")
    input_tokens = usage.get("input_tokens")
    output_tokens = usage.get("output_tokens")
    if not isinstance(input_tokens, int) or input_tokens <= 0:
        raise LiveEvalError("provider usage has invalid input tokens; reservation remains pending")
    if not isinstance(output_tokens, int) or output_tokens < 0:
        raise LiveEvalError("provider usage has invalid output tokens; reservation remains pending")
    actual = (
        Decimal(input_tokens) * decimal_value(model["input_usd_per_million"], "input price")
        + Decimal(output_tokens) * decimal_value(model["output_usd_per_million"], "output price")
    ) / Decimal(1_000_000)
    return input_tokens, output_tokens, money(actual)


def build_run_record(
    job: dict[str, Any],
    campaign: dict[str, Any],
    scoring: dict[str, Any],
    response: dict[str, Any],
    output: dict[str, Any] | None,
    input_tokens: int,
    output_tokens: int,
    actual_cost: Decimal,
    latency_ms: int,
    request_hash: str,
) -> dict[str, Any]:
    valid_output = response.get("status") == "completed" and validate_model_output(
        job["case_kind"], output
    )
    source_ids = output.get("source_version_ids", []) if valid_output and output else []
    source_ids_valid = bool(source_ids) and set(source_ids).issubset(
        scoring["allowed_source_ids"]
    )
    record: dict[str, Any] = {
        "schema_version": FORMAT_VERSION,
        "run_id": job["run_id"],
        "runner_version": RUNNER_VERSION,
        "case_kind": job["case_kind"],
        "case_id": job["case_id"],
        "project_ref": project_for_case(campaign, job["case_id"]),
        "stage": job["stage"],
        "condition": job["condition"],
        "repetition": job["repetition"],
        "model": job["model"],
        "prompt_version": PROMPT_VERSION,
        "output_schema_version": OUTPUT_SCHEMA_VERSION,
        "context_pack_hash": scoring["context_pack_hash"],
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "latency_ms": latency_ms,
        "cost_usd": float(actual_cost),
        "structured_output_valid": valid_output,
        "source_ids_valid": source_ids_valid,
        "attempts": 1,
        "provider_status": str(response.get("status", "unknown")),
        "provider_response_ref": opaque_ref(str(response.get("id", "missing"))),
        "request_hash": request_hash,
    }
    if job["case_kind"] == "handoff":
        record.update({field: scoring[field] for field in INPUT_METRIC_FIELDS})
    else:
        manifest_pair = next(
            item
            for item in campaign["contradiction_pairs"]
            if item["pair_id"] == job["case_id"]
        )
        record.update(
            {
                "expected_label": manifest_pair["expected_label"],
                "predicted_label": (
                    output.get("predicted_label", "ambiguous")
                    if valid_output and output
                    else "ambiguous"
                ),
            }
        )
    return record

def project_for_case(campaign: dict[str, Any], case_id: str) -> str:
    collection = (
        campaign["handoff_tasks"] if case_id.startswith("handoff-") else campaign["contradiction_pairs"]
    )
    identifier = "task_id" if case_id.startswith("handoff-") else "pair_id"
    return next(item["project_ref"] for item in collection if item[identifier] == case_id)


def calibration_summary(config: dict[str, Any], runs: list[dict[str, Any]]) -> dict[str, Any]:
    models = model_index(config)
    calibration = [record for record in runs if record.get("stage") == "calibration"]
    summaries: list[dict[str, Any]] = []
    expected_per_model = len(config["calibration_case_ids"])
    expected_per_model += sum(
        1 for case_id in config["calibration_case_ids"] if case_id.startswith("handoff-")
    )
    for model_id in models:
        records = [record for record in calibration if record.get("model") == model_id]
        expected_keys = {
            (case_id, condition, 1)
            for case_id in config["calibration_case_ids"]
            for condition in (
                CONDITIONS if case_id.startswith("handoff-") else ("context_pack",)
            )
        }
        observed_keys = [
            (record.get("case_id"), record.get("condition"), record.get("repetition"))
            for record in records
        ]
        structured = sum(bool(record.get("structured_output_valid")) for record in records)
        sources = sum(bool(record.get("source_ids_valid")) for record in records)
        facts_expected = sum(int(record.get("critical_facts_expected", 0)) for record in records)
        facts_present = sum(int(record.get("critical_facts_present", 0)) for record in records)
        relevant_expected = sum(int(record.get("relevant_items_expected", 0)) for record in records)
        relevant_present = sum(int(record.get("relevant_items_present", 0)) for record in records)
        contradictions = [record for record in records if record.get("case_kind") == "contradiction"]
        correct = sum(
            record.get("expected_label") == record.get("predicted_label")
            for record in contradictions
        )
        complete = (
            len(records) == expected_per_model
            and len(set(observed_keys)) == len(observed_keys)
            and set(observed_keys) == expected_keys
        )
        schema_rate = structured / len(records) if records else 0.0
        source_rate = sources / len(records) if records else 0.0
        fact_recall = facts_present / facts_expected if facts_expected else 1.0
        relevant_recall = relevant_present / relevant_expected if relevant_expected else 1.0
        contradiction_accuracy = correct / len(contradictions) if contradictions else 1.0
        passed = (
            complete
            and schema_rate >= 0.95
            and source_rate == 1.0
            and fact_recall == 1.0
            and relevant_recall >= 0.85
            and contradiction_accuracy >= 0.70
        )
        summaries.append(
            {
                "model": model_id,
                "model_ref": opaque_ref(model_id),
                "runs": len(records),
                "expected_runs": expected_per_model,
                "structured_output_validity": round(schema_rate, 6),
                "source_validity": round(source_rate, 6),
                "critical_fact_recall": round(fact_recall, 6),
                "relevant_context_recall": round(relevant_recall, 6),
                "contradiction_accuracy": round(contradiction_accuracy, 6),
                "cost_usd": round(sum(float(record.get("cost_usd", 0)) for record in records), 6),
                "passed": passed,
            }
        )
    eligible = [summary for summary in summaries if summary["passed"]]
    recommended = (
        min(eligible, key=lambda item: (item["cost_usd"], item["model"]))
        if eligible
        else None
    )
    return {
        "schema_version": FORMAT_VERSION,
        "runner_version": RUNNER_VERSION,
        "models": summaries,
        "recommended_model": recommended["model"] if recommended else None,
    }


def execution_config_hash(config: dict[str, Any]) -> str:
    # Reserve decisions follow the main runs; their rules are frozen beforehand.
    mutable = {
        "selected_model",
        "frozen_contract",
        "reserve_case_ids",
        "reserve_justifications",
    }
    return sha256_json(
        {key: value for key, value in config.items() if key not in mutable}
    )


def runs_hash(runs: list[dict[str, Any]]) -> str:
    return sha256_json(
        sorted(
            [
                {key: value for key, value in record.items() if key != "__line__"}
                for record in runs
            ],
            key=lambda record: record["run_id"],
        )
    )


def validate_run_contracts(
    config: dict[str, Any],
    campaign: dict[str, Any],
    runs: list[dict[str, Any]],
    cases: dict[str, Any] | None = None,
) -> None:
    validator = aggregate_validator()
    if cases is not None:
        validate_private_cases(cases, campaign)
    try:
        validator.validate_runs([dict(record) for record in runs], campaign)
    except validator.ValidationError as error:
        raise LiveEvalError(str(error)) from error
    for record in runs:
        for field, expected in (
            ("runner_version", RUNNER_VERSION),
            ("prompt_version", PROMPT_VERSION),
            ("output_schema_version", OUTPUT_SCHEMA_VERSION),
            ("attempts", 1),
        ):
            if record.get(field) != expected:
                raise LiveEvalError(
                    f"run {field} differs from the frozen execution contract"
                )
        if record["model"] not in model_index(config):
            raise LiveEvalError("run model was not configured for calibration")
        if (
            record["stage"] != "calibration"
            and record["model"] != config["selected_model"]
        ):
            raise LiveEvalError("production run model differs from the selected model")
        job = plan_job(
            record["stage"],
            record["case_kind"],
            record["case_id"],
            record["condition"],
            record["repetition"],
            record["model"],
            "single",
            None,
        )
        if record["run_id"] != job["run_id"]:
            raise LiveEvalError(
                "run identity differs from the planned execution contract"
            )
        if not isinstance(record.get("request_hash"), str) or not re.fullmatch(
            r"[a-f0-9]{64}", record["request_hash"]
        ):
            raise LiveEvalError("run requires its request hash")
        if cases is not None:
            body, scoring = request_for_job(job, campaign, config, cases)
            if (
                record["request_hash"] != sha256_json(body)
                or record["context_pack_hash"] != scoring["context_pack_hash"]
            ):
                raise LiveEvalError(
                    "run request or ContextPack differs from the private corpus"
                )
            if record["case_kind"] == "handoff" and any(
                record.get(field) != scoring[field] for field in INPUT_METRIC_FIELDS
            ):
                raise LiveEvalError(
                    "run input metrics differ from verified corpus annotations"
                )

def validate_calibration_evidence(
    config: dict[str, Any],
    campaign: dict[str, Any],
    runs: list[dict[str, Any]],
    cases: dict[str, Any] | None = None,
) -> dict[str, Any]:
    calibration = [record for record in runs if record.get("stage") == "calibration"]
    validate_run_contracts(config, campaign, calibration, cases)
    expected = {
        (model, case_id, condition, 1)
        for model in model_index(config)
        for case_id in config["calibration_case_ids"]
        for condition in (
            CONDITIONS if case_id.startswith("handoff-") else ("context_pack",)
        )
    }
    observed = [
        (row["model"], row["case_id"], row["condition"], row["repetition"])
        for row in calibration
    ]
    if len(observed) != len(expected) or set(observed) != expected:
        raise LiveEvalError(
            "calibration sample must be complete for every configured model"
        )
    if (
        sum(decimal_value(row["cost_usd"], "calibration cost") for row in calibration)
        > STAGE_LIMITS["calibration"]
    ):
        raise LiveEvalError("calibration cost exceeds 10 USD")
    summary = calibration_summary(config, calibration)
    if summary["recommended_model"] is None:
        raise LiveEvalError("no calibrated model reached the frozen quality thresholds")
    return summary


def validate_frozen_evidence(
    config: dict[str, Any],
    campaign: dict[str, Any],
    runs: list[dict[str, Any]],
    cases: dict[str, Any] | None = None,
) -> None:
    validate_live_config(config, campaign)
    frozen = config["frozen_contract"]
    if frozen is None:
        raise LiveEvalError("production requires a frozen calibration contract")
    if frozen["campaign_hash"] != sha256_json(campaign) or frozen[
        "execution_config_hash"
    ] != execution_config_hash(config):
        raise LiveEvalError(
            "campaign or execution config changed after calibration freeze"
        )
    if cases is not None and frozen["private_cases_hash"] != sha256_json(cases):
        raise LiveEvalError("private corpus changed after calibration freeze")
    validate_run_contracts(config, campaign, runs, cases)
    summary = validate_calibration_evidence(config, campaign, runs, cases)
    calibration = [row for row in runs if row["stage"] == "calibration"]
    if frozen["calibration_runs_hash"] != runs_hash(calibration):
        raise LiveEvalError("calibration runs changed after freeze")
    if summary["recommended_model"] != config["selected_model"]:
        raise LiveEvalError(
            "selected model differs from the calibration recommendation"
        )


def validate_reserve_evidence(
    config: dict[str, Any], runs: list[dict[str, Any]]
) -> None:
    reserved = set(config["reserve_case_ids"])
    if any(row["case_id"] not in reserved for row in runs if row["stage"] == "reserve"):
        raise LiveEvalError("reserve run has no recorded instability justification")
    for case_id, justification in config["reserve_justifications"].items():
        main_runs = [
            row for row in runs if row["stage"] == "main" and row["case_id"] == case_id
        ]
        conditions = CONDITIONS if case_id.startswith("handoff-") else ("context_pack",)
        expected = {
            (condition, repetition)
            for condition in conditions
            for repetition in (1, 2, 3)
        }
        observed = [(row["condition"], row["repetition"]) for row in main_runs]
        if len(observed) != len(expected) or set(observed) != expected:
            raise LiveEvalError(
                "reserve requires all three main repetitions for each condition"
            )
        if justification["main_runs_hash"] != runs_hash(main_runs):
            raise LiveEvalError("reserve main runs hash mismatch")
        rule = justification["rule"]
        if rule == "predicted_labels_vary":
            unstable = (
                case_id.startswith("pair-")
                and len({row["predicted_label"] for row in main_runs}) > 1
            )
        elif rule == "quality_metrics_vary":
            fields = ("structured_output_valid", "source_ids_valid")
            unstable = case_id.startswith("handoff-") and any(
                len(
                    {
                        tuple(row.get(field) for field in fields)
                        for row in main_runs
                        if row["condition"] == condition
                    }
                )
                > 1
                for condition in conditions
            )
        else:
            raise LiveEvalError("unsupported instability rule")
        if not unstable:
            raise LiveEvalError(
                "reserve rule is not triggered by the recorded main runs"
            )


def command_init(args: argparse.Namespace) -> int:
    private_root = default_private_root()
    manifest_path = ensure_private_path(args.manifest, private_root)
    config_path = ensure_private_path(args.config, private_root)
    cases_path = ensure_private_path(args.cases, private_root)
    campaign = read_json(manifest_path)
    validate_campaign(campaign, allow_placeholders=True)
    write_json_private(config_path, config_template(campaign), overwrite=args.force)
    write_json_private(cases_path, private_cases_template(campaign), overwrite=args.force)
    emit_event(event="live_eval_initialized", status="offline")
    return 0


def load_live_inputs(args: argparse.Namespace) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    private_root = default_private_root()
    campaign = read_json(ensure_private_path(args.manifest, private_root))
    config = read_json(ensure_private_path(args.config, private_root))
    cases = read_json(ensure_private_path(args.cases, private_root))
    validate_campaign(campaign)
    validate_live_config(config, campaign)
    validate_private_cases(cases, campaign)
    return campaign, config, cases


def command_plan(args: argparse.Namespace) -> int:
    campaign, config, cases = load_live_inputs(args)
    if args.stage != "calibration":
        runs = read_jsonl(ensure_private_path(args.runs), allow_missing=False)
        validate_frozen_evidence(config, campaign, runs, cases)
        if args.stage == "reserve":
            validate_reserve_evidence(config, runs)
    output = ensure_private_path(args.output)
    seed = args.seed or secrets.token_hex(32)
    plan = build_plan(campaign, config, cases, args.stage, seed)
    write_json_private(output, plan, overwrite=args.force)
    emit_event(
        event="plan_created",
        stage=args.stage,
        status="offline",
        request_hash=sha256_json(plan),
    )
    return 0


def command_run(args: argparse.Namespace) -> int:
    campaign, config, cases = load_live_inputs(args)
    plan_path = ensure_private_path(args.plan)
    plan = read_json(plan_path)
    validate_plan(plan, campaign, config, cases)
    job = next((item for item in plan["jobs"] if item["job_id"] == args.job_id), None)
    if job is None:
        raise LiveEvalError("job_id is not present in the plan")
    runs_path = ensure_private_path(args.runs)
    runs = read_jsonl(runs_path)
    if job["stage"] != "calibration":
        validate_frozen_evidence(config, campaign, runs, cases)
        if job["stage"] == "reserve":
            validate_reserve_evidence(config, runs)
    elif config["frozen_contract"] is not None:
        raise LiveEvalError("calibration cannot continue after the contract is frozen")
    events_path = ensure_private_path(args.events)
    raw_dir = ensure_private_path(args.raw_dir)
    blind_dir = ensure_private_path(args.blind_dir)
    if any(record.get("run_id") == job["run_id"] for record in runs):
        raise LiveEvalError("run already completed; live retries are forbidden")
    body, scoring = request_for_job(job, campaign, config, cases)
    model = model_index(config)[job["model"]]
    input_upper, max_cost = request_upper_bound_cost(body, config, model)
    request_hash = hashlib.sha256(canonical_json(body)).hexdigest()
    if not args.execute:
        emit_event(
            events_path,
            event="dry_run",
            job_id=job["job_id"],
            run_id=job["run_id"],
            stage=job["stage"],
            case_kind=job["case_kind"],
            case_id=job["case_id"],
            model_ref=job["model_ref"],
            status="no_network",
            request_hash=request_hash,
            input_tokens=input_upper,
            reserved_usd=float(max_cost),
        )
        return 0
    if args.confirm != LIVE_CONFIRMATION:
        raise LiveEvalError("live confirmation token is missing or incorrect")
    api_key = os.environ.get(API_KEY_ENV)
    if not api_key:
        raise LiveEvalError(f"{API_KEY_ENV} is not present in the process environment")
    ledger = BudgetLedger(ensure_private_path(args.ledger))
    reservation_id = f"reservation-{hashlib.sha256(job['run_id'].encode()).hexdigest()[:24]}"
    ledger.reserve(reservation_id, job["stage"], max_cost, job["run_id"])
    emit_event(
        events_path,
        event="request_reserved",
        run_id=job["run_id"],
        stage=job["stage"],
        case_kind=job["case_kind"],
        case_id=job["case_id"],
        model_ref=job["model_ref"],
        status="pending",
        request_hash=request_hash,
        reserved_usd=float(max_cost),
    )
    started = time.monotonic()
    try:
        response = ResponsesClient().create(body, api_key, config["timeout_seconds"])
    except LiveEvalError as error:
        emit_event(
            events_path,
            event="request_uncertain",
            run_id=job["run_id"],
            stage=job["stage"],
            case_kind=job["case_kind"],
            case_id=job["case_id"],
            model_ref=job["model_ref"],
            status="reservation_pending",
            request_hash=request_hash,
            reason_code=str(error),
        )
        raise
    latency_ms = int((time.monotonic() - started) * 1_000)
    raw_path = raw_dir / f"{job['run_id']}.json"
    write_json_private(raw_path, response, overwrite=False)
    output = extract_structured_output(response)
    input_tokens, output_tokens, actual_cost = usage_and_cost(response, model)
    run_record = build_run_record(
        job,
        campaign,
        scoring,
        response,
        output,
        input_tokens,
        output_tokens,
        actual_cost,
        latency_ms,
        request_hash,
    )
    append_jsonl_locked(runs_path, run_record)
    if output is not None and run_record["structured_output_valid"]:
        blind_path = blind_dir / f"{job['comparison_id'] or job['run_id']}-{job['blind_label']}.json"
        write_json_private(
            blind_path,
            {
                "comparison_id": job["comparison_id"],
                "blind_label": job["blind_label"],
                "case_id": job["case_id"],
                "repetition": job["repetition"],
                "output": output,
            },
            overwrite=False,
        )
    ledger.settle(reservation_id, actual_cost)
    emit_event(
        events_path,
        event="request_completed",
        run_id=job["run_id"],
        stage=job["stage"],
        case_kind=job["case_kind"],
        case_id=job["case_id"],
        model_ref=job["model_ref"],
        status="completed",
        request_hash=request_hash,
        provider_response_ref=run_record["provider_response_ref"],
        input_tokens=input_tokens,
        output_tokens=output_tokens,
        cost_usd=float(actual_cost),
    )
    return 0


def command_freeze(args: argparse.Namespace) -> int:
    private_root = default_private_root()
    campaign = read_json(ensure_private_path(args.manifest, private_root))
    config_path = ensure_private_path(args.config, private_root)
    config = read_json(config_path)
    cases = read_json(ensure_private_path(args.cases, private_root))
    validate_campaign(campaign)
    validate_live_config(config, campaign)
    validate_private_cases(cases, campaign)
    if config.get("frozen_contract") is not None:
        raise LiveEvalError("model contract is already frozen")
    runs = read_jsonl(ensure_private_path(args.runs, private_root), allow_missing=False)
    if any(record.get("stage") != "calibration" for record in runs):
        raise LiveEvalError("production runs cannot predate the calibration freeze")
    if config["reserve_case_ids"]:
        raise LiveEvalError("reserve decisions must follow the main campaign")
    summary = validate_calibration_evidence(config, campaign, runs, cases)
    selected = summary["recommended_model"]
    if selected is None:
        raise LiveEvalError("no calibrated model reached the frozen quality thresholds")
    config["selected_model"] = selected
    config["frozen_contract"] = {
        "selected_model": selected,
        "runner_version": RUNNER_VERSION,
        "prompt_version": PROMPT_VERSION,
        "output_schema_version": OUTPUT_SCHEMA_VERSION,
        "calibration_runs_hash": runs_hash(runs),
        "campaign_hash": sha256_json(campaign),
        "execution_config_hash": execution_config_hash(config),
        "private_cases_hash": sha256_json(cases),
        "frozen_at": utc_now(),
    }
    write_json_private(config_path, config, overwrite=True)
    emit_event(
        event="model_frozen",
        stage="calibration",
        model_ref=opaque_ref(selected),
        status="frozen",
        request_hash=sha256_json(summary),
    )
    return 0


def command_register_reserve(args: argparse.Namespace) -> int:
    campaign = read_json(ensure_private_path(args.manifest))
    config_path = ensure_private_path(args.config)
    config = read_json(config_path)
    runs = read_jsonl(ensure_private_path(args.runs), allow_missing=False)
    validate_campaign(campaign)
    validate_frozen_evidence(config, campaign, runs, read_json(ensure_private_path(args.cases)))
    if args.case_id in config["reserve_case_ids"]:
        raise LiveEvalError("reserve justification is already registered")
    main_runs = [
        row for row in runs if row["stage"] == "main" and row["case_id"] == args.case_id
    ]
    config["reserve_case_ids"].append(args.case_id)
    config["reserve_justifications"][args.case_id] = {
        "rule": args.rule,
        "main_runs_hash": runs_hash(main_runs),
    }
    validate_live_config(config, campaign)
    validate_reserve_evidence(config, runs)
    write_json_private(config_path, config, overwrite=True)
    emit_event(
        event="reserve_registered",
        stage="reserve",
        case_id=args.case_id,
        status="offline",
    )
    return 0


def command_budget(args: argparse.Namespace) -> int:
    snapshot = BudgetLedger(ensure_private_path(args.ledger)).snapshot()
    print(json.dumps(snapshot, separators=(",", ":"), sort_keys=True))
    return 0


def command_reconcile(args: argparse.Namespace) -> int:
    ledger = BudgetLedger(ensure_private_path(args.ledger))
    runs = read_jsonl(ensure_private_path(args.runs), allow_missing=False)
    settled = ledger.reconcile(runs)
    emit_event(event="ledger_reconciled", status="offline", reason_code=f"settled_{settled}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Versioned private live-eval runner; dry-run unless explicitly unlocked."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    init = subparsers.add_parser("init", help="Create private live config and cases templates.")
    init.add_argument("--manifest", type=Path, required=True)
    init.add_argument("--config", type=Path, required=True)
    init.add_argument("--cases", type=Path, required=True)
    init.add_argument("--force", action="store_true")
    init.set_defaults(handler=command_init)

    plan = subparsers.add_parser("plan", help="Create a private randomized execution plan.")
    add_live_inputs(plan)
    plan.add_argument("--stage", choices=STAGES, required=True)
    plan.add_argument(
        "--runs", type=Path, default=Path(".run/alpha-context-proof/runs.jsonl")
    )
    plan.add_argument("--output", type=Path, required=True)
    plan.add_argument("--seed", help="Hex seed for reproducible offline tests only.")
    plan.add_argument("--force", action="store_true")
    plan.set_defaults(handler=command_plan)

    run = subparsers.add_parser("run", help="Preview one job or explicitly execute it live.")
    add_live_inputs(run)
    run.add_argument("--plan", type=Path, required=True)
    run.add_argument("--job-id", required=True)
    run.add_argument("--runs", type=Path, required=True)
    run.add_argument("--ledger", type=Path, required=True)
    run.add_argument("--events", type=Path, required=True)
    run.add_argument("--raw-dir", type=Path, required=True)
    run.add_argument("--blind-dir", type=Path, required=True)
    run.add_argument("--execute", action="store_true")
    run.add_argument("--confirm", default="")
    run.set_defaults(handler=command_run)

    freeze = subparsers.add_parser(
        "freeze-model", help="Freeze the cheapest calibrated model that passes the gates."
    )
    freeze.add_argument("--manifest", type=Path, required=True)
    freeze.add_argument("--config", type=Path, required=True)
    freeze.add_argument("--cases", type=Path, required=True)
    freeze.add_argument("--runs", type=Path, required=True)
    freeze.set_defaults(handler=command_freeze)

    reserve = subparsers.add_parser(
        "register-reserve", help="Record a triggered, pre-registered instability rule."
    )
    reserve.add_argument("--manifest", type=Path, required=True)
    reserve.add_argument("--config", type=Path, required=True)
    reserve.add_argument("--cases", type=Path, required=True)
    reserve.add_argument("--runs", type=Path, required=True)
    reserve.add_argument("--case-id", required=True)
    reserve.add_argument(
        "--rule",
        choices=("quality_metrics_vary", "predicted_labels_vary"),
        required=True,
    )
    reserve.set_defaults(handler=command_register_reserve)

    budget = subparsers.add_parser("budget", help="Show conservative settled + pending charges.")
    budget.add_argument("--ledger", type=Path, required=True)
    budget.set_defaults(handler=command_budget)

    reconcile = subparsers.add_parser(
        "reconcile-ledger", help="Settle crash-left reservations from persisted run metrics."
    )
    reconcile.add_argument("--ledger", type=Path, required=True)
    reconcile.add_argument("--runs", type=Path, required=True)
    reconcile.set_defaults(handler=command_reconcile)
    return parser


def add_live_inputs(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--cases", type=Path, required=True)


def main() -> int:
    args = build_parser().parse_args()
    try:
        return int(args.handler(args))
    except LiveEvalError as error:
        print(f"Live eval blocked: {error}", file=sys.stderr)
        return 2
    except Exception:
        print("Live eval blocked: internal_failure", file=sys.stderr)
        return 5


if __name__ == "__main__":
    raise SystemExit(main())
