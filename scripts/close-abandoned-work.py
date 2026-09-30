#!/usr/bin/env python3
"""Inspect and explicitly close abandoned work while the company is stopped for maintenance."""
import argparse
import os
import re
import sys
import uuid
import erasure_common


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("preview", "execute"))
    parser.add_argument("workspace", type=uuid.UUID)
    parser.add_argument("--receipt")
    parser.add_argument("--confirm", type=uuid.UUID)
    parser.add_argument("--operator-actor", type=uuid.UUID)
    parser.add_argument("--workers-stopped", action="store_true", help="Confirm every app server and worker has been stopped")
    args = parser.parse_args()
    workspace = str(args.workspace)
    if args.mode == "preview":
        sql = f"begin isolation level repeatable read read only; select app.operator_abandoned_manifest('{workspace}'::uuid); commit;"
    else:
        if (os.environ.get("AI_CENTER_ALLOW_ABANDONED_MAINTENANCE") != "yes"
                or args.confirm != args.workspace or not args.workers_stopped
                or not args.operator_actor or args.operator_actor.int == 0
                or not re.fullmatch(r"[0-9a-f]{32}", args.receipt or "")):
            parser.error("Closing abandoned work requires enablement, exact workspace confirmation, receipt, operator identity and explicit confirmation that all workers are stopped")
        sql = f"begin; select app.operator_close_abandoned('{workspace}'::uuid,'{workspace}'::uuid,'{args.receipt}',true,'{args.operator_actor}'::uuid); commit;"
    return erasure_common.run_sql(parser, sql)


if __name__ == "__main__":
    sys.exit(main())
