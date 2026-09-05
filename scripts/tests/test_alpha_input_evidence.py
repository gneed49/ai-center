from __future__ import annotations

import copy
import json
import unittest

from test_alpha_live_eval import campaign, config, live, private_cases
from test_alpha_evaluation_protocol import fixture, records_for_plan


class InputEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.campaign = campaign()
        self.cases = private_cases(self.campaign)
        self.case = self.cases["handoffs"]["handoff-01"]
        self.task = self.campaign["handoff_tasks"][0]

    def test_captured_export_hash_handles_french_and_object_key_order(self) -> None:
        live.validate_private_cases(self.cases, self.campaign)
        reordered = json.loads(
            json.dumps(self.cases, ensure_ascii=False, sort_keys=True)
        )
        live.validate_private_cases(reordered, self.campaign)
        self.assertEqual(
            live.validated_handoff_inputs(self.case, self.task),
            live.validated_handoff_inputs(
                reordered["handoffs"]["handoff-01"], self.task
            ),
        )

    def test_hash_graph_ledger_source_and_allowlist_tampering_are_refused(self) -> None:
        def bad_hash(case):
            case["conditions"]["context_pack"]["context_pack_hash"] = "0" * 64

        def bad_graph(case):
            case["conditions"]["context_pack"]["payload"]["source_graph_version"] += 1

        def bad_ledger(case):
            case["conditions"]["context_pack"]["payload"]["selection_items"][0][
                "decision"
            ] = "excluded"

        def bad_source(case):
            pack = case["conditions"]["context_pack"]
            pack["payload"]["content"]["knowledge"][0][
                "statement"
            ] = "An invented statement with a real UUID"
            pack["payload"]["content_hash"] = pack["context_pack_hash"] = (
                live.sha256_json(pack["payload"]["content"])
            )

        def bad_allowlist(case):
            case["conditions"]["context_pack"]["allowed_source_ids"] = case[
                "conditions"
            ]["full_dump"]["allowed_source_ids"]

        def missing_export_metadata(case):
            del case["conditions"]["context_pack"]["payload"]["compiled_at"]

        for mutate in (
            bad_hash,
            bad_graph,
            bad_ledger,
            bad_source,
            bad_allowlist,
            missing_export_metadata,
        ):
            with self.subTest(mutate=mutate.__name__):
                case = copy.deepcopy(self.case)
                mutate(case)
                with self.assertRaises(live.LiveEvalError):
                    live.validated_handoff_inputs(case, self.task)

    def test_empty_input_and_blank_deliverable_cannot_claim_perfect_metrics(
        self,
    ) -> None:
        case = copy.deepcopy(self.case)
        pack = case["conditions"]["context_pack"]
        pack["payload"]["content"]["knowledge"] = []
        pack["payload"]["content_hash"] = pack["context_pack_hash"] = live.sha256_json(
            pack["payload"]["content"]
        )
        with self.assertRaises(live.LiveEvalError):
            live.validated_handoff_inputs(case, self.task)
        blank = {
            "source_version_ids": self.case["conditions"]["context_pack"][
                "allowed_source_ids"
            ],
            "deliverable": {"summary": " ", "actions": [], "risks": []},
        }
        self.assertFalse(live.validate_model_output("handoff", blank))
        blank.update(
            critical_fact_ids_used=self.task["critical_fact_ids"],
            selected_item_ids=self.task["relevant_item_ids"],
        )
        self.assertFalse(live.validate_model_output("handoff", blank))

    def test_counts_follow_actual_selection_and_annotations_stay_out_of_prompt(
        self,
    ) -> None:
        # Human annotation places the critical fact on the excluded version.
        excluded = self.case["conditions"]["full_dump"]["allowed_source_ids"][1]
        self.case["annotations"]["facts"][self.task["critical_fact_ids"][0]][
            "source_version_ids"
        ] = [excluded]
        job = next(
            job
            for job in live.build_plan(
                self.campaign, config(), self.cases, "main", "cafe"
            )["jobs"]
            if job["case_id"] == "handoff-01" and job["condition"] == "context_pack"
        )
        body, scoring = live.request_for_job(job, self.campaign, config(), self.cases)
        self.assertEqual(scoring["critical_facts_present"], 0)
        self.assertEqual(scoring["selected_items_total"], 1)
        self.assertNotIn("synthetic-reviewer", json.dumps(body))
        self.assertNotIn(self.task["critical_fact_ids"][0], json.dumps(body))
        output = {
            "source_version_ids": scoring["allowed_source_ids"],
            "deliverable": {
                "summary": "A paraphrased delivery proposal",
                "actions": ["Review the plan"],
                "risks": [],
            },
        }
        record = live.build_run_record(
            job,
            self.campaign,
            scoring,
            {"status": "completed", "id": "offline"},
            output,
            100,
            10,
            live.Decimal("0.001"),
            1,
            live.sha256_json(body),
        )
        self.assertTrue(record["structured_output_valid"])
        self.assertEqual(record["critical_facts_present"], 0)
        output["source_version_ids"] = [excluded]
        record = live.build_run_record(
            job,
            self.campaign,
            scoring,
            {"status": "completed", "id": "offline"},
            output,
            100,
            10,
            live.Decimal("0.001"),
            1,
            live.sha256_json(body),
        )
        self.assertFalse(record["source_ids_valid"])
        self.assertEqual(record["critical_facts_present"], 0)

    def test_missing_review_and_unannotated_versions_are_refused(self) -> None:
        for field in ("reviewer_ref", "reviewed_at", "facts", "items"):
            case = copy.deepcopy(self.case)
            del case["annotations"][field]
            with self.subTest(field=field), self.assertRaises(live.LiveEvalError):
                live.validated_handoff_inputs(case, self.task)

    def test_contract_anchor_is_shared_by_both_conditions_through_calibration(
        self,
    ) -> None:
        contract = {
            "contract_key": "technical-delivery-plan",
            "required_sections": ["architecture", "implementation", "validation"],
        }
        pack = self.case["conditions"]["context_pack"]
        pack["payload"]["content"]["contract"] = contract
        pack["payload"]["content_hash"] = pack["context_pack_hash"] = live.sha256_json(
            pack["payload"]["content"]
        )
        self.case["annotations"]["facts"][self.task["critical_fact_ids"][0]] = {
            "content_pointer": "/contract",
            "content_hash": live.sha256_json(contract),
        }
        live.validate_private_cases(self.cases, self.campaign)
        inputs = live.validated_handoff_inputs(self.case, self.task)
        for condition, captured in inputs.items():
            with self.subTest(condition=condition):
                self.assertEqual(captured["payload"]["contract"], contract)
                self.assertEqual(captured["critical_facts_expected"], 1)
                self.assertEqual(captured["critical_facts_present"], 1)

        configuration = config(selected=False)
        jobs = live.build_plan(
            self.campaign, configuration, self.cases, "calibration", "cafe"
        )["jobs"]
        handoff_jobs = [job for job in jobs if job["case_id"] == "handoff-01"]
        self.assertEqual(len(handoff_jobs), 4)
        for job in handoff_jobs:
            body, _ = live.request_for_job(job, self.campaign, configuration, self.cases)
            prompt = json.loads(body["input"][0]["content"][0]["text"])
            self.assertEqual(prompt["context"]["contract"], contract)
            self.assertEqual(prompt["task"], self.case["task_text"])

        runs = records_for_plan(self.campaign, configuration, self.cases, "calibration")
        for run in runs:
            if run["case_id"] == "handoff-01":
                self.assertEqual(run["critical_facts_expected"], 1)
                self.assertEqual(run["critical_facts_present"], 1)
        summary = live.validate_calibration_evidence(
            configuration, self.campaign, runs, self.cases
        )
        self.assertEqual(summary["recommended_model"], "model-a")
        for model in summary["models"]:
            self.assertTrue(model["passed"])
            self.assertEqual(model["critical_fact_recall"], 1.0)

    def test_frozen_evidence_recomputes_counts_and_annotation_fingerprint(self) -> None:
        campaign_data, configuration, cases, runs = fixture()
        for field, value in (
            ("critical_facts_present", 0),
            ("selected_items_total", 99),
            ("input_evidence_hash", "f" * 64),
        ):
            changed = copy.deepcopy(runs)
            record = next(
                row
                for row in changed
                if row["stage"] == "main" and row["case_kind"] == "handoff"
            )
            record[field] = value
            with self.subTest(field=field), self.assertRaisesRegex(
                live.LiveEvalError, "input metrics"
            ):
                live.validate_frozen_evidence(
                    configuration, campaign_data, changed, cases
                )
