from __future__ import annotations

import argparse
import concurrent.futures
import contextlib
import importlib.util
import io
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import multiprocessing
import os
import signal
import tempfile
import threading
import time
import traceback
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
                "expected_label": ("contradiction", "compatible", "ambiguous")[
                    (index - 1) % 3
                ],
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
            "campaign_hash": "a" * 64,
            "execution_config_hash": "a" * 64,
            "private_cases_hash": "a" * 64,
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
        "reserve_case_ids": [],
        "reserve_justifications": {},
        "instability_rules": ["quality_metrics_vary", "predicted_labels_vary"],
        "max_output_tokens": 2_000,
        "request_overhead_tokens": 1_024,
        "timeout_seconds": 90,
        "frozen_contract": frozen,
    }


def private_cases(source_campaign: dict) -> dict:
    handoffs = {}
    for index, task in enumerate(source_campaign["handoff_tasks"], start=1):
        versions = [str(uuid.UUID(int=1000 + index * 10 + offset)) for offset in (1, 2)]
        entries = [str(uuid.UUID(int=2000 + index * 10 + offset)) for offset in (1, 2)]
        knowledge = [
            {
                "knowledge_public_id": entries[offset],
                "version_public_id": versions[offset],
                "version_number": 1,
                "entry_type": "requirement",
                "title": "Décision confirmée",
                "statement": (
                    "Conserver les décisions."
                    if offset == 0
                    else "Préférence hors sujet."
                ),
                "rationale": "Fixture synthétique",
                "node_key": "product",
            }
            for offset in range(2)
        ]
        content = {
            "objective": "Préparer le produit",
            "project_summary": "Contexte français",
            "graph_version": 3,
            "contract": {},
            "knowledge": knowledge[:1],
            "provenance": [
                {
                    "knowledge_public_id": entries[0],
                    "version_public_id": versions[0],
                    "reason_code": "contract_required",
                    "explanation": "Source du contrat",
                }
            ],
        }
        pack_hash = live.sha256_json(content)
        pack = {
            "public_id": str(uuid.UUID(int=3000 + index)),
            "version": 1,
            "status": "current",
            "source_graph_version": 3,
            "compiler_version": "alpha-context-compiler-v1",
            "selection_mode": "deterministic",
            "content_hash": pack_hash,
            "token_budget": 12000,
            "token_count": 100,
            "compiled_at": "2026-09-05T00:00:00Z",
            "invalidated_at": None,
            "stale_reason": None,
            "content": content,
            "selection_items": [
                {
                    "candidate_public_id": version,
                    "decision": "included" if offset == 0 else "excluded",
                    "reason_code": (
                        "contract_required" if offset == 0 else "out_of_scope"
                    ),
                    "explanation": "Annotation synthétique",
                    "rank": offset,
                    "estimated_tokens": 50,
                    "is_mandatory": offset == 0,
                }
                for offset, version in enumerate(versions)
            ],
        }
        snapshot = {
            "project": {
                "public_id": str(uuid.UUID(int=4000 + index)),
                "graph_version": 3,
                "objective": content["objective"],
                "summary": content["project_summary"],
            },
            "knowledge": [
                {
                    "public_id": row["knowledge_public_id"],
                    "created_at": "2026-09-05T00:00:00Z",
                    **{
                        key: value
                        for key, value in row.items()
                        if key != "knowledge_public_id"
                    },
                }
                for row in knowledge
            ],
        }
        handoffs[task["task_id"]] = {
            "task_text": f"Prepare handoff {task['task_id']}",
            "project_ref": task["project_ref"],
            "annotations": {
                "reviewer_ref": "synthetic-reviewer",
                "reviewed_at": "2026-09-05T00:00:00Z",
                "facts": {
                    task["critical_fact_ids"][0]: {"source_version_ids": versions[:1]}
                },
                "items": {
                    task["relevant_item_ids"][0]: versions[0],
                    task["irrelevant_item_ids"][0]: versions[1],
                },
            },
            "conditions": {
                "context_pack": {
                    "payload": pack,
                    "allowed_source_ids": versions[:1],
                    "context_pack_hash": pack_hash,
                },
                "full_dump": {
                    "payload": snapshot,
                    "allowed_source_ids": versions,
                    "context_pack_hash": None,
                },
            },
        }
    return {
        "schema_version": "1.0",
        "campaign_id": source_campaign["campaign_id"],
        "handoffs": handoffs,
        "contradictions": {
            pair["pair_id"]: {
                "left": {
                    "version_id": pair["left_version_ref"],
                    "text": "Keep forever",
                },
                "right": {
                    "version_id": pair["right_version_ref"],
                    "text": "Delete after 30 days",
                },
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
        source_config = config(selected=False)
        cases = private_cases(source_campaign)
        plan = live.build_plan(
            source_campaign, source_config, cases, "calibration", "abcd"
        )
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


class LocalResponsesFixture:
    """Real HTTP framing on loopback, containing only synthetic test data."""

    def __init__(self, status=200, body=b'{"ok":true}', redirect=None, delay=0, length=None):
        self._requests = []
        self.receiver, self.sender = multiprocessing.get_context("fork").Pipe(duplex=False)
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_POST(self):
                request_body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
                fixture.sender.send((self.command, self.path, dict(self.headers), request_body))
                self.reply()

            def do_GET(self):
                fixture.sender.send((self.command, self.path, dict(self.headers), b""))
                self.reply()

            def reply(self):
                self.send_response(status)
                if redirect:
                    self.send_header("Location", redirect)
                if length is not False:
                    self.send_header("Content-Length", str(len(body) if length is None else length))
                self.end_headers()
                try:
                    if delay:
                        for byte in body:
                            self.wfile.write(bytes([byte]))
                            self.wfile.flush()
                            time.sleep(delay)
                    else:
                        self.wfile.write(body)
                except (BrokenPipeError, ConnectionResetError):
                    pass  # The client deliberately closes rejected responses.

        self.server = HTTPServer(("127.0.0.1", 0), Handler)
        self.process = multiprocessing.get_context("fork").Process(target=self.server.serve_forever)
        self.url = f"http://127.0.0.1:{self.server.server_port}/v1/responses"

    def __enter__(self):
        self.process.start()
        self.sender.close()
        return self

    @property
    def requests(self):
        while self.receiver.poll():
            try:
                self._requests.append(self.receiver.recv())
            except EOFError:
                break
        return self._requests

    def __exit__(self, *_args):
        if self.process.is_alive():
            self.process.kill()
        self.process.join()
        self.process.close()
        self.server.server_close()
        self.receiver.close()


class FakeProviderResponse(io.BytesIO):
    def getheader(self, _name):
        return None


class ResponsesTransportTests(unittest.TestCase):
    def test_only_the_fixed_official_origin_receives_the_request(self):
        def opener(request, *, timeout):
            return FakeProviderResponse(json.dumps({
                "url": request.full_url,
                "method": request.get_method(),
                "body": json.loads(request.data),
                "timeout": timeout,
            }).encode())

        body = {"input": "synthetic", "base_url": "https://untrusted.invalid"}
        with mock.patch.dict(os.environ, {"OPENAI_BASE_URL": "https://untrusted.invalid"}, clear=True):
            result = live.ResponsesClient(opener).create(body, "fixture-only-key", 2)
        self.assertEqual(result, {
            "url": "https://api.openai.com/v1/responses",
            "method": "POST",
            "body": body,
            "timeout": 2,
        })

    def test_inherited_proxy_is_not_used(self):
        with LocalResponsesFixture() as proxy, LocalResponsesFixture() as source:
            with (
                mock.patch.object(live, "RESPONSES_API_URL", source.url),
                mock.patch.dict(os.environ, {
                    "http_proxy": proxy.url, "https_proxy": proxy.url, "no_proxy": "",
                }, clear=True),
            ):
                self.assertEqual(live.ResponsesClient().create({}, "fixture-only-key", 2), {"ok": True})
            self.assertEqual(len(source.requests), 1)
            self.assertEqual(proxy.requests, [])

    def test_valid_response_at_the_size_boundary_is_preserved(self):
        body = b'{"result":"' + b'x' * (live.MAX_RESPONSE_BYTES - 13) + b'"}'
        self.assertEqual(len(body), live.MAX_RESPONSE_BYTES)
        with LocalResponsesFixture(body=body) as source:
            with mock.patch.object(live, "RESPONSES_API_URL", source.url):
                result = live.ResponsesClient().create({"input": "synthetic"}, "fixture-only-key", 2)
            self.assertEqual(result, json.loads(body))

    def test_unknown_length_still_has_a_response_size_limit(self):
        with LocalResponsesFixture(body=b'x' * (live.MAX_RESPONSE_BYTES + 1), length=False) as source:
            with mock.patch.object(live, "RESPONSES_API_URL", source.url):
                with self.assertRaisesRegex(live.LiveEvalError, "^provider_response_too_large$"):
                    live.ResponsesClient().create({}, "fixture-only-key", 2)

    def test_invalid_json_and_non_object_response_have_safe_errors(self):
        for body, reason in (
            (b'{"private":"synthetic-broken-body"', "provider_unreadable_response"),
            (b'{"private":"\xff"}', "provider_unreadable_response"),
            (b'[]', "provider_invalid_response_shape"),
            (b'null', "provider_invalid_response_shape"),
        ):
            with self.subTest(body=body):
                with self.assertRaisesRegex(live.LiveEvalError, f"^{reason}$"):
                    live.ResponsesClient(lambda *_a, **_kw: FakeProviderResponse(body)).create(
                        {}, "fixture-only-key", 2
                    )

    def test_http_failures_never_expose_provider_body_url_or_key(self):
        for status in (401, 403, 429, 500, 503):
            with self.subTest(status=status), LocalResponsesFixture(
                status=status, body=b'fixture-only-key https://private.invalid synthetic-error-body'
            ) as source:
                with mock.patch.object(live, "RESPONSES_API_URL", source.url):
                    api_key = "fixture-only-key"
                    try:
                        live.ResponsesClient().create({}, api_key, 2)
                    except live.LiveEvalError as error:
                        self.assertEqual(str(error), f"provider_http_{status}")
                        rendered = "".join(traceback.format_exception(error))
                        for private in ("fixture-only-key", "private.invalid", "synthetic-error-body", source.url):
                            self.assertNotIn(private, rendered)
                    else:
                        self.fail("an HTTP failure was accepted")

    def test_worker_is_reaped_on_success_error_timeout_and_abrupt_exit(self):
        for outcome in ("success", "error", "timeout", "exit"):
            with self.subTest(outcome=outcome):
                worker_pid = multiprocessing.RawValue("i", 0)

                def opener(*_args, **_kwargs):
                    worker_pid.value = os.getpid()
                    if outcome == "error":
                        raise OSError("fixture-only-key https://private.invalid synthetic-error-body")
                    if outcome == "timeout":
                        time.sleep(5)
                    if outcome == "exit":
                        os._exit(0)
                    return FakeProviderResponse(b'{"ok":true}')

                started = time.monotonic()
                if outcome == "success":
                    self.assertEqual(live.ResponsesClient(opener).create({}, "fixture-only-key", 0.2), {"ok": True})
                else:
                    expected = "provider_timeout" if outcome == "timeout" else "provider_transport_failure"
                    with self.assertRaisesRegex(live.LiveEvalError, f"^{expected}$"):
                        live.ResponsesClient(opener).create({}, "fixture-only-key", 0.2)
                self.assertLess(time.monotonic() - started, 0.8)
                self.assertGreater(worker_pid.value, 0)
                with self.assertRaises(ChildProcessError):
                    os.waitpid(worker_pid.value, os.WNOHANG)

    def test_cancelling_the_call_reaps_the_transport_worker(self):
        worker_pid = multiprocessing.RawValue("i", 0)
        caller_pid = os.getpid()

        def opener(*_args, **_kwargs):
            worker_pid.value = os.getpid()
            time.sleep(5)
            return FakeProviderResponse(b'{"ok":true}')

        def cancel_when_worker_starts():
            deadline = time.monotonic() + 1
            while not worker_pid.value and time.monotonic() < deadline:
                time.sleep(0.01)
            if worker_pid.value:
                os.kill(caller_pid, signal.SIGINT)

        cancellation = multiprocessing.get_context("fork").Process(target=cancel_when_worker_starts)
        cancellation.start()
        try:
            with self.assertRaises(KeyboardInterrupt):
                live.ResponsesClient(opener).create({}, "fixture-only-key", 2)
        finally:
            if cancellation.is_alive():
                cancellation.kill()
            cancellation.join()
            cancellation.close()
        self.assertGreater(worker_pid.value, 0)
        with self.assertRaises(ChildProcessError):
            os.waitpid(worker_pid.value, os.WNOHANG)

    def test_transport_setup_failures_are_sanitized(self):
        context = multiprocessing.get_context("fork")
        for factory in ("Pipe", "Process"):
            with self.subTest(factory=factory), mock.patch.object(
                context, factory, side_effect=OSError("synthetic-private-setup-detail")
            ):
                with self.assertRaisesRegex(live.LiveEvalError, "^provider_transport_failure$"):
                    live.ResponsesClient().create({}, "fixture-only-key", 2)
        with mock.patch.object(
            multiprocessing.process.BaseProcess,
            "start", side_effect=RuntimeError("synthetic-private-setup-detail"),
        ):
            with self.assertRaisesRegex(live.LiveEvalError, "^provider_transport_failure$"):
                live.ResponsesClient().create({}, "fixture-only-key", 2)

    def test_bad_request_and_unsupported_runtime_fail_before_transport(self):
        unused_opener = mock.Mock(side_effect=AssertionError("must not send"))
        client = live.ResponsesClient(unused_opener)
        for timeout in (True, 0, -1, 301, float("inf"), float("nan"), "2"):
            with self.subTest(timeout=timeout):
                with self.assertRaisesRegex(live.LiveEvalError, "^provider_invalid_request$"):
                    client.create({}, "fixture-only-key", timeout)
        with self.assertRaisesRegex(live.LiveEvalError, "^provider_invalid_request$"):
            client.create({}, "fixture\nkey", 2)
        with self.assertRaisesRegex(live.LiveEvalError, "^provider_request_too_large$"):
            client.create({"input": "x" * live.MAX_REQUEST_BYTES}, "fixture-only-key", 2)
        for patch in (
            mock.patch.object(live.multiprocessing, "get_all_start_methods", return_value=["spawn"]),
            mock.patch.object(live.threading, "active_count", return_value=2),
        ):
            with patch, self.assertRaisesRegex(live.LiveEvalError, "^provider_transport_unavailable$"):
                client.create({}, "fixture-only-key", 2)
        unused_opener.assert_not_called()

    def test_interrupted_body_is_refused_even_when_prefix_is_valid_json(self):
        with LocalResponsesFixture(body=b'{"ok":true}', length=100) as source:
            with mock.patch.object(live, "RESPONSES_API_URL", source.url):
                with self.assertRaisesRegex(live.LiveEvalError, "^provider_transport_failure$"):
                    live.ResponsesClient().create({"input": "synthetic"}, "fixture-only-key", 2)

    def test_slow_progress_does_not_extend_the_total_response_deadline(self):
        with LocalResponsesFixture(body=b'{"result":"slow-but-continuous"}', delay=0.04) as source:
            with mock.patch.object(live, "RESPONSES_API_URL", source.url):
                started = time.monotonic()
                with self.assertRaisesRegex(live.LiveEvalError, "^provider_timeout$"):
                    live.ResponsesClient().create({"input": "synthetic"}, "fixture-only-key", 0.2)
                self.assertLess(time.monotonic() - started, 0.8)
            self.assertEqual(len(source.requests), 1)

    def test_excessive_response_is_rejected_without_returning_the_body(self):
        body = json.dumps({"private": "x" * (2 * 1024 * 1024)}).encode()
        with LocalResponsesFixture(body=body) as source:
            with mock.patch.object(live, "RESPONSES_API_URL", source.url):
                with self.assertRaisesRegex(live.LiveEvalError, "^provider_response_too_large$"):
                    live.ResponsesClient().create({"input": "synthetic"}, "fixture-only-key", 2)
            self.assertEqual(len(source.requests), 1)

    def test_redirect_cannot_send_a_second_request_or_credential_to_another_origin(self):
        for status in (301, 302, 303, 307, 308):
            with self.subTest(status=status), LocalResponsesFixture() as target, LocalResponsesFixture(
                status=status, redirect=target.url
            ) as source:
                with mock.patch.object(live, "RESPONSES_API_URL", source.url):
                    error = None
                    try:
                        live.ResponsesClient().create({"input": "synthetic"}, "fixture-only-key", 2)
                    except live.LiveEvalError as cause:
                        error = cause
                self.assertEqual(len(source.requests), 1)
                self.assertEqual(target.requests, [], "a redirect forwarded the API credential")
                self.assertIsNotNone(error)
                self.assertEqual(str(error), "provider_redirect_refused")


if __name__ == "__main__":
    unittest.main()
