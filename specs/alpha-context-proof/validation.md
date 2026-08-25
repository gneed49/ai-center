# Validation — Alpha Context Proof

> Date d'ouverture : 25 août 2026
>
> État du jalon : en cours
>
> Clients couverts : web desktop et Tauri Linux uniquement

## Règle de tenue du registre

Une ligne ne passe à `Vérifié` qu'après reproduction courante. L'auteur ajoute
la date, la commande ou le scénario exact, l'environnement et le chemin de
l'artefact. Les rapports de CI, captures et résultats agrégés sont conservés
sans secret ni donnée du corpus.

## État courant du registre

| Surface                        | Statut                       | Constat courant                                                                             | Preuve encore attendue                          |
| ------------------------------ | ---------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| Positionnement context-control | `Vérifié`                    | Docs, ADR, UI et API excluent IDE, runner et production de code.                            | Validation qualitative en dogfood.              |
| Flow déterministe PostgreSQL   | `Vérifié`                    | Base neuve, rôle RLS, `mvp_flow` et navigateur full-stack passent localement et en CI.      | Exploitation prolongée sur l’hébergement alpha. |
| ContextPack sélectif           | `Vérifié`                    | Obligations, exclusions, budget, raisons, versions, hash et stale testés.                   | Qualité sur corpus réel.                        |
| Plan et couverture             | `Vérifié` déterministe       | Plan et couverture sont structurés depuis le pack, sans contenu `Credits v2` codé en dur.   | Sorties OpenAI et notation humaine.             |
| Steward générique              | `Vérifié` déterministe       | Assessments complets, outbox, crash recovery et contradictions multiples.                   | Précision/rappel sur 60 paires réelles.         |
| Résolution transactionnelle    | `Vérifié`                    | Révision, graph version, stale, recompilation et couverture rejoués sur PostgreSQL.         | Dogfood sur décisions réelles.                  |
| Auth et isolation RLS          | `Vérifié` localement         | 32 pgtap, rôle runtime, magic link, viewer `403`, workspace forgé `403`, sans bearer `401`. | Rotation JWKS et déploiement HTTPS.             |
| GitHub read-only               | `Présent mais non reproduit` | Client et ExternalReference certifiés par faux HTTP, ETag et anti-SSRF.                     | GitHub App privée et PR réelle.                 |
| Backup/restore PostgreSQL      | `Vérifié`                    | 39 tables, 96 policies, structure/données comparées, cleanup confirmé.                      | Restauration sur l'hébergement alpha.           |
| E2E UI desktop                 | `Vérifié`                    | 57/57 mockés sur trois projets desktop et 1/1 full-stack Chromium réel.                     | Scénarios d'erreur full-stack supplémentaires.  |
| Campagne IA réelle             | `Futur`                      | Harness 9/9 ; premier appel réel bloqué par `insufficient_quota`.                           | Quota actif, corpus approuvé et rapport A/B.    |
| CI de pull request             | `Vérifié`                    | Desktop CI #20 : 5/5 jobs ; OCI #19 : 3/3 jobs au commit `2162d60`.                         | Rejouer après toute modification fonctionnelle. |
| Certification pré-alpha        | `Présent mais non reproduit` | Workflow manuel compact/Firefox/Tauri versionné.                                            | Premier run manuel complet.                     |
| Android/iOS/mobile             | `Futur`                      | Explicitement différé et non bloquant.                                                      | Aucune preuve demandée pour ce jalon.           |

## Preuves reproduites le 25 août 2026

