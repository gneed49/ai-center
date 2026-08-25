# Images OCI portables

Statut : **présentes, vérifiées statiquement, non reproduites localement**. Le
daemon Docker n'est pas disponible dans l'environnement de développement ayant
produit ce lot. La CI construit les deux images sans les publier.

## Périmètre

- `deploy/oci/server.Dockerfile` produit uniquement l'API Axum Linux ;
- `deploy/oci/web.Dockerfile` produit le web desktop statique servi par nginx ;
- aucune cible, suite ou ressource Android, iOS ou mobile n'est incluse ;
- les images sont indépendantes de l'hébergeur et ne publient rien par défaut.

## Construction

Depuis la racine du dépôt :

```bash
docker build --file deploy/oci/server.Dockerfile --tag ai-center-api:local .
docker build --file deploy/oci/web.Dockerfile --tag ai-center-web:local .
```

`VITE_SUPABASE_URL` et `VITE_SUPABASE_ANON_KEY` peuvent être transmis comme
arguments de construction pour l'authentification navigateur. La clé anon est
une configuration cliente publique, jamais une clé service-role. Aucun autre
secret ne doit être fourni comme argument de construction.

## Exécution

L'API écoute sur `0.0.0.0:4317` dans le conteneur. Elle conserve le contrat de
configuration par variables d'environnement de l'application. En particulier,
un déploiement exposé doit fournir `AI_CENTER_AUTH_MODE=supabase`,
`SUPABASE_URL`, `DATABASE_URL`, `AI_CENTER_CORS_ORIGINS` et, lorsque le moteur
réel est activé, `AI_CENTER_AGENT_MODE=openai`, `OPENAI_MODEL` et
`OPENAI_API_KEY`.

Les secrets sont injectés uniquement au runtime par le mécanisme de secrets de
l'environnement cible. La clé privée GitHub App est montée en lecture seule et
référencée par `GITHUB_APP_PRIVATE_KEY_PATH`; elle n'est jamais copiée dans une
image.

Le web écoute sur `8080`. Il appelle `/api` sur sa propre origine et nginx
redirige cette route vers `AI_CENTER_API_UPSTREAM`, configurable au démarrage :

```bash
docker run --rm --publish 8080:8080 \
  --env AI_CENTER_API_UPSTREAM=http://api.internal:4317 \
  ai-center-web:local
```

Ce reverse proxy évite de reconstruire le bundle web à chaque changement
d'hôte de l'API. `/healthz` teste nginx et `/api/health` teste l'API ainsi que sa
connexion PostgreSQL.

## Sécurité de l'image

- contextes `.env*`, certificats et clés privées exclus par `.dockerignore` ;
- copie explicite des sources nécessaires dans l'image API ;
- utilisateurs non-root dans les deux images finales ;
- images de base versionnées et build Rust exécuté avec `--locked` ;
- publication absente du workflow `.github/workflows/oci-build.yml`.

Exécuter la vérification locale sans daemon :

```bash
./scripts/check-oci-packaging.sh
git diff --check
```
