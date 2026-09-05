# syntax=docker/dockerfile:1.7

FROM node:22.22.0-bookworm-slim AS build

WORKDIR /workspace

COPY package.json package-lock.json ./
COPY apps/web/package.json apps/web/package.json

RUN --mount=type=cache,target=/root/.npm,sharing=locked \
    npm ci --ignore-scripts

COPY apps/web apps/web

# The browser always calls the same origin. Nginx selects the actual API
# upstream at container startup, so the shipped bundle is hosting-neutral.
ARG VITE_API_URL=/
ARG VITE_SUPABASE_URL=
ARG VITE_SUPABASE_ANON_KEY=
ENV VITE_API_URL=${VITE_API_URL} \
    VITE_SUPABASE_URL=${VITE_SUPABASE_URL} \
    VITE_SUPABASE_ANON_KEY=${VITE_SUPABASE_ANON_KEY}

RUN npm run build --workspace @ai-center/web

FROM nginxinc/nginx-unprivileged:1.29.1-alpine3.22 AS runtime

ARG OCI_VERSION=dev
ARG OCI_REVISION=unknown

LABEL org.opencontainers.image.title="AI Center Web" \
      org.opencontainers.image.description="Desktop web control plane for AI Center" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.source="https://github.com/gneed49/ai-center" \
      org.opencontainers.image.licenses="UNLICENSED"

ENV AI_CENTER_API_UPSTREAM=http://api:4317 \
    NGINX_ENVSUBST_FILTER=AI_CENTER_API_UPSTREAM

COPY deploy/oci/nginx/default.conf.template /etc/nginx/templates/default.conf.template
COPY --from=build /workspace/apps/web/dist /usr/share/nginx/html

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["wget", "--quiet", "--output-document=-", "http://127.0.0.1:8080/healthz"]
