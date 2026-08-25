from __future__ import annotations

import argparse
import concurrent.futures
import contextlib
import importlib.util
import io
import json
import tempfile
import threading
import unittest
import uuid
from decimal import Decimal
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "alpha-live-eval.py"
SPEC = importlib.util.spec_from_file_location("alpha_live_eval", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
live = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(live)

AGGREGATOR_SCRIPT = Path(__file__).resolve().parents[1] / "alpha-eval.py"
AGGREGATOR_SPEC = importlib.util.spec_from_file_location("alpha_eval", AGGREGATOR_SCRIPT)
assert AGGREGATOR_SPEC is not None and AGGREGATOR_SPEC.loader is not None
aggregate = importlib.util.module_from_spec(AGGREGATOR_SPEC)
AGGREGATOR_SPEC.loader.exec_module(aggregate)


def project_ref(index: int) -> str:
    return f"project-{index:02d}"


def campaign(task_count: int = 2, pair_count: int = 1) -> dict:
    tasks = []
    for index in range(1, task_count + 1):
        tasks.append(
            {
                "task_id": f"handoff-{index:02d}",
                "project_ref": project_ref(((index - 1) % 3) + 1),
                "critical_fact_ids": [f"fact-{index}"],
                "relevant_item_ids": [f"relevant-{index}"],
                "irrelevant_item_ids": [f"irrelevant-{index}"],
            }
        )
    pairs = []
    for index in range(1, pair_count + 1):
        pairs.append(
            {
                "pair_id": f"pair-{index:03d}",
                "project_ref": project_ref(((index - 1) % 3) + 1),
                "left_version_ref": str(uuid.UUID(int=index * 2 - 1)),
                "right_version_ref": str(uuid.UUID(int=index * 2)),
                "expected_label": "contradiction",
            }
        )
    return {
        "schema_version": "1.0",
        "campaign_id": "alpha-context-proof-test",
        "budget_usd": {"calibration": 10, "main": 70, "reserve": 20, "total": 100},
        "privacy": {
            "contains_secrets": False,
            "contains_personal_data": False,
            "contains_sensitive_data": False,
        },
        "projects": [
            {
                "project_ref": project_ref(index),
                "corpus_role": "ai_center" if index == 1 else "comparison",
                "corpus_fingerprint": f"{index:064x}",
                "consent_confirmed": True,
            }
            for index in range(1, 4)
        ],
        "handoff_tasks": tasks,
        "contradiction_pairs": pairs,
    }


def config(selected: bool = True) -> dict:
    selected_model = "model-a" if selected else None
    frozen = (
        {
            "selected_model": "model-a",
            "runner_version": live.RUNNER_VERSION,
            "prompt_version": live.PROMPT_VERSION,
            "output_schema_version": live.OUTPUT_SCHEMA_VERSION,
            "calibration_runs_hash": "a" * 64,
            "frozen_at": "2026-08-25T00:00:00Z",
        }
        if selected
        else None
    )
    return {
        "schema_version": "1.0",
        "campaign_id": "alpha-context-proof-test",
        "pricing_version": "2026-08-25",
        "pricing_verified_at": "2026-08-25T00:00:00Z",
        "models": [
            {
                "id": "model-a",
                "input_usd_per_million": "1.00",
                "output_usd_per_million": "4.00",
            },
            {
                "id": "model-b",
                "input_usd_per_million": "2.00",
                "output_usd_per_million": "8.00",
            },
        ],
        "selected_model": selected_model,
        "calibration_case_ids": ["handoff-01", "pair-001"],
        "reserve_case_ids": ["handoff-01"],
        "max_output_tokens": 2_000,
        "request_overhead_tokens": 1_024,
        "timeout_seconds": 90,
        "frozen_contract": frozen,
    }


def private_cases(source_campaign: dict) -> dict:
    return {
        "schema_version": "1.0",
        "campaign_id": source_campaign["campaign_id"],
        "handoffs": {
            task["task_id"]: {
                "task_text": f"Prepare handoff {task['task_id']}",
                "conditions": {
                    "context_pack": {
                        "payload": {"knowledge": [task["critical_fact_ids"][0]]},
                        "allowed_source_ids": [str(uuid.UUID(int=1000 + index))],
                        "context_pack_hash": f"{index + 100:064x}",
                    },
                    "full_dump": {
                        "payload": {"knowledge": ["all"]},
                        "allowed_source_ids": [str(uuid.UUID(int=2000 + index))],
                        "context_pack_hash": None,
                    },
                },
            }
            for index, task in enumerate(source_campaign["handoff_tasks"], start=1)
        },
        "contradictions": {
            pair["pair_id"]: {
                "left": {"version_id": pair["left_version_ref"], "text": "Keep forever"},
                "right": {"version_id": pair["right_version_ref"], "text": "Delete after 30 days"},
            }
            for pair in source_campaign["contradiction_pairs"]
        },
    }


class LiveEvalTests(unittest.TestCase):
    def test_calibration_rejects_more_than_two_models(self) -> None:
        invalid = config()
        invalid["models"].append(
            {
                "id": "model-c",
                "input_usd_per_million": "1",
                "output_usd_per_million": "1",
            }
        )
        with self.assertRaisesRegex(live.LiveEvalError, "one or two"):
            live.model_index(invalid)

    def test_main_plan_has_three_randomized_blind_repetitions(self) -> None:
        source_campaign = campaign()
        source_config = config()
        cases = private_cases(source_campaign)
        plan = live.build_plan(source_campaign, source_config, cases, "main", "1234abcd")

        handoff_jobs = [job for job in plan["jobs"] if job["case_kind"] == "handoff"]
        self.assertEqual(len(handoff_jobs), 2 * 2 * 3)
        for task in source_campaign["handoff_tasks"]:
            for repetition in (1, 2, 3):
                comparison = [
                    job
                    for job in handoff_jobs
                    if job["case_id"] == task["task_id"] and job["repetition"] == repetition
                ]
                self.assertEqual({job["condition"] for job in comparison}, set(live.CONDITIONS))
                self.assertEqual({job["blind_label"] for job in comparison}, {"A", "B"})
        self.assertEqual(plan["randomization_commitment"], live.hashlib.sha256(b"1234abcd").hexdigest())
        live.validate_plan(plan, source_campaign, source_config, cases)
        plan["jobs"][0]["condition"] = (
            "context_pack" if plan["jobs"][0]["condition"] == "full_dump" else "full_dump"
        )
        with self.assertRaisesRegex(live.LiveEvalError, "modified"):
            live.validate_plan(plan, source_campaign, source_config, cases)

    def test_request_is_stateless_and_uses_strict_structured_output(self) -> None:
        source_campaign = campaign()
        source_config = config()
        cases = private_cases(source_campaign)
        plan = live.build_plan(source_campaign, source_config, cases, "main", "abcd")
        job = next(job for job in plan["jobs"] if job["case_kind"] == "handoff")
        body, _ = live.request_for_job(job, source_campaign, source_config, cases)

        self.assertIs(body["store"], False)
        self.assertEqual(body["text"]["format"]["type"], "json_schema")
        self.assertIs(body["text"]["format"]["strict"], True)
        self.assertNotIn("api_key", json.dumps(body).lower())
        token_upper, maximum = live.request_upper_bound_cost(
            body, source_config, live.model_index(source_config)[job["model"]]
        )
        self.assertGreater(token_upper, len(live.canonical_json(body)))
        self.assertGreater(maximum, Decimal("0"))

    def test_dry_run_never_reads_a_key_or_opens_the_network(self) -> None:
        source_campaign = campaign(task_count=12, pair_count=60)
        source_config = config()
        cases = private_cases(source_campaign)
        plan = live.build_plan(source_campaign, source_config, cases, "main", "abcd")
        job = plan["jobs"][0]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = {
                "manifest": root / "campaign.json",
                "config": root / "live-config.json",
                "cases": root / "private-cases.json",
                "plan": root / "plan.json",
            }
            for key, value in (
                ("manifest", source_campaign),
                ("config", source_config),
                ("cases", cases),
                ("plan", plan),
            ):
                live.write_json_private(paths[key], value, overwrite=False)
            args = argparse.Namespace(
                manifest=paths["manifest"],
                config=paths["config"],
                cases=paths["cases"],
                plan=paths["plan"],
                job_id=job["job_id"],
                runs=root / "runs.jsonl",
                ledger=root / "ledger.jsonl",
                events=root / "events.jsonl",
                raw_dir=root / "raw",
                blind_dir=root / "blind",
                execute=False,
                confirm="",
            )
            with (
                mock.patch.object(live, "default_private_root", return_value=root),
                mock.patch.dict(live.os.environ, {}, clear=True),
                mock.patch.object(
                    live.ResponsesClient,
                    "create",
                    side_effect=AssertionError("network path must not be reached"),
                ),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                self.assertEqual(live.command_run(args), 0)

    def test_budget_ledger_uses_pending_reservations_and_hard_stage_stop(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            ledger = live.BudgetLedger(Path(directory) / "ledger.jsonl")
            ledger.reserve("one", "calibration", Decimal("9"), "run-one")
            with self.assertRaisesRegex(live.LiveEvalError, "hard budget stop"):
                ledger.reserve("two", "calibration", Decimal("2"), "run-two")
            ledger.settle("one", Decimal("2"))
            ledger.reserve("two", "calibration", Decimal("8"), "run-two")
            snapshot = ledger.snapshot()
            self.assertEqual(snapshot["total_charged_usd"], 10.0)
            self.assertEqual(snapshot["pending_reservations"], 1)
            self.assertEqual(
                ledger.reconcile([{"run_id": "run-two", "cost_usd": 1.5}]),
                1,
            )
            reconciled = ledger.snapshot()
            self.assertEqual(reconciled["total_charged_usd"], 3.5)
            self.assertEqual(reconciled["pending_reservations"], 0)

    def test_budget_reservations_are_serialized_between_workers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            ledger = live.BudgetLedger(Path(directory) / "ledger.jsonl")
            barrier = threading.Barrier(2)

            def reserve(index: int) -> str:
                barrier.wait()
                try:
                    ledger.reserve(
                        f"reservation-{index}",
                        "calibration",
                        Decimal("6"),
                        f"run-{index}",
                    )
                except live.LiveEvalError:
                    return "blocked"
                return "allowed"

            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                results = list(executor.map(reserve, (1, 2)))
            self.assertEqual(sorted(results), ["allowed", "blocked"])
            self.assertEqual(ledger.snapshot()["total_charged_usd"], 6.0)

    def test_safe_events_drop_payloads_and_credentials(self) -> None:
        event = live.safe_event(
            event="request_failed",
            run_id="run-safe",
            status="blocked",
            api_key="sk-should-never-appear",
            provider_body={"private": "payload"},
        )
        serialized = json.dumps(event)
        self.assertNotIn("sk-", serialized)
        self.assertNotIn("payload", serialized)
        self.assertEqual(event["run_id"], "run-safe")

    def test_expurgated_live_metrics_remain_compatible_with_aggregator(self) -> None:
        source_campaign = campaign()
        record = {
            "schema_version": "1.0",
            "run_id": "run-live-test",
            "case_kind": "handoff",
            "case_id": "handoff-01",
            "project_ref": "project-01",
            "stage": "calibration",
            "condition": "context_pack",
            "repetition": 1,
            "model": "model-a",
            "prompt_version": live.PROMPT_VERSION,
            "output_schema_version": live.OUTPUT_SCHEMA_VERSION,
            "context_pack_hash": "a" * 64,
            "input_tokens": 100,
            "output_tokens": 25,
            "latency_ms": 50,
            "cost_usd": 0.01,
            "structured_output_valid": True,
            "source_ids_valid": True,
            "critical_facts_expected": 1,
            "critical_facts_present": 1,
            "relevant_items_expected": 1,
            "relevant_items_present": 1,
            "selected_items_total": 1,
            "irrelevant_items_selected": 0,
            "attempts": 1,
            "provider_status": "completed",
            "provider_response_ref": "opaque-response",
            "request_hash": "b" * 64,
        }
        runs = aggregate.validate_runs([record], source_campaign)
        self.assertIn("run-live-test", runs)

    def test_calibration_recommends_the_cheapest_complete_passing_model(self) -> None:
        source_config = config(selected=False)
        records = []
        for model_id, cost in (("model-a", 0.2), ("model-b", 0.1)):
            records.extend(
                [
                    {
                        "stage": "calibration",
                        "model": model_id,
                        "case_kind": "handoff",
                        "case_id": "handoff-01",
                        "condition": "context_pack",
                        "repetition": 1,
                        "structured_output_valid": True,
                        "source_ids_valid": True,
                        "critical_facts_expected": 1,
                        "critical_facts_present": 1,
                        "relevant_items_expected": 1,
                        "relevant_items_present": 1,
                        "cost_usd": cost,
                    },
                    {
                        "stage": "calibration",
                        "model": model_id,
                        "case_kind": "handoff",
                        "case_id": "handoff-01",
                        "condition": "full_dump",
                        "repetition": 1,
                        "structured_output_valid": True,
                        "source_ids_valid": True,
                        "critical_facts_expected": 1,
                        "critical_facts_present": 1,
                        "relevant_items_expected": 1,
                        "relevant_items_present": 1,
                        "cost_usd": cost,
                    },
                    {
                        "stage": "calibration",
                        "model": model_id,
                        "case_kind": "contradiction",
                        "case_id": "pair-001",
                        "condition": "context_pack",
                        "repetition": 1,
                        "structured_output_valid": True,
                        "source_ids_valid": True,
                        "expected_label": "contradiction",
                        "predicted_label": "contradiction",
                        "cost_usd": cost,
                    },
                ]
            )
        summary = live.calibration_summary(source_config, records)
        self.assertEqual(summary["recommended_model"], "model-b")

        for record in records:
            if record["model"] == "model-b" and record["case_kind"] == "handoff":
                record["relevant_items_present"] = 0
        summary = live.calibration_summary(source_config, records)
        self.assertEqual(summary["recommended_model"], "model-a")
        rejected = next(item for item in summary["models"] if item["model"] == "model-b")
        self.assertEqual(rejected["relevant_context_recall"], 0.0)
        self.assertFalse(rejected["passed"])


if __name__ == "__main__":
    unittest.main()
