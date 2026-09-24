# Tickets — Company Context V1

> Réalisation autonome en cours sur `feat/company-context-v1` ; aucune ouverture de production déclarée.  
> Dernière mise à jour : 2026-09-22  
> Contrat : [spec.md](spec.md) ; séquence : [plan.md](plan.md)

Ce registre formalise des travaux autorisés. `pending` signifie non encore
accepté sur cette spécification ; il ne nie pas l'existence des briques Alpha.
Un ticket passe à `in_progress`, puis `review`, puis `done` uniquement avec
commit et preuves localisables. Un accès externe manquant est noté comme
dépendance explicite ; il ne ferme pas le ticket par assimilation à un mock.

Les responsables ci-dessous sont des rôles de réalisation à attribuer par le
coordinateur ; ils n'introduisent pas de nouvelles attentes d'approbation.

| Ticket | Lot et responsable                                         | Exigences                        | Dépendances                                | Statut      |
| ------ | ---------------------------------------------------------- | -------------------------------- | ------------------------------------------ | ----------- |
| CC-T01 | Base, contrats et amendements — coordination/produit       | Contrat V1 et sources            | Aucune                                     | review       |
| CC-T02 | Domaine partagé et migrations — backend/données            | CC-001,003,010,020,030           | T01                                        | review       |
| CC-T03 | Société, équipe et projets — backend + web                 | CC-001 à 005                     | T02                                        | review       |
| CC-T04 | Agents, conversations et contexte — backend + web          | CC-010 à 014                     | T02                                        | review       |
| CC-T05 | Artefacts, bibliothèque et destinations — backend + web    | CC-020 à 024                     | T02                                        | review       |
| CC-T06 | Transport et jobs de destination — backend                 | CC-025 à 027                     | T02, T05                                   | review       |
| CC-T07 | Adapters Notion/Linear/GitHub — intégrations               | CC-025 à 027,043                 | T06                                        | review       |
| CC-T08 | Graphes projet et société — backend/données                | CC-030 à 034                     | T02                                        | review       |
| CC-T09 | Steward et écarts sourcés — backend                        | CC-040 à 045                     | T04, T05, T07, T08                         | review       |
| CC-T10 | Parcours web et graphe interactif — frontend               | CC-004,022,033,044,053           | T03, T04, T05, T08 ; T09 pour inbox finale | review       |
| CC-T11 | Reprise, quotas, arrêt et usage — backend + frontend       | CC-050 à 053                     | T02 ; intégration avec T04/T06/T09         | review       |
| CC-T12 | Données et préparation exploitation — backend/Ops          | CC-054 à 056                     | T03, T05, T06, T11                         | review       |
| CC-T13 | Candidate intégrée, revue et publication — QA/coordination | CC-057, CC-G1                    | T03 à T12, T16 et T17                       | in_progress  |
| CC-T14 | Qualification réelle et ouverture contrôlée — Ops/produit  | CC-014,026,027,055,056, CC-G2/G3 | T13 + accès/cible/budget identifiés        | pending      |
| CC-T15 | Pilote et décision V1 — produit/pilotes                    | CC-G4                            | T14 + temps et participants réels          | pending      |
| CC-T16 | Tickets distincts vers les outils — backend + web          | CC-020,026,028,031,050,054        | T05, T06, T07 ; local réussi, CI à suivre   | in_progress  |
| CC-T17 | Contexte Notion/Linear existant — backend + web            | CC-012,020,023,025,027,029,031,054 | T04 à T09 ; après T16                       | pending      |

### Amendement de granularité — CC-T16

L’audit de publication a confirmé qu’un artefact contenant plusieurs tickets
créait une seule issue contenant tout le document. Le nouveau contrat
[Tickets distincts](../ticket-publication/spec.md) et son
[plan](../ticket-publication/plan.md) exigent une demande de publication par
entrée sélectionnée, avec états/liens individuels et filiation version/index.
Notion reste documentaire ; aucun tableau de tâches interne n’est ajouté.

Le plan est préparé pendant la recette, **sans code ni SQL avant le signal de fin
du gel donné par l’intégrateur**. Le lot doit prouver sélection N → N issues,
inscription atomique sous quotas, réponse perdue/réconciliation sans doublon par
entrée, autorisations, confirmation des créations entre versions, exports et
effacement. Les preuves documentaires précédentes ne satisfont pas ce critère.

CC-T13/CC-G1 dépendent également de CC-T16 ; ils ne sont pas clos par la recette
du seul comportement documentaire existant. Les comptes/outils réels restent
qualifiés séparément par CC-T14.