| Contrôle                                                                        | Résultat                                                                     | Limite                                                           |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `python3 scripts/check-markdown-links.py README.md docs specs`                  | `Vérifié` — registre de liens locaux valide.                                 | Les URLs distantes ne sont pas contactées.                       |
| `python3 -m unittest scripts/tests/test_alpha_live_eval.py`                     | `Vérifié` — 9/9.                                                             | Harness offline, aucune qualité IA réelle.                       |
| Budget 10/70/20, facts 100 %, relevant recall ≥ 85 %                            | `Vérifié` dans le runner.                                                    | Les seuils doivent encore être atteints sur le corpus réel.      |
| ESLint web, Vitest et build web                                                 | `Vérifié` — lint sans avertissement, 4/4, bundle produit.                    | Les E2E et le backend possèdent leurs preuves séparées.          |
| `cargo test --workspace --lib --offline`                                        | `Vérifié` — 75/75.                                                           | Faux serveurs loopback exécutés hors sandbox.                    |
| upgrade baseline→alpha                                                          | `Vérifié` — deux workspaces historiques conservent leur owner accepté.       | Migration locale isolée, pas encore déployée.                    |
| intégrations PostgreSQL                                                         | `Vérifié` — 6/6 sur quatre binaires, localement et dans Desktop CI #20.      | Stack isolée, pas encore hébergement alpha.                      |
| pgtap + vérificateur runtime                                                    | `Vérifié` — 32/32 et rôle dédié conforme, localement et dans CI #20.         | Pas encore hébergement alpha.                                    |
| Playwright API mockée                                                           | `Vérifié` — 19/19 par projet, 57/57 total.                                   | Contrats HTTP simulés.                                           |
| Playwright full-stack Chromium `1440×900`                                       | `Vérifié` — 1/1 localement et en CI #20, reload, zéro erreur.                | Happy path unique ; erreurs certifiées aux couches DB/faux HTTP. |
| navigateur intégré                                                              | `Vérifié` — Center, projet, handoff et provenance ContextPack inspectés.     | Contrôle exploratoire desktop.                                   |
| `./scripts/ci-desktop.sh desktop`                                               | `Vérifié` — build web puis ELF Tauri Linux release, localement et en CI #20. | Smoke métier natif interactif encore ouvert.                     |
| backup/restore                                                                  | `Vérifié` — 39 tables, 96 policies, hash identique, localement et en CI #20. | Pas encore restauré sur l’hébergement alpha.                     |
| magic link Supabase                                                             | `Vérifié` — `200/200/403/403/401`, localement et en CI #20.                  | Pas de rotation JWKS réelle ni environnement HTTPS.              |
| appel OpenAI applicatif                                                         | `Vérifié` comme erreur — `provider_quota`, 1 tentative, replay exact `502`.  | Aucune sortie réelle ; crédit/quota requis avant calibration.    |
| [Desktop CI #20](https://github.com/gneed49/ai-center/actions/runs/32876961859) | `Vérifié` — 5/5 jobs au commit `2162d60`.                                    | La certification pré-alpha manuelle reste ouverte.               |
| [OCI images #19](https://github.com/gneed49/ai-center/actions/runs/32876961945) | `Vérifié` — policy de packaging et images API/web, 3/3 jobs.                 | Images construites sans push ni déploiement.                     |

Les trois commandes Playwright ont été exécutées depuis la racine avec
`npm run test:e2e -w @ai-center/web -- --project=<projet>`. Aucune commande
Android, iOS ou mobile n'a été lancée.

## Matrice des preuves

| Gate                | ACP principaux         | Suite / scénario                 | Artefact attendu           | Statut                                                              |
| ------------------- | ---------------------- | -------------------------------- | -------------------------- | ------------------------------------------------------------------- |
| G0 Contrat          | ACP-001, 006, 070, 077 | Revue documentaire               | Spec et diagrammes         | `Vérifié` localement                                                |
| G1 Reproductibilité | ACP-070, 071, 076, 077 | CI desktop                       | logs et builds             | `Vérifié` localement et en CI de PR                                 |
| G2 Isolation        | ACP-029 à 032          | Tests DB/HTTP RLS                | rôles × opérations         | `Vérifié` localement                                                |
| G3 Fiabilité        | ACP-033 à 039          | crash/replay/faux providers      | traces expurgées           | `Vérifié` localement                                                |
| G4 Compiler         | ACP-010 à 021          | unités + intégration             | packs et exports           | `Vérifié` déterministe                                              |
| G5 Steward          | ACP-040 à 046          | DB déterministe puis corpus réel | boucle + matrice confusion | boucle `Vérifiée`, qualité ouverte                                  |
| G6 UI               | ACP-070, 071           | Playwright UF-01…UF-10           | traces desktop             | `Vérifié` mock + happy réel                                         |
| G7 GitHub           | ACP-050 à 056          | faux GitHub puis dépôt autorisé  | observations et preuve     | faux HTTP `Vérifié`, live ouvert                                    |
| G8 Certification    | ACP-030 à 077          | CI/pré-alpha complète            | rapport de certification   | CI de PR `Vérifiée` ; pré-alpha manuel `Présent mais non reproduit` |
| G9 Valeur           | ACP-072 à 075          | Campagne A/B                     | `report.json` expurgé      | `Futur`                                                             |
| G10 Dogfood         | Tous les P0            | Dix boucles réelles              | Registre de preuve         | `Futur`                                                             |
| G11 Équipe          | Tous                   | Alpha privée deux semaines       | Décision de promotion      | `Futur`                                                             |

## Scénarios E2E obligatoires

| Fichier cible                           | Parcours                          | Viewports de PR   | Gate pré-alpha supplémentaire |
| --------------------------------------- | --------------------------------- | ----------------- | ----------------------------- |
| `p0-context-loop.spec.ts`               | UF-01 à UF-06                     | Chromium 1440×900 | Chromium 1024×768 + Firefox   |
| `blocked-reject-revise.spec.ts`         | UF-02, UF-03, UF-07, UF-08        | Chromium 1440×900 | Firefox                       |
| `resilience-reload-idempotence.spec.ts` | UF-01, UF-02, UF-04, UF-06, UF-10 | Chromium 1440×900 | Chromium 1024×768             |
| `isolation-routing.spec.ts`             | UF-05, UF-10                      | Chromium 1440×900 | Firefox                       |
| `page-states.spec.ts`                   | Toutes les routes                 | Chromium 1440×900 | Chromium 1024×768             |
| `accessibility-desktop.spec.ts`         | UF-01 à UF-10                     | Chromium 1440×900 | Firefox + reduced motion      |
| `external-proof.spec.ts`                | UF-04, UF-09                      | Chromium 1440×900 | Firefox                       |

Les E2E UI de pull request interceptent l'API avec le faux serveur versionné
afin de couvrir les états et retries sans fournisseur. Les tests Rust/PostgreSQL
utilisent séparément une base Supabase isolée. Le happy path P0 a aussi été
rejoué contre cette base et l'API réelle en mode déterministe. Les traces
Playwright sont conservées au premier retry et aucun projet ou dépôt réel n'est
appelé en CI.

## Protocole de campagne réelle

### Échantillon

- exactement 3 projets autorisés, dont AI Center ;
- au moins 12 tâches de handoff réparties entre les projets ;
- au moins 60 paires : 20 `contradiction`, 20 `compatible`, 20 `ambiguous` ;
- 3 répétitions par cas et condition ; 5 seulement si une règle d'instabilité
  pré-enregistrée est déclenchée ;
- même modèle, même tâche, même contrat et mêmes versions de prompt/schéma pour
  les conditions `context_pack` et `full_dump`.

### Budget et confidentialité

| Enveloppe                           |     Maximum |
| ----------------------------------- | ----------: |
| Calibration de deux modèles au plus |      10 USD |
| Campagne principale                 |      70 USD |
| Reprises justifiées                 |      20 USD |
| **Total absolu**                    | **100 USD** |

La campagne utilise un projet et une clé OpenAI dédiés. Le manifest réel, les
sorties et les annotations restent sous `.run/alpha-context-proof/`, déjà
ignoré par Git. Le dépôt ne reçoit que des schémas, outils et rapports agrégés
expurgés. La CI n'a ni clé, ni job live, ni permission de créer une PR.

### Seuils de passage

| Métrique                                   |     Seuil |
| ------------------------------------------ | --------: |
| Sources valides et dans le bon projet      |     100 % |
| Présence des faits critiques               |     100 % |
| Rappel du contexte pertinent               |    ≥ 85 % |
| Éléments sélectionnés jugés non pertinents |    ≤ 20 % |
| Réduction médiane des tokens vs dump       |    ≥ 30 % |
| Handoffs sans reformulation majeure        |    ≥ 80 % |
| Préférence aveugle pour ContextPack        |    ≥ 70 % |
| Précision contradiction                    |    ≥ 80 % |
| Rappel contradiction                       |    ≥ 70 % |
| Sorties structurées valides après retries  |    ≥ 95 % |
| Coût total                                 | ≤ 100 USD |

Le second évaluateur couvre au moins 25 % des comparaisons. Les égalités
textuelles ne servent jamais d'oracle. Un seul seuil manqué place G9 en échec.

## Commandes de validation documentaire et CI

```bash
bash -n scripts/dev.sh scripts/ci-desktop.sh scripts/auth-magic-link-smoke.sh \
  scripts/baseline-alpha-upgrade-smoke.sh scripts/postgres-backup-restore-smoke.sh
python3 scripts/check-markdown-links.py README.md docs specs
python3 scripts/alpha-eval.py --help
./scripts/ci-desktop.sh secret-scan
./scripts/ci-desktop.sh quality
git diff --check
```

Les commandes d'intégration demandant Docker sont exécutées séparément :

```bash
npm run supabase -- start
npm run supabase -- db reset --local
AI_CENTER_ADMIN_DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:54322/postgres \
  AI_CENTER_RUNTIME_DATABASE_URL=postgresql://ai_center_runtime:<mot-de-passe-local>@127.0.0.1:54322/postgres \
  AI_CENTER_RUNTIME_DB_PASSWORD=<mot-de-passe-local> \
  AI_CENTER_AGENT_MODE=deterministic \
  ./scripts/ci-desktop.sh integration
AI_CENTER_ADMIN_DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:54322/postgres \
  ./scripts/ci-desktop.sh backup-restore
npm run supabase -- stop --no-backup
```

## Interdictions de promotion

La promotion est refusée en présence d'une fuite inter-scope, d'une perte de
connaissance, d'une duplication, d'une source inventée, d'une résolution sans
mutation, d'un pack stale utilisable, d'une écriture GitHub, d'un secret exposé,
d'un SSRF, d'une restauration non vérifiée, d'un P0 sans scénario d'erreur ou
d'une absence d'avantage mesuré face au dump complet.
