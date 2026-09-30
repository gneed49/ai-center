# Qualification T17 — 30 septembre 2026

**Statut : recette locale intégrée réussie ; publication et CI en cours.** Ce rapport
complète les [preuves intermédiaires du 24 septembre](validation-2026-09-24.md).
Il ne déclare ni accès à des comptes fournisseurs réels, ni déploiement, ni
fermeture de CC-G1 à G4. Les contenus et fournisseurs de recette sont **[FICTIF]**.

## Résultat métier implémenté

Un membre autorisé rattache explicitement une page Notion ou un ticket Linear
existant au projet. Les agents peuvent citer la lecture exacte et conserver sa
provenance dans un artefact, un contexte transmis et l'analyse de cohérence.
L'état et l'identifiant Linear sont transmis avec le texte. Une observation reste
une information externe : elle ne devient pas une règle d'entreprise confirmée.

Le bouton de vérification produit une nouvelle observation uniquement si l'état
métier change. Les consultations et la reprise d'un reçu ne déclenchent aucun
appel fournisseur. Le lecteur distingue l'historique, la lecture courante, une
lecture partielle et une source devenue indisponible.

## Preuves déjà obtenues

| Contrôle | Résultat et portée |
| --- | --- |
| Services et chaîne de contexte, avant les deux dernières corrections de revue | 11 tests PostgreSQL réussis : cinq société, un regroupement de publications et cinq services sources. Le scénario concurrent IA couvre chat, artefact et steward, avec sortie valide ou invalide. `.run/tool-sources-integration-seventh.log`. |
| Qualité, avant les dernières corrections de revue et le harnais navigateur | 192 tests web, 186 unités Rust, trois contrats synthétiques, 78 contrôles Python ; format, lint, Clippy et constructions réussis. Dix tests Rust DB ignorés dans cette phase sont qualifiés séparément. `.run/tool-sources-quality-2026-09-30.log`. |
| État Linear dans le contexte | Deux unités de projection et Clippy ciblé réussis après correction ; scénario vertical DB à rejouer. `.run/t17-state-projection-{unit,clippy}.log`. |
| Navigateur avec doubles API | 21/21 scénarios réussis en 50,2 s : Chromium desktop/compact et Firefox, passages à 390 px, clavier et titres longs. Aucun débordement ni violation axe sur les écrans audités. `/tmp/t17-browser-final-matrix.log`. |
| Web après corrections | Lint et construction réussis après la correction du focus ; 192 unités réussies avant cet ultime cadre de focus, couvert par les tests navigateur. `/tmp/t17-frontend-final-{lint,build}.log`, `/tmp/t17-frontend-tests.log`. |
| Unités web finales | 192/192 réussies après le cadre de focus. `.run/t17-final-web-units.log`. |
| Qualité finale après corrections | Exit 0 : 192 unités web, 188 unités Rust, trois contrats synthétiques, 78 contrôles Python ; format, lint, Clippy strict, liens et constructions web/serveur réussis. `.run/t17-final-quality.log`. |
| Navigateur avec vraie API et PostgreSQL | 1/1 réussi en 11,7 s : réponse Linear perdue/reçue récupérée, Notion V1 puis V2 partielle, historique exact, mobile et axe. Assertions serveur réussies : un appel Linear, six appels Notion, deux références et trois observations. Fournisseurs loopback. `.run/tool-sources-full-database-2026-09-30.log`. |
| Mise à niveau historique | 23 migrations appliquées jusqu'à `20260924004937_observed_source_dates.sql` ; propriétaires et données legacy conservés. Même journal intégré. |

## Revue et corrections

La [revue indépendante](review-2026-09-30.md) a identifié deux P2, corrigés puis
relus sans nouveau défaut confirmé : l'état Linear manquait aux consommateurs de
contexte ; le handoff verrouillait son projet avant l'ensemble ordonné des scopes.
Les nouvelles régressions vérifient un changement d'état seul, l'ancien snapshot
inchangé et la prise de verrous avec deux transactions réellement concurrentes.

La recette navigateur a aussi reproduit et corrigé une course entre POST et GET
du reçu après perte de réponse. La consultation attend la fin de l'envoi ; un reçu
terminé actualise les listes et retire l'erreur de transport devenue obsolète.
Le focus d'ouverture se pose une fois sur le titre sans être repris par le suivi.

## Correspondance des critères logiciels