### Amendement de contexte entrant — CC-T17

Les connecteurs permettent aujourd'hui de relire leurs propres publications,
mais pas de rattacher une page Notion ou une issue Linear existante. Le graphe
et le steward disposent d'observations que le contexte PM/génération ne sélectionne
pas encore directement. Cet écart P1 empêche de travailler à partir des outils
déjà utilisés par l'entreprise.

La [spécification ciblée](../existing-tool-context/spec.md) et son
[plan](../existing-tool-context/plan.md) couvrent le rattachement volontaire d'un
objet, les observations immuables/refresh et leur chaîne complète de provenance
vers agents, artefacts et handoffs. Deux petites tables dédiées évitent d'affaiblir
les contraintes GitHub ou les reçus de publication. Connexions, quotas, commandes,
graphe et steward existants sont réutilisés ; pas de synchronisation globale.

**État : conception seulement**, aucun code ou SQL dans cette passe. Le lot se
réalise après T16 et participe à CC-T13/CC-G1 ; la recette précédente ne le ferme
pas. Fin : ETC-001 à ETC-012 avec fixtures marquées, recette isolée, preuves de
révocation/couverture/confiance et export/effacement. La qualification des objets
réels reste une preuve distincte de CC-T14.

## CC-T01 — Base, contrats et amendements

Identifier branche/HEAD, diffs existants, provenance des ajouts et stratégie de
consolidation. Décliner les amendements de la spec dans les décisions produit
actives sans modifier les sources historiques. Figer noms, opérations, erreurs
et contrats partagés avant travail concurrent.

**Fin :** baseline traçable, contrats communiqués aux lots, nouvelles limites
explicites et aucun ancien gate présenté comme clos. L'autorisation de réaliser
est déjà acquise ; ce ticket ne produit pas une nouvelle pause « valider le plan ».

## CC-T02 — Domaine partagé et migrations

Poser les structures nécessaires société/projets/permissions, scope/profil,
artefact/version/destination, relation/provenance et jobs. Réutiliser les
entités existantes, produire migrations additives, contraintes et RLS, backfill
des projets/session/livrables legacy sans perte. Un responsable unique intègre
les changements du schéma partagé.

**Fin :** migration baseline → courant sur stack isolée, deux sociétés et
projets synthétiques, objets legacy retrouvés, routes de lecture minimales et
matrice d'accès testée. Une table vide sans service/contrat ne suffit pas.

## CC-T03 — Société, équipe et projets

Livrer setup guidé, membres/invitations/rôles/retraits, création/renommage/archive
de projet et sélection société/projet. Le partage des projets avec les membres
de la société est affiché clairement ; aucune restriction
privée par projet promise par l'UI sans contrôle correspondant. Retrait recontrôlé sur
requêtes et jobs, cache purgé au changement de société.

**Fin :** CC-U01 sur API/DB, invitation et révocation testées avec session
existante, viewer refusé en écriture, société A ne lit pas B. Tests email locaux
distincts du SMTP réel réservé à T14.

## CC-T04 — Agents, conversations et contexte

Configurer profils société et PM/Commercial/Lead technique/Développeur par
projet ; distinguer métier d'agent et rôle d'accès. Conversations durables,
scope visible, questions/brainstorming/propositions ; contexte composé de
versions autorisées avec provenance et obligations conservées. Aucun agent
Développeur n'exécute ou ne modifie du code distant.

**Fin :** une conversation de chaque profil persiste, les scopes restent
isolés, le relais PM → lead consomme les bonnes sources, les refus et retries
sont explicites. La génération live du fournisseur reste une preuve T14.

## CC-T05 — Artefacts, bibliothèque et destinations

Kickoff, spécifications, tickets produit, plans et tickets techniques ; édition
de brouillon, validation, versions, comparaison et historique. Bibliothèque
complète/recherche et exports. Réglages des destinations par société/type et
projet, remplacement ponctuel ; stockage interne utilisable sans connecteur.

**Fin :** deux révisions liées aux sources restent consultables, au-delà de la
première page ; JSON/Markdown correspond à la version sélectionnée ; changer
de destination ne réécrit pas l'historique. Aucun succès externe simulé.

## CC-T06 — Transport et jobs de destination

Définir capacités, destinations autorisées et connexions sécurisées ; jobs de
publication/refresh durables, tentative hors transaction, idempotence,
réconciliation, erreurs, permissions réévaluées et audit. La publication part
d'une action explicite sur un artefact validé ; le steward ne publie pas.

