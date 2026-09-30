#!/usr/bin/env python3
"""Preview or execute one operator-authorized company erasure; never an API."""
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
    parser.add_argument("--receipt", help="Exact receipt returned by preview")
    parser.add_argument("--confirm", type=uuid.UUID, help="Repeat the exact workspace UUID")
    args = parser.parse_args()
    workspace = str(args.workspace)
    if args.mode == "preview":
        sql = f"begin isolation level repeatable read read only; select app.operator_workspace_manifest('{workspace}'::uuid); commit;"
    else:
        if (os.environ.get("AI_CENTER_ALLOW_COMPANY_ERASURE") != "yes"
                or args.confirm != args.workspace or not re.fullmatch(r"[0-9a-f]{32}", args.receipt or "")):
            parser.error("Execution requires explicit erasure enablement, exact UUID confirmation and preview receipt")
        sql = f"begin; select app.operator_purge_workspace('{workspace}'::uuid,'{workspace}'::uuid,'{args.receipt}'); commit;"
    return erasure_common.run_sql(parser, sql)


if __name__ == "__main__":
    sys.exit(main())