| Critères | Preuves de comportement |
| --- | --- |
| ETC-001 à 004 | Lecteurs et localisateurs : résolutions UUID/URL/identifiant, titre Notion indépendant du nom de propriété, identité vérifiée, hôtes/redirects refusés, budgets de taille et de requêtes, échéance globale, couverture des blocs inconnus et transcriptions exclues. `work_tools/sources/{locator,reader_tests,models}.rs`. |
| ETC-005 et 006 | Services sur PostgreSQL : reçu rejoué sans HTTP, reprise après erreur sans perdre le snapshot, concurrence/head exact, isolation, révocation durant HTTP, propriétaire exigé à nouveau après lecture de rebind. `tests/work_tools/sources.rs`. |
| ETC-007 | Vertical `observed_sources_flow_from_chat_to_tickets_and_handoff_without_becoming_rules` : sources entrantes et publications, citations chat, artefact, tickets, pack et handoff ; état Linear modifié sans changement du corps. |
| ETC-008 | Le compilateur traite une fausse « règle obligatoire » externe comme facultative ; UUID inconnus refusés, couverture et confiance gardées dans les snapshots, métadonnées libres non propagées. Les instructions fournisseur séparent les données externes des règles. Ce sont des contrats synthétiques ; aucune résistance sémantique absolue d'un modèle réel n'est certifiée. |
| ETC-009 | Source incluse modifiée et autorité remplacée : anciens packs non courants, nouveaux contextes actualisés ; aucune réécriture d'artefact. Six barrières chat/artefact/steward avec sortie valide ou invalide vérifient l'absence de texte final enregistré après rotation/réattestation. |
| ETC-010 | Regroupement contigu des reçus : état inchangé stable, A→B→A distinct, ancien reçu sans attestation exclu, nouvelle vérification sans rétro-écriture de l'historique. `tests/company_context/ticket_provenance.rs`. |
| ETC-011 | 21 scénarios navigateur avec doubles API, puis scénario réel API/DB et compteurs HTTP ; historique, lecture partielle, indisponibilité, réponse perdue, changement de droits, clavier et mobile. |
| ETC-012 | Exports et effacement gouvernés avec projets voisins conservés ; rôle runtime sans bypass RLS, inventaires tables/colonnes/FKs, quotas et trois places réseau partagées, cooldown partagé entre projets/acteurs. Tests société/sources, vérificateurs SQL et recette de restauration. |

Les chemins de code sont relatifs à `apps/server/src/` ou `apps/server/` pour
les tests. Cette matrice indique où trouver les preuves ; la clôture dépend aussi
du résultat de la recette intégrée et de la CI ci-dessous.

## Recette intégrée finale

Le lifecycle suivant termine **avec code 0**, nettoyage gardé compris :

```bash
./scripts/integration-stack.sh run integration tls-smoke backup-restore \
  auth-smoke auth-browser source-browser ticket-browser real-e2e
```

Journal privé : `.run/t17-final-integration.log`. Le client PostgreSQL et le socket
Podman sont ceux du poste local ; aucun secret ou contenu de `.env` n'est chargé.

- **94 tests PostgreSQL réussis**, dont 42 société, cinq artefacts et 19 outils ;
  les deux harnais navigateur ignorés dans la suite Rust sont exécutés ensuite.
  **32 assertions pgTAP** et mise à niveau des 23 migrations réussies. Les deux
  régressions de revue, y compris les six cas IA concurrents, passent.
- **TLS réussi** : CA de test acceptée, CA inconnue et nom d'hôte incorrect
  refusés. **Restauration réussie : 60 tables, 143 politiques**, données comparées.
- **Auth Supabase local API et navigateur réussis** : deux comptes, société,
  invitation, lecture seule, promotion et retrait ; ancien JWT refusé après
  retrait. Aucun SMTP externe n'est testé.
- **Sources : 1/1 en 12,1 s**, avec assertions de position du titre et du fil de
  navigation avant capture. Les compteurs fournisseur et les snapshots DB passent.
- **Tickets : 1/1 en 22 s** : deux Linear puis trois GitHub, cinq reçus et
  conservation des versions. **Quatre parcours métier : 4/4 en 45,2 s**, couvrant
  génération, réponse perdue, révision/validation, sources/graphe et relais technique.

Les nouvelles captures `/tmp/t17-real-linear-desktop.png` et
`/tmp/t17-real-notion-mobile.png` portent des données fictives. Les tests vérifient
l'absence de débordement, de violation axe sur ces écrans et d'erreur inattendue.
La remise à zéro du défilement évite d'interpréter un fil contextuel volontairement
défilé derrière le header fixe comme un défaut de page.

Le premier passage complet a trouvé deux assertions de test restées sur les
versions précédentes du prompt/compilateur. Elles attendent désormais les versions
réellement introduites par T17 ; les contrôles de cycle de vie et de fraîcheur
restent inchangés. Les deux suites passent dans le lifecycle final.

Le harnais sources utilise Auth local et les vraies politiques RLS. Il ne prouve
pas le parcours JWT, couvert par les recettes Auth distinctes. Son lecteur
loopback n'existe qu'en compilation debug ; aucune requête, variable d'environnement
ou configuration enregistrée ne peut activer ce lecteur en production.

Un conteneur de stockage orphelin de la seule stack jetable empêchait sa reprise
après interruption. Son identité a été vérifiée avant suppression ciblée. Les
autres stacks et leurs volumes n'ont pas été modifiés.

Après le lifecycle final, aucun conteneur ni volume DB/storage de cette stack ne
subsiste. Un volume historique `edge_runtime` sans label, créé le 21 septembre,
est hors du nettoyage gardé et reste conservé ; aucune purge générale effectuée.

La revue indépendante a relu les corrections ; la publication et les contrôles
distants doivent encore identifier le commit exact. Aucun résultat CI antérieur
n'est utilisé pour qualifier ces modifications locales.

Les métadonnées, fichiers et publications GitHub dans le catalogue commun des
agents restent le [lot T18](../github-context-consumers/spec.md). Leur présence
dans le graphe ne vaut pas preuve du parcours PM vers développement.
