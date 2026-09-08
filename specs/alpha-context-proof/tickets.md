# Tickets de reprise — Alpha Context Proof

Le plan directeur fourni par le propriétaire le 4 septembre 2026 est comparé
à la branche existante `feat/alpha-context-proof`, baseline `8a6e0da`.
La PR de consolidation reste [#1](https://github.com/gneed49/ai-center/pull/1).
Le périmètre et les critères de preuve sont ceux de [spec.md](spec.md) et
[validation.md](validation.md). L'instruction d'implémenter ce plan autorise
la reprise du périmètre existant ; les gates de promotion restent distinctes.

Les tickets ci-dessous forment un graphe. Un ticket dépendant commence après
intégration de ses prérequis. Les branches de travail sont fusionnées dans la
branche de consolidation après leurs validations ciblées.

| Ticket  | Objet                                                          | Dépendances                  | Statut                                                                           |
| ------- | -------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------------------------- |
| ACP-T01 | Intégration locale isolée                                      | Aucune                       | Intégré et vérifié localement                                                    |
| ACP-T02 | Invalidation ciblée des versions sources                       | Aucune                       | Intégré et vérifié localement                                                    |
| ACP-T03 | Références GitHub repository, PR, commit et checks             | Aucune                       | Intégré ; contrats simulés et PostgreSQL vérifiés                                |
| ACP-T04 | Certification des parcours desktop manquants                   | T01, T02, T03, T07           | Intégré ; 126 tests UI simulés historiques, recette E2E propriétaire ouverte     |
| ACP-T05 | Revue du protocole d'évaluation et gates de promotion          | Aucune                       | Technique vérifiée ; gates live ouvertes                                         |
| ACP-T06 | Revue Standards/Spec, corrections et preuves courantes         | T01–T05, T07–T10             | Correctifs et revues vérifiés ; gates externes ouvertes                          |
| ACP-T07 | Évaluation structurée de couverture                            | T02                          | Intégré et vérifié localement                                                    |
| ACP-T08 | Traçabilité du travail externe                                 | T03, T07                     | Intégré et vérifié localement                                                    |
| ACP-T09 | Dépendances web                                                | Aucune                       | Intégré ; audit npm sans avis                                                    |
| ACP-T10 | Smoke métier du client Tauri Linux                             | T01                          | Code/build vérifiés historiquement ; recette native propriétaire ouverte         |
| ACP-T11 | Upgrade de la baseline jusqu'à toutes les migrations courantes | T01, connexions personnelles | Intégré ; upgrade des 5 migrations vérifié localement le 8 septembre             |
| ACP-T12 | Reprise d'un message liée à son contenu original               | T06, connexions personnelles | Intégré ; régression incluse dans les 7 tests DB ciblés réussis le 8 septembre   |
| ACP-T13 | Transport borné du harness de campagne réelle                  | T05                          | Intégré ; 41 tests Alpha simulés réussis le 8 septembre, campagne réelle ouverte |

Point de contrôle intermédiaire du 5 septembre 2026 : T01, T02, T03 et T07 sont réunis au
commit `37921d8`. Le cycle isolé complet passe : 32 pgTAP, neuf tests métier
PostgreSQL, restauration de 39 tables et 96 policies, magic link local et
parcours Chromium full-stack jusqu'au handoff rechargé. Le nettoyage de la
stack est confirmé. Ces preuves utilisent le moteur déterministe et des faux
fournisseurs ; elles ne ferment pas les gates de valeur, GitHub live ou alpha
privée. Le build Tauri Linux passe ; son smoke métier reste distinct.

Point de contrôle final du socle : `a9b3140`. Les constats de revue sont
corrigés ; 89 tests Rust, 27 tests évaluation, 11 tests métier et 32 pgTAP
passent. Backup/auth du schéma corrigé et 126 tests desktop simulés sont
qualifiés dans le [rapport daté](review-2026-09-05.md). Le smoke natif et les
gates réels restent ouverts. La [PR #1](https://github.com/gneed49/ai-center/pull/1)
porte la consolidation suivie par l'[issue #2](https://github.com/gneed49/ai-center/issues/2).

## ACP-T01 — Intégration locale isolée

**Exigences :** phase 1, ACP-070, ACP-076, ACP-077.

Le script d'intégration courant lance un reset dans le projet Supabase de
développement `AICenter`. Construire un environnement de test avec identité,
configuration, ports et cycle de vie propres. Toute opération destructive
doit vérifier sa cible avant exécution. Adapter CI et documentation.

**Acceptation :** stack jetable complète, migration baseline→alpha, RLS,
tests PostgreSQL, auth et parcours réel ; teardown de la seule stack de test.
Un test de garde démontre qu'une cible de développement est refusée avant
reset. Aucun secret réel ni base de développement nécessaire.

## ACP-T02 — Invalidation ciblée

**Exigences :** ACP-017, ACP-041, ACP-044, ACP-045, ACP-046.

`invalidate_dependent_projections` ignore actuellement les versions modifiées.
Propager l'obsolescence uniquement aux packs, livrables, preuves et couvertures
qui en dépendent. Conserver le fallback global seulement quand la provenance
est insuffisante, avec cause explicite et audit. Respecter les contraintes de
handoff sur la graph version courante et l'immutabilité historique.

**Acceptation :** graphe à deux branches, révision d'une branche, projections
de l'autre conservées ; résolution atomique, fallback tracé, refus de génération
stale et recompilation vérifiés sur PostgreSQL.

## ACP-T03 — Périmètre GitHub et rate limits

**Exigences :** phase 7, ACP-050 à ACP-056.

Étendre le chemin d'import actuellement limité aux URLs de PR aux repositories
et commits. Les checks restent observés sur le SHA pertinent. Préserver
allowlist, URLs canoniques, API officielle, absence de redirects, historique,
validation humaine et compatibilité des références PR existantes. Classifier
une limite GitHub HTTP 403 avec ses headers comme transitoire : elle ne doit
pas être traitée comme perte d'accès durable.

**Acceptation :** contrats faux HTTP pour chaque type, URLs invalides/SSRF,
ETag, changement de SHA, 403 rate limit, perte d'accès, retries et aucune écriture
GitHub ; parcours d'import cohérent dans l'interface.

## ACP-T04 — Certification desktop

**Exigences :** ACP-020, ACP-034, ACP-070, ACP-071 ; phases 6 et 8.

Compléter les lacunes constatées dans les scénarios existants : offline et
reconnexion avec saisie préservée, réponse perdue et replay, résolution en conflit,
parcours complet plan/preuve/couverture/historique, erreurs par route et
ressources hors scope. Les scénarios sont intégrés ; les preuves simulées et
full-stack historiques restent identifiées. Selon la consigne du 7 septembre,
la recette navigateur/E2E et le contrôle exploratoire de la version courante
sont réalisés par le propriétaire ; ils ne sont pas présumés réussis.

Vérifier également le changement d'utilisateur dans le même onglet et entre
onglets : les requêtes en cours, caches et sélections de workspace de l'ancien
acteur ne doivent jamais apparaître sous la nouvelle identité. Le constat
initial du QueryClient global sans reset lié à l'identité a été corrigé dans
la consolidation ; les preuves datées restent celles du rapport du 5 septembre.

**Acceptation :** suites significatives sur Chromium 1440×900 et 1024×768,
Firefox desktop ; zéro erreur inattendue, duplication, fuite ou violation axe
critical/serious. La preuve native Tauri est distincte du build Linux.

## ACP-T05 — Évaluation et promotion

**Exigences :** ACP-072 à ACP-076 ; phases 9 à 11.

Vérifier les contrôles exécutables du harness : budget ferme, calibration,
sources autorisées, corpus, aveugle, répétitions, annotations et seuils. Corriger
les écarts reproductibles sans inventer de résultats ou de consentements.
Conserver un registre explicite des prérequis externes et durées d'exploitation.

**Acceptation technique :** tests offline des refus et calculs, manifests et
commandes opérationnelles utilisables. **Acceptation live :** corpus autorisé,
fournisseurs configurés, évaluations humaines et périodes d'usage réalisées.
L'acceptation technique ne ferme pas les gates G9 à G11.

## ACP-T06 — Intégration et revue

**Exigences :** ACP-006 et toutes les exigences touchées.

Revue en deux axes Standards/Spec de `origin/main...HEAD`, corrections sur
worktree dédié, validations adaptées et documentation datée. Mettre à jour la
PR après vérification du contenu destiné au dépôt public. Nettoyer les worktrees
d'implémentation après fusion. La release reste conditionnée aux gates du plan.

**Acceptation :** écarts techniques corrigés, résultats localisables, statut
exact des gates restant ouverts, branche unique et PR relisible.

**Corrections de revue du 2026-09-05 (baseline `90ca7588`) :**

- Révision datée de l'ADR 0003 ; ouverture transactionnelle commune aux services,
  steward et références en préservant leurs vérifications ; noms Rust GitHub
  génériques sans modifier les routes, JSON ni clés d'opération historiques.
- Observations append-only dédupliquées seulement contre le dernier état sous
  verrou, ordonnées par événement appliqué. Les cycles de SHA, checks et perte
  d'accès conservent le retour à un ancien contenu ; les retries restent uniques.
- Bail renouvelé pendant chaque attente fournisseur, génération UUID distincte
  du délai ; annulation du fournisseur à la perte du bail, puis model_run terminal.
- Feature Brief et ProductReadyGate sérialisés avec la révision du graphe ; gate
  devenu ancien refusé avant publication et brief publié invalidé par la révision
  suivante.
- Évaluation v1.2 : export JSON et empreinte du contenu vérifiés contre les
  versions du snapshot ; annotations humaines préalables liées aux sources ;
  compteurs d'entrée recalculés depuis la sélection réelle. Le livrable est
  évalué humainement en aveugle, sans oracle textuel ni auto-déclaration d'IDs.

Régressions ajoutées aux suites déjà collectées : publication du brief dans
`context_pack_concurrency`, fournisseur de plus de 30 secondes/perte de bail
avec run terminal dans `model_run_lifecycle`, cycles GitHub dans le contrat DB
ignoré explicite existant, et intégrité/annotations dans les tests offline éval.
La migration est générée depuis le schéma déclaratif ; aucune observation
historique n'est réécrite. Les preuves courantes et gates externes restent
consignées séparément dans le registre de validation.

## Écarts additionnels confirmés pendant la lecture du code

Ces tickets complètent le graphe initial. T07 attend T02 ; T08 attend T03 et
T07. T04 commence par les parcours indépendants de T08 ; la certification
consolidée de T06 attend leur intégration à tous les deux.

### ACP-T07 — Évaluation structurée de couverture

**Exigences :** phase 4, ACP-015, ACP-018, ACP-035 à ACP-039, ACP-056.

Introduire une opération de modèle distincte de la génération du plan pour
évaluer le mapping des exigences et les lacunes, exclusivement depuis le pack
et le plan produits. Valider les UUID, l'appartenance au pack, l'exhaustivité et
l'unicité des exigences. Persister un model run propre avec versions du contrat,
usage et statut. Garder les appels réseau hors transaction, le contrôle optimiste
et la commande idempotente. Une appréciation du modèle ne valide jamais une
preuve : la couverture effective dépend des preuves humaines courantes.

**Acceptation :** faux provider validant les cinq opérations structurées,
rejet des sources inventées/dupliquées/omises, échec explicite sans livrable
partiel, traçabilité distincte du run de plan et du run de couverture ; aucun
contenu spécifique à Credits v2 dans le moteur réel.

### ACP-T08 — Traçabilité du travail externe

**Exigences :** phase 7, ACP-020, ACP-021, ACP-050, ACP-054 à ACP-056.

Relier explicitement les références et preuves au ContextPack transmis en
réutilisant les enveloppes existantes `tasks`, `executions`, `execution_events`
et `artifacts`, ainsi que les arêtes `tracked_by`. Les états décrivent le travail
observé dans l'outil externe ; aucun runner ni action GitHub en écriture.
Conserver les imports historiques en lecture et fournir le lien explicite dans
le parcours de preuve. Vérifier toutes les relations au workspace et au projet.

**Acceptation :** export/handoff → référence externe → enveloppe et artefact →
preuve candidate → validation humaine ; reload et retry conservent la même
chaîne sans duplication. Les refreshes ajoutent des événements d'observation
et préservent l'historique en cas de changement de SHA ou de perte d'accès.

### ACP-T09 — Dépendances web

**Exigences :** phase 1, audit des dépendances.

L'audit npm courant signale `fast-uri` 3.1.5 et `qs` 6.15.3. Mettre à jour les
résolutions compatibles, vérifier le diff du lockfile et relancer audit, build
et validations web. Ne pas déplacer des dépendances pour masquer les avis.
T06 attend également T09. Ce ticket est indépendant des changements métier.

### ACP-T10 — Smoke métier du client Tauri Linux

**Exigences :** ACP-071 ; phases 1, 8 et 10.

Le smoke WebDriver est intégré : son contrat prévoit le vrai client Linux
avec un profil et un affichage jetables, puis des actions métier contre la
stack déterministe isolée T01. Son overlay est limité aux origines loopback
de test, sans élargir la CSP produit ni toucher au mobile gelé. Les pilotes
et l'affichage virtuel restent des dépendances de validation.

Le code/build possède les preuves historiques du rapport du 5 septembre ;
l'exécution métier était alors bloquée par l'environnement. Selon la consigne
du 7 septembre, la recette native reste désormais à réaliser par le propriétaire.
Aucun succès natif de la version courante n'est revendiqué.

**Acceptation :** application native chargée, projet et connaissances persistés,
pack/handoff lisibles après reload, capture et assertions de rendu ; nettoyage
des processus et du profil. Le workflow pré-alpha contient ce smoke en plus
du build ; son exécution relève de la recette propriétaire. Un succès local
ne certifie pas le déploiement privé HTTPS.
T10 dépend de T01 et rejoint les prérequis de T06.

## Reprise du 8 septembre — écarts logiciels confirmés

Point de départ : `a22335cd1849c9b8c5f317eb94905fa3489a51a8`. La livraison
des connexions personnelles est intégrée et ses preuves figurent dans le
[rapport du 7 septembre](../provider-connections/validation-2026-09-07.md).
Le propriétaire réalise les E2E et les essais avec ses comptes. Les corrections
ci-dessous utilisent uniquement des tests unitaires, des contrats simulés et
une base PostgreSQL jetable. Elles ne ferment pas les gates de valeur réelle.

**Intégration du 8 septembre :** T11/T12 sont livrés par `34af226` et T13 par
`27e3145`, réunis sur la branche de consolidation au point `b20d36f`. Les
preuves ci-dessous sont des résultats locaux sur leurs commits d'implémentation,
pas une nouvelle exécution lors de la revue documentaire. L'upgrade des cinq
migrations et sept intégrations DB ciblées passent ; le harness Alpha compte
41 tests réussis. Aucun E2E, smoke métier natif ou fournisseur réel n'a été
exécuté pendant cette reprise. Les gates de corpus, campagne, usage propriétaire,
alpha privée et promotion restent ouverts.

### ACP-T11 — Upgrade jusqu'au schéma courant

**Exigences :** phase 1 (baseline→alpha), phase 8 (migration/reprise), ACP-070.

**Constat à l'ouverture :** `scripts/baseline-alpha-upgrade-smoke.sh` appliquait
seulement les migrations du 18 et du 25 août avant de conclure, sans celles de
septembre ni les connexions personnelles.

**Acceptation :** créer la baseline et ses fixtures legacy dans la base
strictement jetable existante, appliquer chaque migration suivante en ordre
déterministe et vérifier l'état courant ainsi que la conservation des owners
et des données legacy. Un oubli ou un échec de migration ne peut pas produire
un succès. Couvrir la découverte/ordre/refus par unités et reproduire l'upgrade
complet sur PostgreSQL réel. Préserver les gardes de cible et le nettoyage.
Ne pas modifier les migrations historiques pour faire passer le test.

**Livré et vérifié localement le 8 septembre (`34af226`) :** découverte ordonnée
des migrations, refus explicites, arrêt sur erreur et assertions après upgrade
sur données legacy. Les cinq migrations actuelles passent dans PostgreSQL réel,
avec sept unités du plan de migration et les 17 gardes de stack. La restauration
SQL complémentaire conserve 41 tables et 98 policies ; la réutilisation d'une
clé déchiffrée après restauration avec sa clé maîtresse séparée n'a pas été
reproduite. Aucune restauration sur l'hébergement alpha n'est revendiquée.

### ACP-T12 — Contenu stable d'un message repris

**Exigences :** phase 3, ACP-003, ACP-033, ACP-034 et ADR 0003.

**Constat à l'ouverture :** après un échec fournisseur, une nouvelle commande
avec le même `client_message_id` mais un contenu différent conservait l'ancien
message tout en transmettant le nouveau texte au moteur. Après succès, elle
pouvait rejouer l'ancienne réponse sans signaler le changement de contenu.

**Acceptation :** une identité de message existante reste liée au même contenu
normalisé dans sa session. Un autre contenu renvoie `409` avant tout nouvel
appel fournisseur, run ou proposition. Une reprise légitime avec le même
contenu, une nouvelle commande et la même identité reste possible après un
échec, sans duplication. Tester après échec et après succès, avec assertions
sur les messages, runs et appels simulés dans PostgreSQL réel.

**Livré et vérifié localement le 8 septembre (`34af226`) :** comparaison du
contenu normalisé avant reprise, `409` sans nouvel appel/run/proposition en cas
de changement, reprise identique sans duplication après échec ou succès. La
régression a échoué avant le correctif puis réussi ; elle figure dans les sept
intégrations DB ciblées réussies (idempotence 2, cycle modèle 3, connexions/messages
2). Cette preuve HTTP/PostgreSQL ne constitue pas une recette E2E.

### ACP-T13 — Transport de campagne sans redirection et borné

**Exigences :** phases 3, 8 et 9 ; ACP-036, ACP-039, ACP-074, ACP-076.

**Constat à l'ouverture :** `ResponsesClient` dans `scripts/alpha-live-eval.py`
utilisait l'opener urllib par défaut, susceptible de suivre une redirection avec
l'en-tête Authorization, puis lisait le corps sans borne. Le défaut concernait
le harness autonome, pas les transports API Rust déjà corrigés.

**Acceptation :** refuser les redirections avant tout second appel, garder
l'origine officielle fixe, borner la taille et le temps de réception même
pour un corps lent, et exposer uniquement des erreurs nettoyées. Les tests
offline ou HTTP loopback utilisent des clés fictives et couvrent redirection,
corps excessif, interruption/timeout, JSON invalide et réponse valide. Aucun
appel OpenAI réel. Conserver le contrôle de budget, le contrat de campagne et
les reprises existants.

**Livré et vérifié localement le 8 septembre (`27e3145`) :** redirections refusées,
origine fixe, corps bornés, délai global et erreurs expurgées. Les 41 tests Alpha
passent, dont 14 nouvelles régressions transport avec données fictives. Aucun
appel OpenAI réel ; calibration, corpus autorisé, notation humaine et campagne
comparative restent à réaliser.
