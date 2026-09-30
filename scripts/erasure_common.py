"""Pinned, private operator PostgreSQL transport shared by maintenance CLIs."""
import ipaddress
import os
import subprocess
import sys
import urllib.parse


def run_sql(parser, sql):
    raw = os.environ.get("AI_CENTER_OPERATOR_DATABASE_URL", "")
    try:
        parsed = urllib.parse.urlparse(raw)
        port = parsed.port or 5432
    except ValueError:
        parser.error("Invalid operator connection configuration")
    if (parsed.scheme not in ("postgres", "postgresql") or parsed.username != "postgres"
            or not parsed.hostname or parsed.query or parsed.fragment):
        parser.error("A direct postgres operator connection without query/fragment is required")
    database = urllib.parse.unquote(parsed.path.removeprefix("/"))
    if (parsed.hostname != os.environ.get("AI_CENTER_ERASURE_ALLOWED_HOST")
            or str(port) != os.environ.get("AI_CENTER_ERASURE_ALLOWED_PORT")
            or database != os.environ.get("AI_CENTER_ERASURE_ALLOWED_DATABASE")
            or not database):
        parser.error("Pin the exact allowed host, port and database in the erasure configuration")
    try:
        loopback = ipaddress.ip_address(parsed.hostname).is_loopback
    except ValueError:
        loopback = parsed.hostname == "localhost"
    # PGHOSTADDR and PGSERVICE can otherwise redirect an apparently pinned host.
    # Keep only certificate settings; all connection parameters are explicit.
    tls_settings = {"PGSSLROOTCERT", "PGSSLCRL", "PGSSLCRLDIR", "PGSSLCERT", "PGSSLKEY"}
    env = {name: value for name, value in os.environ.items()
           if not name.startswith("PG") or name in tls_settings}
    # Keep credentials out of argv, SQL, stdout and stderr. Remote connections
    # require verification with the operator's configured CA certificate.
    env.update(PGHOST=parsed.hostname, PGPORT=str(port), PGUSER="postgres", PGDATABASE=database,
               PGPASSWORD=urllib.parse.unquote(parsed.password or ""),
               PGSSLMODE="disable" if loopback else "verify-full",
               PGOPTIONS="-c statement_timeout=30000 -c lock_timeout=2000 -c idle_in_transaction_session_timeout=30000")
    env.pop("AI_CENTER_OPERATOR_DATABASE_URL", None)
    try:
        result = subprocess.run(["psql", "-X", "--no-password", "--quiet", "--tuples-only", "--no-align", "--set=ON_ERROR_STOP=1"],
                                input=sql, text=True, env=env, capture_output=True, timeout=40, check=False)
    except (FileNotFoundError, subprocess.TimeoutExpired):
        print("Operator client unavailable or operation timed out; no success receipt was obtained.", file=sys.stderr)
        return 1
    if result.returncode:
        # Never relay arbitrary server diagnostics that may contain row content.
        print("Erasure refused or transaction failed. No success receipt was obtained; inspect protected operator logs and refresh the preview before retrying.", file=sys.stderr)
        return 1
    print(result.stdout.strip())
    return 0