**Fin :** faux adapter démontre réponse perdue sans duplication, conflit sans
écrasement, révocation avant job, capacité non prise en charge expliquée et
absence de secret dans réponses/logs. Contrat stable pour trois adapters.

## CC-T07 — Notion, Linear et GitHub

Implémenter contre les APIs officielles leurs capacités utiles : pages Notion,
issues Linear, issues/références GitHub, lecture/refresh et conflits. Lecture
GitHub ciblée et bornée de fichiers/versions autorisés pour l'analyse, sans
exécution de code. Les tests des trois adapters sont indépendants.

**Fin :** chaque adapter passe création/lecture/reprise/conflit/refus et limites
avec faux HTTP, URLs canoniques et versions traçables. L'interface annonce les
capacités réellement disponibles. Comptes/destinations réels qualifiés en T14,
sans prétendre que les contrats simulés prouvent un accès réel.

## CC-T08 — Graphes projet et société

Produire entités/arêtes sourcées, requêtes bornées et fédération au sein de la
société. Filtrer avant agrégation et sélection du contexte ; ne pas révéler
les projets restreints dans compteurs, noms, labels ou synthèses. Maintenir
filiation et invalidation inter-projets ciblée.

**Fin :** deux graphes projet se retrouvent dans la vue société selon les droits,
les arêtes pointent vers des versions existantes, aucune arête inter-sociétés,
une révision ne périme que ses dépendances. Fixtures et contrats API disponibles
pour T10 ; aucune image statique à la place du graphe de données.

## CC-T09 — Steward et écarts sourcés

Déclencher analyses sur événements de connaissance/artefact/observation. Réutiliser
outbox/baux/retry ; traiter contradictions projet/entreprise et écarts entre
spec, ticket et sources GitHub. Exposer inconnu/ambigu/incomplet, faits observés,
confiance et sources ; décisions humaines distinctes et révisions atomiques.

**Fin :** corpus `[FICTIF]` contient contradiction réelle, compatibilité,
ambiguïté, métadonnées seules, fichier manquant et source obsolète. Aucun verdict
confirmé sans preuve ; jobs concurrent/repris dédupliqués ; accept/reject/resolve
audités et pas de mutation automatique dans un outil externe.

## CC-T10 — Parcours web et graphe interactif

Assembler navigation société/projets, agents/conversations, bibliothèque et
destinations, vues graphiques, panneau de provenance et inbox. Vocabulaire métier,
onboarding, parcours clavier, listes alternatives, recherche/filtres et états
vide/chargement/erreur. Afficher identité et états réels.

**Fin :** CC-U01 à CC-U07 sont praticables via API/DB réelles, screenshots de
données fictives et exploration clavier. Le graphe ouvre les objets et explique
les liens ; aucun bouton décoratif, faux membre ou faux état « moteur actif ».

## CC-T11 — Reprise, quotas, arrêt et usage

Conserver l'identité du travail lors de réponses perdues ; suivre les opérations
longues derrière le proxy et restaurer saisies/état après reload. Limiter
concurrence/appels, arrêter tous chemins IA actifs dont profils personnels et
jobs différés ; mesurer usage sans confondre coût absent et zéro.

**Fin :** panne/réponse perdue/retry/refresh causent un seul traitement logique,
plafond appliqué avant appel, arrêt complet testé, coût inconnu et service
indisponible explicites. Instrumentation sans prompts, réponses ni secrets.

## CC-T12 — Données et préparation exploitation

Exporter et supprimer une société/projet avec procédure contrôlée ; traiter
références, historiques, blobs éventuels, droits, rétention et sauvegardes.
Préparer images/configuration/migrations, readiness, Auth/SMTP, alertes, backups,
restauration séparée, clés récupérables et retour arrière compatible.

**Fin :** exercice local isolé sur fixtures sans effet sur autre société,
contrôles et runbook reproductibles. La configuration est prête à recevoir
les secrets hors dépôt ; aucune base distante créée, offre achetée ou alerte
réelle présumée par existence des scripts. Preuves de cible en T14.

## CC-T13 — Candidate intégrée, revue et publication

Relier chaque exigence aux lots et preuves ; intégrer et corriger les revues
Standards/Spec. Rejouer les suites affectées puis le parcours intégré, migrations,
RLS, connexions et E2E web. Actualiser docs/ADR/statut, pousser les changements
autorisés et faire qualifier la PR/CI sur le candidat exact.

**Fin :** CC-G1 prouvé pour commit identifié, aucune lacune logicielle majeure
masquée par un mock, limites externes listées, rapport et revue de la candidate.
Les tickets externes restent ouverts ; ni push ni build ne vaut déploiement.

