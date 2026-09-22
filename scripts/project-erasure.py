#!/usr/bin/env python3
"""Preview or erase one isolated, operator-authorized project without outside dependencies."""
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
    parser.add_argument("project", type=uuid.UUID)
    parser.add_argument("--receipt")
    parser.add_argument("--confirm", type=uuid.UUID, help="Repeat the exact project UUID")
    args = parser.parse_args()
    workspace, project = str(args.workspace), str(args.project)
    if args.mode == "preview":
        sql = f"begin isolation level repeatable read read only; select app.operator_project_manifest('{workspace}'::uuid,'{project}'::uuid); commit;"
    else:
        if (os.environ.get("AI_CENTER_ALLOW_PROJECT_ERASURE") != "yes"
                or args.confirm != args.project or not re.fullmatch(r"[0-9a-f]{32}", args.receipt or "")):
            parser.error("Execution requires explicit project erasure enablement, exact project confirmation and preview receipt")
        sql = f"begin; select app.operator_purge_project('{workspace}'::uuid,'{project}'::uuid,'{project}'::uuid,'{args.receipt}'); commit;"
    return erasure_common.run_sql(parser, sql)


if __name__ == "__main__":
    sys.exit(main())
