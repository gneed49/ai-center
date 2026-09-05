# Intégration desktop isolée

La commande complète de certification utilise une stack Supabase jetable,
distincte de celle de développement :

```bash
npm ci
./scripts/integration-stack.sh run
```

Prérequis : Python 3.11+, `flock`, Docker compatible avec la CLI Supabase,
Node/npm, Rust, les clients PostgreSQL 17 et Chromium Playwright. Le mode IA
reste `deterministic`. Aucun fournisseur réel, secret de développement ou
client mobile n’est nécessaire.

## Cycle de vie

`run` prépare `.run/integration-stack/`, démarre Supabase, vérifie son identité,
exécute dans l’ordre les contrôles `integration`, `backup-restore`, `auth-smoke`
et `real-e2e`, puis arrête cette seule stack avec suppression de ses volumes.
Le nettoyage s’exécute également après échec ou interruption. Les journaux de
démarrage et de nettoyage restent dans le répertoire privé ignoré par Git ;
ils ne sont pas publiés comme artefacts de CI.

Sous Podman, certaines versions de la CLI arrêtent les conteneurs puis
échouent avec `LegacyStopVolumePruneError`. Ce seul cas possède un rattrapage :
le moteur Podman doit répondre sur le même socket Unix explicite de
`DOCKER_HOST`, aucun conteneur du projet ne doit subsister, et chaque volume
doit porter le nom attendu et le label exact de cette identité CI. Seuls les
volumes DB/storage validés et détachés sont supprimés, individuellement et sans
force. Toute autre erreur reste un échec de nettoyage.

La phase `integration` comprend migration baseline→alpha, reset de la seule
base de test, pgTAP, contrôle/provisionnement du rôle runtime et tests Rust
PostgreSQL. Le mot de passe runtime est généré pour chaque run et transmis
uniquement par l’environnement des processus.

Pour exécuter seulement une partie du contrôle, lister les phases nécessaires.
Les phases qui utilisent les données applicatives nécessitent `integration`
avant elles sur une stack neuve :

```bash
./scripts/integration-stack.sh run integration
./scripts/integration-stack.sh run integration backup-restore
./scripts/integration-stack.sh run integration auth-smoke real-e2e
```

`prepare` ne démarre aucun conteneur et ne touche aucune base. Il permet
d’inspecter la configuration générée. `stop` fournit un nettoyage explicite
après une interruption brutale du processus ; il refuse d’interrompre un run
encore actif.

```bash
./scripts/integration-stack.sh prepare
./scripts/integration-stack.sh stop
```

## Identité et ports

| Surface                         | Développement | Intégration                            |
| ------------------------------- | ------------- | -------------------------------------- |
| Identité Supabase               | `AICenter`    | `ai-center-ci-<empreinte du checkout>` |
| PostgreSQL                      | 54322         | 55322                                  |
| API Supabase                    | 54321         | 55321                                  |
| Shadow / mail / autres services | 5432x         | 5532x                                  |
| API Axum                        | 4317          | 4617                                   |
| Smoke Auth Axum                 | 4318          | 4618                                   |
| Web desktop                     | 5173          | 5183                                   |

Les ports sont fixes pour rendre la cible destructible explicite. Plusieurs
checkouts ont des identités et volumes distincts, mais leurs suites doivent
être exécutées successivement. Un port occupé fait échouer le démarrage sans
arrêter l’autre service. Un verrou couvre préparation, tests et teardown dans
un checkout donné.

Le template `supabase/config.integration.toml` est indépendant du fichier de
développement. Seuls `roles.sql`, `seed.sql`, `migrations/`, `schemas/` et
`tests/` sont copiés ; `.env`, les identifiants de projet distant et `.temp`
ne le sont jamais. La base neuve utilise donc les sources SQL du checkout
courant. Des fichiers dotenv vides empêchent le serveur de remonter vers les
secrets du développement lorsqu’il démarre dans la stack jetable.

## Gardes avant mutation

Avant reset, smoke SQL/Auth ou arrêt, les scripts exigent le workdir canonique
sans symlink, le marqueur du checkout, le template CI exact et les endpoints
loopback prévus. Le statut de la CLI doit confirmer les URLs DB/API de cette
stack avant les commandes SQL. Une URL de développement, même accompagnée
d’un override de port, est refusée avant de contacter la CLI ou la base.
Les anciens appels directs `ci-desktop.sh integration` avec la DB 54322 ne
sont plus autorisés.

Les variables shell `DATABASE_URL`, `AI_CENTER_ADMIN_DATABASE_URL` et
`AI_CENTER_RUNTIME_DATABASE_URL`, lorsqu’elles existent déjà, doivent être
compatibles avec ce contrat. Sinon lancer la suite depuis un shell sans ces
variables. Le script n’affiche jamais leur valeur dans ses refus.

Contrôles hors réseau :

```bash
python3 -m unittest discover -s scripts/tests -p 'test_integration_target.py'
bash -n scripts/integration-common.sh scripts/integration-stack.sh scripts/ci-desktop.sh
```

Ces gardes et mocks CLI prouvent les refus et le ciblage du nettoyage ; seule
une exécution complète avec conteneurs prouve les migrations, l’Auth, le
backup/restore et le parcours navigateur réels. Aucun de ces contrôles ne
constitue un déploiement de l’alpha privée.

Le 5 septembre 2026, le lifecycle de reprise ACP-T01 a été reproduit sur
Podman 5.8.4 avec le serveur recompilé depuis son worktree : migration
baseline→alpha, 32 pgTAP, six tests PostgreSQL existants, restauration de
39 tables et 96 policies, Auth magic-link et un parcours Chromium réel avec
reload et sans erreur console. Le run termine avec code 0 ; des inventaires
ciblés confirment zéro conteneur et zéro volume CI restant. Les 17 tests de
garde hors réseau passent aussi. Les nouvelles suites d’invalidation et de
persistance GitHub doivent compléter ces preuves sur la branche consolidée.

La configuration utilise les options documentées de la [CLI Supabase](https://supabase.com/docs/reference/cli/introduction) :
`--workdir` sélectionne le projet local ; `stop --project-id` limite l’arrêt à
une identité ; `--no-backup` supprime ses volumes. Aucun `stop --all` n’est
utilisé.
