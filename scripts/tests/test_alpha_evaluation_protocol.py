from __future__ import annotations

import argparse
import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from test_alpha_live_eval import aggregate, campaign, config, live, private_cases


def records_for_plan(
    source_campaign: dict, source_config: dict, cases: dict, stage: str
) -> list[dict]:
    records = []
    for job in live.build_plan(source_campaign, source_config, cases, stage, "aabbcc")[
        "jobs"
    ]:
        body, scoring = live.request_for_job(job, source_campaign, source_config, cases)
        output = {"source_version_ids": scoring["allowed_source_ids"]}
        if job["case_kind"] == "handoff":
            task = next(
                task
                for task in source_campaign["handoff_tasks"]
                if task["task_id"] == job["case_id"]
            )
            output.update(
                critical_fact_ids_used=task["critical_fact_ids"],
                selected_item_ids=task["relevant_item_ids"],
                deliverable={
                    "summary": "Synthetic handoff for offline testing.",
                    "actions": ["Review"],
                    "risks": [],
                },
            )
        else:
            pair = next(
                pair
                for pair in source_campaign["contradiction_pairs"]
                if pair["pair_id"] == job["case_id"]
            )
            output.update(
                predicted_label=pair["expected_label"],
                confidence=1.0,
                explanation="Synthetic classification.",
            )
        record = live.build_run_record(
            job,
            source_campaign,
            scoring,
            {"status": "completed", "id": "synthetic-response"},
            output,
            100 if job["condition"] == "context_pack" else 200,
            20,
            live.Decimal("0.001"),
            100,
            live.sha256_json(body),
        )
        records.append(record)
    return records


def fixture(*, full: bool = False) -> tuple[dict, dict, dict, list[dict]]:
    source_campaign = campaign(task_count=12, pair_count=60) if full else campaign()
    source_config = config(selected=False)
    cases = private_cases(source_campaign)
    calibration = records_for_plan(source_campaign, source_config, cases, "calibration")
    source_config["selected_model"] = "model-a"
    source_config["frozen_contract"] = {
        "selected_model": "model-a",
        "runner_version": live.RUNNER_VERSION,
        "prompt_version": live.PROMPT_VERSION,
        "output_schema_version": live.OUTPUT_SCHEMA_VERSION,
        "calibration_runs_hash": live.runs_hash(calibration),
        "campaign_hash": live.sha256_json(source_campaign),
        "execution_config_hash": live.execution_config_hash(source_config),
        "private_cases_hash": live.sha256_json(cases),
        "frozen_at": "2026-09-05T00:00:00Z",
    }
    return (
        source_campaign,
        source_config,
        cases,
        calibration + records_for_plan(source_campaign, source_config, cases, "main"),
    )


def evaluations_for(runs: list[dict]) -> list[dict]:
    evaluations = []
    for pack in runs:
        if (
            pack["stage"] == "calibration"
            or pack["case_kind"] != "handoff"
            or pack["condition"] != "context_pack"
        ):
            continue
        dump = next(
            row
            for row in runs
            if row["stage"] == pack["stage"]
            and row["case_id"] == pack["case_id"]
            and row["repetition"] == pack["repetition"]
            and row["condition"] == "full_dump"
        )
        for role in ("owner", "secondary"):
            evaluations.append(
                {
                    "schema_version": "1.0",
                    "evaluation_id": f"eval-{pack['run_id']}-{role}",
                    "comparison_id": f"cmp-{pack['case_id']}-{pack['repetition']}-{live.opaque_ref(pack['model'])}",
                    "task_id": pack["case_id"],
                    "repetition": pack["repetition"],
                    "context_pack_run_id": pack["run_id"],
                    "full_dump_run_id": dump["run_id"],
                    "evaluator_ref": f"person-{role}",
                    "evaluator_role": role,
                    "blind_preference": "context_pack",
                    "context_pack_without_major_reformulation": True,
                }
            )
    return evaluations


class EvaluationProtocolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.campaign, self.config, self.cases, self.runs = fixture()

    def test_valid_frozen_evidence_accepts_reordered_calibration(self) -> None:
        live.validate_frozen_evidence(
            self.config, self.campaign, list(reversed(self.runs)), self.cases
        )
        live.validate_reserve_evidence(self.config, self.runs)

    def test_frozen_evidence_rejects_changed_model_prompt_runner_corpus_and_calibration(
        self,
    ) -> None:
        for field, value in (
            ("model", "model-b"),
            ("prompt_version", "other"),
            ("runner_version", "1.0.0"),
            ("output_schema_version", "other"),
        ):
            with self.subTest(field=field):
                runs = copy.deepcopy(self.runs)
                next(row for row in runs if row["stage"] == "main")[field] = value
                with self.assertRaises(live.LiveEvalError):
                    live.validate_frozen_evidence(self.config, self.campaign, runs)
        for field in ("cost_usd", "request_hash"):
            with self.subTest(calibration=field):
                runs = copy.deepcopy(self.runs)
                runs[0][field] = 0.002 if field == "cost_usd" else "f" * 64
                with self.assertRaises(live.LiveEvalError):
                    live.validate_frozen_evidence(self.config, self.campaign, runs)
        for key, value in (("max_output_tokens", 4000), ("instability_rules", [])):
            changed = copy.deepcopy(self.config)
            changed[key] = value
            with self.assertRaisesRegex(live.LiveEvalError, "changed after"):
                live.validate_frozen_evidence(changed, self.campaign, self.runs)
        changed_cases = copy.deepcopy(self.cases)
        changed_cases["handoffs"]["handoff-01"]["task_text"] = "Changed task"
        with self.assertRaisesRegex(live.LiveEvalError, "private corpus changed"):
            live.validate_frozen_evidence(
                self.config, self.campaign, self.runs, changed_cases
            )

    def test_freeze_rejects_invalid_calibration_types_missing_samples_and_request_hash(
        self,
    ) -> None:
        for field, value in (
            ("structured_output_valid", "false"),
            ("source_ids_valid", 1),
            ("critical_facts_expected", 999),
            ("request_hash", "f" * 64),
        ):
            runs = copy.deepcopy(self.runs)
            handoff = next(
                row
                for row in runs
                if row["stage"] == "calibration" and row["case_kind"] == "handoff"
            )
            handoff[field] = value
            with self.subTest(field=field), self.assertRaises(live.LiveEvalError):
                live.validate_calibration_evidence(
                    self.config, self.campaign, runs, self.cases
                )
        with self.assertRaisesRegex(live.LiveEvalError, "complete"):
            live.validate_calibration_evidence(
                self.config, self.campaign, self.runs[1:]
            )

    def test_evaluations_reject_same_person_invalid_ids_and_calibration(self) -> None:
        run_by_id = aggregate.validate_runs(self.runs, self.campaign)
        valid = evaluations_for(self.runs)
        aggregate.validate_evaluations(valid, run_by_id)
        for field, value in (
            ("evaluator_ref", " PERSON-OWNER "),
            ("schema_version", "2.0"),
            ("comparison_id", {}),
            ("evaluator_ref", ""),
            ("repetition", True),
        ):
            evaluations = copy.deepcopy(valid)
            evaluations[1][field] = value
            with self.subTest(field=field), self.assertRaises(
                aggregate.ValidationError
            ):
                aggregate.validate_evaluations(evaluations, run_by_id)
        evaluation = copy.deepcopy(valid[0])
        for condition in live.CONDITIONS:
            calibration = next(
                row
                for row in self.runs
                if row["stage"] == "calibration"
                and row["case_id"] == "handoff-01"
                and row["condition"] == condition
                and row["model"] == "model-a"
            )
            evaluation[f"{condition}_run_id"] = calibration["run_id"]
        evaluation.update(task_id="handoff-01", repetition=1)
        with self.assertRaisesRegex(aggregate.ValidationError, "calibration"):
            aggregate.validate_evaluations([evaluation], run_by_id)

    def test_comparison_ids_are_bijective_with_run_pairs(self) -> None:
        run_by_id = aggregate.validate_runs(self.runs, self.campaign)
        valid = evaluations_for(self.runs)
        invalid = copy.deepcopy(valid)
        invalid[1]["comparison_id"] = "another-comparison"
        with self.assertRaisesRegex(
            aggregate.ValidationError, "plusieurs comparison_id"
        ):
            aggregate.validate_evaluations(invalid, run_by_id)
        invalid = [copy.deepcopy(valid[0]), copy.deepcopy(valid[3])]
        invalid[1]["comparison_id"] = invalid[0]["comparison_id"]
        with self.assertRaisesRegex(aggregate.ValidationError, "runs différents"):
            aggregate.validate_evaluations(invalid, run_by_id)

    def reserve(self, *, unstable: bool) -> None:
        main = [
            row
            for row in self.runs
            if row["stage"] == "main" and row["case_id"] == "handoff-01"
        ]
        if unstable:
            next(row for row in main if row["condition"] == "full_dump")[
                "relevant_items_present"
            ] = 0
        self.config["reserve_case_ids"] = ["handoff-01"]
        self.config["reserve_justifications"] = {
            "handoff-01": {
                "rule": "quality_metrics_vary",
                "main_runs_hash": live.runs_hash(main),
            }
        }

    def test_reserve_requires_recorded_trigger_hash_and_all_three_main_repetitions(
        self,
    ) -> None:
        self.reserve(unstable=False)
        with self.assertRaisesRegex(live.LiveEvalError, "not triggered"):
            live.validate_reserve_evidence(self.config, self.runs)
        self.reserve(unstable=True)
        live.validate_frozen_evidence(self.config, self.campaign, self.runs, self.cases)
        live.validate_reserve_evidence(self.config, self.runs)
        incomplete = [
            row
            for row in self.runs
            if not (
                row["stage"] == "main"
                and row["case_id"] == "handoff-01"
                and row["repetition"] == 3
            )
        ]
        with self.assertRaisesRegex(live.LiveEvalError, "all three"):
            live.validate_reserve_evidence(self.config, incomplete)
        self.config["reserve_justifications"]["handoff-01"]["main_runs_hash"] = "f" * 64
        with self.assertRaisesRegex(live.LiveEvalError, "hash mismatch"):
            live.validate_reserve_evidence(self.config, self.runs)

    def test_reserve_comparisons_are_required_and_count_towards_owner_and_secondary(
        self,
    ) -> None:
        self.reserve(unstable=True)
        reserved_runs = records_for_plan(
            self.campaign, self.config, self.cases, "reserve"
        )
        all_runs = self.runs + reserved_runs
        evaluations = evaluations_for(self.runs)
        report = aggregate.build_report(
            self.campaign, all_runs, evaluations, {"handoff-01"}
        )
        self.assertFalse(report["sample"]["complete"])
        self.assertIn("comparaisons owner: 6/8", report["sample"]["missing"])
        evaluations = evaluations_for(all_runs)
        aggregate.validate_evaluations(
            evaluations, aggregate.validate_runs(all_runs, self.campaign)
        )
        report = aggregate.build_report(
            self.campaign, all_runs, evaluations, {"handoff-01"}
        )
        self.assertTrue(report["sample"]["complete"])
        self.assertTrue(report["gate_passed"])
        report = aggregate.build_report(
            self.campaign, self.runs, evaluations_for(self.runs), {"handoff-01"}
        )
        self.assertFalse(report["sample"]["complete"])

    def test_main_repetitions_cannot_be_replaced_with_reserve_or_calibration(
        self,
    ) -> None:
        for stage, repetition in (("main", 4), ("calibration", 2), ("reserve", 3)):
            runs = copy.deepcopy(self.runs)
            runs[0].update(stage=stage, repetition=repetition)
            with self.subTest(stage=stage), self.assertRaisesRegex(
                aggregate.ValidationError, "incompatible"
            ):
                aggregate.validate_runs(runs, self.campaign)

    def test_campaign_rejects_duplicate_or_overlapping_annotations_in_both_commands(
        self,
    ) -> None:
        manifest = campaign(task_count=12, pair_count=60)
        for field in ("critical_fact_ids", "relevant_item_ids", "irrelevant_item_ids"):
            changed = copy.deepcopy(manifest)
            changed["handoff_tasks"][0][field] *= 2
            with self.subTest(field=field), self.assertRaises(live.LiveEvalError):
                live.validate_campaign(changed)
        manifest["handoff_tasks"][0]["irrelevant_item_ids"] = manifest["handoff_tasks"][
            0
        ]["relevant_item_ids"]
        with self.assertRaisesRegex(live.LiveEvalError, "disjointes"):
            live.validate_campaign(manifest)

    def test_full_offline_report_requires_valid_config_and_remains_redacted(
        self,
    ) -> None:
        manifest, configuration, cases, runs = fixture(full=True)
        live.validate_frozen_evidence(configuration, manifest, runs, cases)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            args = argparse.Namespace(
                manifest=root / "campaign.json",
                config=root / "config.json",
                runs=root / "runs.jsonl",
                evaluations=root / "evaluations.jsonl",
                output=root / "report.json",
                force=True,
            )
            args.manifest.write_text(json.dumps(manifest))
            args.config.write_text(json.dumps(configuration))
            args.runs.write_text("\n".join(json.dumps(row) for row in runs) + "\n")
            args.evaluations.write_text(
                "\n".join(json.dumps(row) for row in evaluations_for(runs)) + "\n"
            )
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(aggregate.command_summarize(args), 0)
            report = json.loads(args.output.read_text())
            self.assertEqual(report["sample"]["owner_comparisons"], 36)
            self.assertNotIn("person-owner", args.output.read_text())
            configuration["selected_model"] = "model-b"
            args.config.write_text(json.dumps(configuration))
            with self.assertRaises(aggregate.ValidationError):
                aggregate.command_summarize(args)

    def test_freeze_and_reserve_commands_preserve_pre_registered_rules_without_network(
        self,
    ) -> None:
        manifest, configuration, cases, runs = fixture(full=True)
        calibration = [row for row in runs if row["stage"] == "calibration"]
        configuration.update(selected_model=None, frozen_contract=None)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            args = argparse.Namespace(
                manifest=root / "campaign.json",
                config=root / "config.json",
                cases=root / "cases.json",
                runs=root / "runs.jsonl",
                case_id="pair-001",
                rule="predicted_labels_vary",
            )
            for path, value in (
                (args.manifest, manifest),
                (args.config, configuration),
                (args.cases, cases),
            ):
                live.write_json_private(path, value, overwrite=False)
            args.runs.write_text(
                "\n".join(json.dumps(row) for row in calibration) + "\n"
            )
            with (
                mock.patch.object(live, "default_private_root", return_value=root),
                mock.patch.object(
                    live.ResponsesClient,
                    "create",
                    side_effect=AssertionError(
                        "offline commands must not call provider"
                    ),
                ),
                mock.patch.dict(live.os.environ, {}, clear=True),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                self.assertEqual(live.command_freeze(args), 0)
                frozen = live.read_json(args.config)
                live.validate_frozen_evidence(frozen, manifest, runs, cases)
                with self.assertRaisesRegex(live.LiveEvalError, "already frozen"):
                    live.command_freeze(args)
                args.runs.write_text("\n".join(json.dumps(row) for row in runs) + "\n")
                before = args.config.read_bytes()
                with self.assertRaisesRegex(live.LiveEvalError, "not triggered"):
                    live.command_register_reserve(args)
                self.assertEqual(args.config.read_bytes(), before)
                next(
                    row
                    for row in runs
                    if row["stage"] == "main" and row["case_id"] == "pair-001"
                )["predicted_label"] = "ambiguous"
                args.runs.write_text("\n".join(json.dumps(row) for row in runs) + "\n")
                self.assertEqual(live.command_register_reserve(args), 0)
                updated = live.read_json(args.config)
                self.assertEqual(updated["frozen_contract"], frozen["frozen_contract"])
                live.validate_frozen_evidence(updated, manifest, runs, cases)
                live.validate_reserve_evidence(updated, runs)
                with self.assertRaisesRegex(live.LiveEvalError, "already registered"):
                    live.command_register_reserve(args)


if __name__ == "__main__":
    unittest.main()
