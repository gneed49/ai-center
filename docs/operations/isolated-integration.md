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
exécute dans l'ordre `ticket-browser`, `source-browser`, `integration`,
`tls-smoke`, `backup-restore`, `auth-smoke`, `auth-browser`, `real-e2e`, puis
arrête cette seule stack avec suppression de ses volumes.
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

La phase `integration` comprend migration baseline→schéma courant, reset de la seule
base de test, pgTAP, contrôle/provisionnement du rôle runtime et tests Rust
PostgreSQL. Le mot de passe runtime est généré pour chaque run et transmis
uniquement par l’environnement des processus.

Le contrôle d'upgrade découvre toutes les migrations versionnées dans l'ordre,
à partir de la baseline du 18 août. Il refuse les noms ambigus, versions dupliquées,
fichiers vides et liens symboliques avant de créer sa base temporaire. Deux
workspaces avec connaissances confirmées et événements antérieurs sont créés
avant la première mise à niveau ; les assertions vérifient leur conservation,
les owners acceptés et les effets des migrations jusqu'aux connexions personnelles.
Chaque erreur SQL arrête la chaîne et déclenche le nettoyage de cette seule base.

Pour exécuter seulement une partie du contrôle, lister les phases nécessaires.
Le démarrage applique les migrations et prépare le rôle runtime même pour une
phase isolée. `integration` refait ensuite sa propre preuve de mise à niveau,
son reset gardé et les contrôles de privilèges :

```bash
./scripts/integration-stack.sh run integration
./scripts/integration-stack.sh run integration backup-restore
./scripts/integration-stack.sh run integration auth-smoke real-e2e
./scripts/integration-stack.sh run source-browser
```

`source-browser` utilise les vraies routes Axum, les transactions, reçus et RLS
dans Chromium. Le seul fournisseur injecté est un serveur HTTP loopback fictif,
via un constructeur absent des compilations de production. Le scénario vérifie
Linear avec réponse perdue, Notion puis nouvelle lecture partielle, historique,
compteurs d'appels, mobile et accessibilité. Auth est local dans ce harnais :
`auth-smoke` et `auth-browser` qualifient séparément les sessions Supabase locales.
Les diagnostics synthétiques de cette phase sont sous
`/tmp/ai-center-source-real-playwright-results` ; les journaux de démarrage et
les identifiants éphémères de la stack n'entrent pas dans ces artefacts.

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
ne le sont jamais. `prepare` crée lui-même le seul pin PostgreSQL décrit
ci-dessous dans le `.temp` CI. La base neuve utilise les sources SQL du checkout
courant. Des fichiers dotenv vides empêchent le serveur de remonter vers les
secrets du développement lorsqu’il démarre dans la stack jetable.

## Version PostgreSQL et preuve de l'image

La CLI reste figée à **2.114.0** et le major à **17**. Le lifecycle produit
`supabase/.temp/postgres-version` avec **17.11.0.002** dans le seul workdir CI.
Cette sélection est lue par le [code officiel de la CLI](https://github.com/supabase/cli/blob/v2.114.0/apps/cli/src/legacy/shared/legacy-db-image.ts#L95)
et son [bootstrap](https://github.com/supabase/cli/blob/v2.114.0/apps/cli/src/legacy/shared/db-bootstrap/bootstrap-config.ts#L284),
y compris lors d'un reset. Il n'y a ni changement de moteur, ni retag, ni lien
vers une base distante. La [release Supabase PostgreSQL](https://github.com/supabase/postgres/releases/tag/v17.11.0.002-cli)
inclut PostgreSQL 17.11 ; Docker Hub et le miroir ECR officiel publient les images
Linux AMD64 et ARM64.

Le marqueur de workdir version 2 engage ce pin et le digest d'index
`sha256:0450166354dc9c1d25f0322ac8b580774d4fb0184d2b087f6e4fe9499c66cf53`.
Un ancien marqueur version 1 exact peut être arrêté ou préparé vers la version 2 ;
un marqueur courant dont le pin manque ou a changé est refusé. Les liens
symboliques et les métadonnées de liaison distante sont également refusés.
La préparation seule ne prouve pas qu'une image a démarré.

Après démarrage, après chaque reset et avant les phases SQL, le contrôle exige
le conteneur courant, son label de checkout, une image officielle 17.11.0.002,
son ID et l'un des digests officiels autorisés pour son architecture. Il lit
ensuite `server_version_num`, attendu à **170011**, sur le serveur du conteneur.
Le reçu privé `.run/integration-stack/postgres-image.json` contient la date,
les identités d'image, l'architecture, le digest et la version constatés ; aucun
environnement de conteneur ou secret n'est inspecté. Aucun reçu antérieur
n'autorise une action ; chaque contrôle atteste à nouveau le serveur courant.
Le contrôle ne tire aucune image
lui-même : le démarrage habituel de Supabase effectue les téléchargements requis.

Les overrides de binaire CLI, registre d'image, OrioleDB et stack expérimentale
sont refusés ; `SUPABASE_DB_MAJOR_VERSION`, si présent, doit être `17`.
La récupération Podman garde le code `LegacyStopVolumePruneError`, les labels et
les deux seuls volumes autorisés. L'arrêt ne dépend pas d'une base vivante ou
d'une attestation d'image réussie. Ne pas élargir ce nettoyage aux volumes
historiques qui ne portent pas les marqueurs requis.

Les tests hors réseau vérifient ces refus et le raccord au reset avec des doubles.
Seul un nouveau lifecycle complet qualifie l'image corrigée avec migrations,
Auth, RLS, TLS et restauration ; les journaux antérieurs sur 17.6 ne le prouvent
pas. La version d'une cible hébergée doit être vérifiée séparément.

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
python3 -m unittest discover -s scripts/tests -p 'test_migration_plan.py'
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
