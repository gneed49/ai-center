# Rôle PostgreSQL d’exécution

Le serveur Axum doit se connecter avec `ai_center_runtime`, jamais avec le rôle
de migration. Ce login est explicitement `NOSUPERUSER`, `NOBYPASSRLS`,
`NOCREATEDB`, `NOCREATEROLE`, `NOREPLICATION` et `NOINHERIT`. Il peut utiliser
les données du schéma privé `app` uniquement à travers les grants listés et les
policies RLS forcées.

## Sources de vérité

- `supabase/roles.sql` décrit le rôle cluster. La CLI Supabase applique ce
  fichier avant les migrations et `db push --include-roles` le déploie sur un
  projet lié.
- `supabase/schemas/02_runtime_access.sql` décrit les grants. Il est chargé
  après `01_domain.sql` par `schema_paths = ["./schemas/*.sql"]`.
- `scripts/sql/verify-runtime-db-role.sql` échoue si le rôle est élevé, hérite
  d’un autre rôle, dispose d’un droit destructif, exécute une fonction non
  autorisée ou lit une table sans RLS forcée.

Les changements de schéma commencent toujours dans `supabase/schemas/`. La
migration correspondante doit ensuite être générée par la CLI et relue ; elle
ne doit pas être écrite à la main.

```bash
npm run supabase -- db diff -f alpha_context_proof
npm run supabase -- db reset --local
```

## Matrice des privilèges

| Objet                                               | Privilèges runtime                                                |
| --------------------------------------------------- | ----------------------------------------------------------------- |
| Base courante                                       | `CONNECT` uniquement en plus des droits publics de la plateforme  |
| Schéma `app`                                        | `USAGE` uniquement                                                |
| Tables consultées par l’API                         | `SELECT`                                                          |
| Tables append-only ou créées par l’API              | `INSERT`                                                          |
| Projections et états explicitement mutables         | `UPDATE`                                                          |
| Séquences des tables insérées                       | `USAGE`                                                           |
| Fonctions de contexte, d’autorisation et de reprise | `EXECUTE`, allowlist de sept fonctions                            |
| Toutes les tables                                   | aucun `DELETE`, `TRUNCATE`, `REFERENCES`, `TRIGGER` ou `MAINTAIN` |
| DDL et contournement RLS                            | aucun                                                             |

Les futures tables et fonctions ne reçoivent aucun grant automatique. Toute
extension de la surface runtime doit donc être revue et ajoutée explicitement.

## Provisionnement local ou alpha

Le script est idempotent et réapplique les deux sources ci-dessus dans une
transaction. L’URL d’administration et le mot de passe runtime restent des
variables privées ; ils ne doivent être ni committés ni copiés dans une issue.

```bash
export AI_CENTER_ADMIN_DATABASE_URL='postgresql://...'
read -rsp 'Mot de passe runtime: ' AI_CENTER_RUNTIME_DB_PASSWORD
export AI_CENTER_RUNTIME_DB_PASSWORD
./scripts/runtime-db-role.sh apply
unset AI_CENTER_RUNTIME_DB_PASSWORD
```

Sans `AI_CENTER_RUNTIME_DB_PASSWORD`, un mot de passe existant est conservé.
Un rôle créé à neuf reste volontairement inutilisable à distance jusqu’à ce
qu’un secret soit configuré. Pour un déploiement Supabase lié, appliquer les
migrations et le rôle ensemble :

```bash
npm run supabase -- db push --include-roles
./scripts/runtime-db-role.sh verify
```

Le `DATABASE_URL` du serveur privé doit ensuite utiliser le login
`ai_center_runtime`. En mode `AI_CENTER_AUTH_MODE=supabase`, le serveur vérifie
aussi au démarrage que `current_user` n’est ni superuser ni `BYPASSRLS`.

Cette vérification certifie les attributs et grants du rôle. Elle ne remplace
pas les tests owner/editor/viewer et inter-workspace, qui doivent être exécutés
sur une base reconstruite avec Docker/Supabase avant promotion.

Le septième contrat, `app.list_due_steward_workspaces(integer)`, est un
bootstrap privé de reprise. Il ne retourne que des workspaces ayant un
événement `knowledge.committed` ou `knowledge.revised` arrivé à échéance et un
membre accepté `owner` ou `editor`. Il ne lit aucun payload et ne claim aucun
événement. Le serveur réinstalle ensuite actor/workspace/rôle dans des GUC
transactionnelles et traite le scope sous les grants runtime et la RLS forcée.
`PUBLIC`, `anon`, `authenticated` et `service_role` ne peuvent pas l’exécuter.
