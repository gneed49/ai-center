# Provisionner la connexion GitHub App

AI Center n'autorise pas le rôle HTTP `ai_center_runtime` à créer ou modifier
une connexion d'outil. Le provisionnement est une opération d'administration
séparée : PostgreSQL ne conserve qu'une référence opaque vers la clé privée,
jamais la clé elle-même.

## Préconditions

- GitHub App privée installée sur l'allowlist de dépôts voulue ;
- permissions strictement read-only : metadata, contents, pull requests,
  checks et commit statuses ;
- clé privée montée hors dépôt et référencée par
  `GITHUB_APP_PRIVATE_KEY_PATH` ;
- URL PostgreSQL d'administration disponible uniquement dans le terminal
  opérateur.

## Enregistrer la connexion

Créer d'abord une connexion `pending`. Le script est idempotent sur
`workspace + provider + installation`.

```bash
export AI_CENTER_ADMIN_DATABASE_URL='postgresql://...'
export AI_CENTER_WORKSPACE_ID='...'
export AI_CENTER_ACTOR_ID='...'
export AI_CENTER_GITHUB_INSTALLATION_ID='...'
export AI_CENTER_GITHUB_CONNECTION_STATUS='pending'
export AI_CENTER_GITHUB_SECRET_REFERENCE='env:GITHUB_APP_PRIVATE_KEY_PATH'
./scripts/provision-github-connection.sh
```

Après un contrôle live réussi de l'installation, rejouer explicitement avec
`AI_CENTER_GITHUB_CONNECTION_STATUS=active`. Le serveur n'utilise que la
connexion active dont l'installation correspond exactement à
`GITHUB_APP_INSTALLATION_ID`.

La sortie contient l'identifiant public et la référence opaque, mais aucune
credential. Ne jamais activer une connexion sur la seule base d'un seed ou
d'un test simulé.
