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

## Importer et rafraîchir une référence

L'interface accepte trois URL canoniques :

- dépôt : `https://github.com/owner/repository` ;
- PR : `https://github.com/owner/repository/pull/123` ;
- commit : `https://github.com/owner/repository/commit/<SHA complet>`.

Les SHA abrégés, branches, paramètres, fragments, redirections et chemins
normalisés ou encodés sont refusés. Le dépôt conserve uniquement son identité,
son état actif/archivé et ses dates ; une preuve requiert une PR ou un commit
portant une révision précise. Les références PR déjà enregistrées conservent
leur identité et leur API d'import.

Les checks comprennent les check-runs et les commit statuses. Leur type est
conservé pour distinguer deux systèmes CI utilisant le même nom. Les checks
sont relus sur le SHA concerné même lorsque les métadonnées PR/commit répondent
`304 Not Modified`. Le connecteur refuse une projection paginée incomplète ;
il n'importe ni patch, ni contenu de fichier, ni message de commit, ni sortie CI.

Un `403` avec `Retry-After` ou `X-RateLimit-Remaining: 0`, ainsi qu'un `429`,
est traité comme une limite transitoire. Le serveur répond `503` avec un
`Retry-After` nettoyé, garde la preuve et la couverture inchangées, et permet
le rejeu de la même commande. Le délai le plus strict entre `Retry-After` et
`X-RateLimit-Reset` est respecté. Au-delà de cinq secondes, l'appel rend la
main immédiatement ; les commandes suivantes partagent le délai restant dans
le processus serveur. Une perte d'accès `403` sans indication de limite ou un
`404` sur la référence conserve son historique et rend les preuves indisponibles.
Une erreur d'authentification de la GitHub App n'est pas une observation de
perte d'accès à une référence.

Les contrats HTTP utilisent uniquement un faux serveur local. Le test de
persistance PostgreSQL est explicite :

```bash
cargo test -p ai-center-server --lib \
  github_persistence_preserves_proofs_under_rate_limits_and_binds_idempotence_to_target \
  -- --ignored --test-threads=1
```

Il requiert la stack d'intégration jetable, `DATABASE_URL` pour le rôle runtime
et `AI_CENTER_ADMIN_DATABASE_URL` pour provisionner uniquement la connexion
GitHub factice. Il ne constitue pas une preuve d'installation GitHub réelle.

Références officielles consultées : [limites de débit GitHub](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api),
[commit statuses](https://docs.github.com/en/rest/commits/statuses),
[métadonnées repository](https://docs.github.com/en/rest/repos/repos#get-a-repository)
et [métadonnées commit](https://docs.github.com/en/rest/commits/commits#get-a-commit).