## CC-T14 — Qualification réelle et ouverture contrôlée

Rassembler uniquement les ressources nécessaires : compte IA, modèle et budget,
destinations de test Notion/Linear/GitHub, hébergement/domaine/Auth/SMTP. Configurer
les secrets via les mécanismes sécurisés. Exécuter le workflow sur cible privée,
publication/refresh/revocation réels, quotas/arrêt, sauvegarde/restauration et
alerte. Préparer responsable support et règles de données.

**Fin :** CC-G2 et CC-G3 avec dates/comptes pseudonymisés/capacités/versions,
coûts connus et inconnus, aucun accès non autorisé. Une ressource indisponible
est nommée avec l'action attendue ; les sous-preuves réussies restent conservées.

## CC-T15 — Pilote et décision V1

Dix boucles sur trois projets pendant au moins sept jours propriétaire, puis
au moins deux à trois participants actifs pendant quatorze jours. Relever
frictions, reformulations, contradictions utiles, sources inconnues, coûts et
incidents ; corriger puis requalifier les chemins touchés.

**Fin :** CC-G4 sur temps effectivement écoulé et personnes réellement actives,
décision de promotion ou prolongation motivée. Aucun nombre de fixtures ou de
tests ne remplace le pilote. Un suivi ultérieur n'est créé que selon
l'autorisation de programmation applicable ; ce ticket ne simule pas l'attente.

## Registre des preuves

Pour chaque clôture, ajouter date, commit, exigences, scénarios/commandes,
résultat, chemins de rapports, revue/corrections et limite. Utiliser les statuts
`vérifié`, `présent non reproduit`, `historique`, `à réaliser` par preuve et
préciser `simulation`, `API/DB réelle`, `service tiers réel` ou `exploitation`.
Ne pas confondre le statut d'un ticket avec la portée de ses preuves.

### Point d’intégration local — 2026-09-21

Base : `966ce9bbb597327e0ebbc3084836c6dedd759eca`. Les changements sont encore
locaux et non commités à ce point de contrôle. T01 produit les contrats et
l’ADR0007 ; société/catalogue/graphe et bibliothèque sont en intégration.

Preuves intermédiaires (elles ne clôturent pas un ticket) :

- 129 unités Rust réussies et une intégration ignorée avant ajout de la bibliothèque.
- 76 tests web réussis, lint et compilation réussis avant le lot bibliothèque.
- Graphe : six tests de projection, liens, navigation et sources réussis.
- Première recette Chromium : 34/42 réussites ; sept assertions/fixtures doivent
  suivre les nouveaux libellés et endpoints ; un débordement réel corrigé en
  recette ciblée. Les 18 scénarios concernés par les libellés/isolation/reprise passent après
  correction ; le contrôle a11y/débordement repasse également (1/1). La suite
  complète sera rejouée sur la candidate intégrée.
- Migration générée depuis le schéma déclaratif sur la stack isolée ; revue en
  cours, particulièrement backfill des arêtes, FORCE RLS et droits des fonctions
  que l’outil de diff ne reproduit pas intégralement.
- Aucune preuve fournisseur payant, publication vers outil tiers, invitation par
  SMTP réel, déploiement public ou usage pilote à ce stade.


### Point de consolidation — 2026-09-22

Les implémentations T02 à T12 sont maintenant présentes ; leurs preuves locales
sont distinguées des accès externes dans le
[rapport courant](candidate-validation-2026-09-22.md). `review` ne signifie pas
que tous les critères sont acceptés : le commit, la revue du candidat et sa CI
restent à identifier. Aucun ticket n'est fermé uniquement par présence de code.

La revue T08 a trouvé des objets absents du graphe : conversations et tâches,
navigation vers une connaissance historique ou un pack exact. Le complément
[graph-navigation-plan.md](graph-navigation-plan.md) est implémenté et sa recette
intégrée est en cours. T10 conserve aussi la consultation de grandes bibliothèques
de connaissances et le vocabulaire des anciens écrans comme travaux de finition.
Les qualifications fournisseur/outils, hébergement et pilote restent ouvertes.


Le complément du 22 septembre ferme les défauts locaux identifiés de navigation
fichier/tâche, de bibliothèque de connaissances et de vocabulaire projet.
157 unités web, 155 unités Rust, 21 scénarios société, 150 scénarios navigateur
avec doubles HTTP et deux parcours API/DB réelles passent. T08/T10 passent en
revue ; T13 doit rattacher images et CI au commit courant. T14/T15 restent
conditionnés aux ressources et usages réels, sans assimilation aux fixtures.
