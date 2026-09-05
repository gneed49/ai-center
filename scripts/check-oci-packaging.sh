#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

required_files=(
  .dockerignore
  deploy/oci/server.Dockerfile
  deploy/oci/web.Dockerfile
  deploy/oci/nginx/default.conf.template
)

for file in "${required_files[@]}"; do
  test -s "$file" || {
    echo "missing OCI packaging file: $file" >&2
    exit 1
  }
done

grep -Fxq '.env.*' .dockerignore
grep -Fxq '**/.env.*' .dockerignore
grep -Fxq '*.pem' .dockerignore
grep -Fq 'cargo build --locked --release --package ai-center-server' deploy/oci/server.Dockerfile
grep -Fq 'USER 10001:10001' deploy/oci/server.Dockerfile
grep -Fq 'AI_CENTER_API_UPSTREAM' deploy/oci/web.Dockerfile
grep -Fq 'location = /healthz' deploy/oci/nginx/default.conf.template
grep -Fq 'location /api/' deploy/oci/nginx/default.conf.template

if grep -Eiq '(android|ios|mobile)' deploy/oci/*.Dockerfile deploy/oci/nginx/*.template; then
  echo "mobile-specific content found in desktop OCI packaging" >&2
  exit 1
fi

if grep -Eiq '(OPENAI_API_KEY|DATABASE_URL|GITHUB_APP_PRIVATE_KEY|SUPABASE_SERVICE_ROLE).*=.+' \
  deploy/oci/*.Dockerfile deploy/oci/nginx/*.template; then
  echo "a credential-like value appears to be baked into OCI packaging" >&2
  exit 1
fi

echo "OCI packaging static checks passed"
