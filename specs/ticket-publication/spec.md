# Spécification — Tickets distincts dans les outils de l’équipe

> Statut : conception autorisée ; implémentation en attente de la fin du gel d’intégration
> Responsable : lot publications, coordination Company Context V1
> Dernière mise à jour : 2026-09-23

## Intention

Le PM et le lead peuvent préparer un document contenant plusieurs tickets
structurés. La publication actuelle transforme tout ce document en une seule
issue Linear ou GitHub. Les personnes doivent donc recréer les tâches à la main
avant de les affecter et de les réaliser. Cette granularité ne remplit pas le
parcours demandé : spécification → tickets produit → tickets techniques → travail
dans les outils existants.

Cette évolution précise la granularité manquante de CC-020/026. Elle ne remplace
pas les outils de tâches et ne transforme pas AI Center en outil d’implémentation.

## Résultat attendu

Après validation humaine de la version, sélectionner N tickets puis confirmer
leur destination crée N demandes de publication distinctes. En cas de succès
des N créations externes, N issues existent, chacune avec son titre, sa
description, ses critères d’acceptation, ses sources et son lien individuel.
L’interface rapporte les états réels par ticket ; elle ne présente pas un lot
partiellement exécuté comme entièrement publié.

Exemple fictif : cinq tickets produit publiés dans Linear donnent cinq issues.
Un document ultérieur de huit tickets techniques publiés dans GitHub donne huit
issues supplémentaires. Les documents validés restent conservés dans AI Center
et reliés à ces objets. Les affectations, cycles, tableaux et réalisation restent
dans Linear/GitHub.

## Contexte et sources

- [Mandat et contrat Company Context V1](../company-context-v1/spec.md), CC-U03,
  CC-U04, CC-020, CC-026, CC-027, CC-031 et gates CC-G1 à G4.
- [Lot artefacts générés](../company-context-v1/artifact-agent-plan.md) et
  [revue indépendante](../company-context-v1/acceptance-review-2026-09-23.md).
- [Outils externes canoniques](../../docs/decisions/0005-context-control-not-tool-replacement.md)
  et [graphe société](../../docs/decisions/0007-company-context-graph.md).
- Code existant : `artifacts/generation_contract.rs`, `work_tools/publications.rs`,
  `work_tools/worker.rs`, `work_tools/client.rs`, `work_tools/reliability.rs`,
  `supabase/schemas/06_work_tools.sql` et `ArtifactPublications`.
- Contrat observé avant changement : `ArtifactTicket` est une entrée d’un tableau
  de 1 à 30 tickets ; `publication_jobs` stocke un objet distant par version et
  destination. L’interface annonce une page ou un ticket. Aucun critère N → N
  issues ne figurait dans CC-026 : le présent amendement comble cette omission
  par rapport à l’objectif utilisateur, sans requalifier l’ancien test.

## Périmètre

### Inclus

- Artefacts `product_tickets` et `technical_tickets` au format structuré
  `agent-artifact-v1`, version courante explicitement validée.
- Publication d’une sélection de tickets distincts vers Linear ou GitHub,
  prévisualisée et confirmée en une commande humaine.
- Reçu, état, annulation avant départ, réconciliation et relecture par ticket,
  avec les garanties existantes de droits, quotas et résultat incertain.
- Filiation vers la version source et la position exacte de l’entrée ; visibilité
  dans les listes et le graphe, exports et procédure d’effacement conservés.
- Conservation des publications documentaires existantes ; Notion continue à
  recevoir une page pour le document complet.

### Exclu

- Nouveau tableau de tâches interne, exécution de code, affectation automatique,
  cycle/sprint, estimation, hiérarchie distante ou synchronisation exhaustive.
- Mise à jour automatique d’une ancienne issue depuis une nouvelle version,
  rapprochement sémantique des tickets entre versions, suppression distante.
- Transaction distribuée prétendant annuler les issues déjà créées si un autre
  ticket échoue ; relance aveugle d’une création au résultat incertain.
- Nouvelle table de batch, nouvelle dépendance ou nouveau moteur de jobs.
- Conversion silencieuse des anciennes issues documentaires en tâches séparées.

## Modèle et invariants

1. **Entrée source** : `(artifact_version_id, source_ticket_index)` identifie une
   entrée dans une version immuable. Les indices valent 0 à 29 ; le libellé humain
   vaut « Ticket 1 » à « Ticket 30 ». L’indice n’est pas une identité durable entre
   deux versions : l’ordre et le contenu peuvent changer.
2. **Publication** : un `publication_job` existant représente une création externe.
   `source_ticket_index = -1` signifie document complet, notamment pour les lignes
   historiques et Notion. L’unicité devient société + version + fournisseur +
   destination normalisée + indice. Les publications existantes gardent leurs
   identifiants, corps, marqueurs, reçus, états et historique.
3. **Lot** : une commande contient une liste canonique d’indices uniques, non vide,
   au plus 30. Tous les nouveaux jobs et le reçu de commande sont enregistrés dans
   la même transaction, ou aucun nouveau job ne l’est. Les jobs déjà présents sont
   retournés comme existants ; ils ne sont ni dupliqués ni remis à zéro.
4. **Contenu canonique** : le serveur extrait l’entrée depuis la version choisie.
   Le client ne fournit pas un titre ou un corps substitutif. Chaque corps reprend
   la description, les critères, les sources effectivement citées par cette entrée
   et la filiation document/version/index. L’aperçu montre exactement ce contenu
   métier et le titre. Un marqueur technique unique est ajouté seulement lors de
   l’admission du job ; il est explicitement hors aperçu, sans identité globale
   déterministe et sans réservation pendant l’aperçu. La limite existante de
   60 Kio s’applique au corps final, marqueur et séparateurs compris. Les sections
   globales de cadrage restent consultables par le lien vers l’artefact ; elles ne
   sont pas recopiées comme autant de tickets supplémentaires.
5. **Autorisation** : owner/editor, société et projet actifs, connexion et
   destination autorisées, automatisation active et version courante validée sont
   contrôlés à l’admission ; les contrôles du worker restent exécutés pour chaque
   job. Le créateur initial du job reste son acteur d’origine. Le rôle d’agent ne
   confère aucun droit. Un lecteur peut consulter les reçus autorisés, jamais créer.
6. **Exécution** : chaque job conserve sa tentative automatique unique, son bail,
   son marqueur et sa réconciliation. Un reçu perdu impose de vérifier l’objet
   correspondant à ce job. Le succès d’un autre ticket ne justifie ni duplication
   ni recréation du ticket incertain. Le statut affiché distingue publication et
   observation distante ; un succès de publication n’affirme pas que le travail
   métier est terminé.

## Parcours et comportements

### Préparer et confirmer

La page du document validé affiche les entrées, leur titre, leur état de publication
dans la destination sélectionnée et leur contenu métier exact prêt à transmettre,
avec la mention qu’un identifiant technique de suivi sera ajouté à l’envoi. La
personne sélectionne les entrées et voit « N nouveaux tickets seront créés » ainsi
que les jobs existants réutilisés. La confirmation est distincte de la génération
et de la validation du document. Changer de version, destination ou sélection
annule la confirmation précédente ; les contrôles serveur restent déterminants.
L’aperçu fournit une empreinte canonique liant acteur/société/projet, version et
empreinte du contenu source, sélection triée, titres/corps métier, destination
normalisée, connexion et révision, et empreinte des publications antérieures.
La commande renvoie cette empreinte ; le serveur la recalcule et refuse tout
changement. Marqueur futur, capacité et compteurs créé/existant n’entrent pas
dans cette empreinte : un autre batch ayant inscrit la même entrée ne doit pas
invalider sa réutilisation, ni rendre nécessaire un second envoi.

Pour Linear/GitHub, un nouvel artefact de tickets ne propose plus l’envoi implicite
de toutes ses entrées dans une seule issue. Un document historique agrégé reste
consultable et exportable. Une version sans tableau structuré valide ne peut pas
être découpée par inférence : expliquer le format requis, sans création externe.
Les autres types d’artefacts gardent leur publication documentaire actuelle.

### Capacité et exécution partielle

Les quotas existants restent par société, partagés entre réplicas et comptés par
nouveau job : par défaut 25 jobs en attente/en cours, 100 publications par heure.
La sélection s’ajoute entièrement à ces compteurs sous le verrou commun. Si les
places ou le débit sont insuffisants, aucun nouveau job n’est inscrit ; l’interface
indique une sélection plus petite ou une attente. Elle ne découpe pas la demande
en arrière-plan. Un document de 30 tickets peut ainsi demander deux sélections
explicites selon la capacité. Les jobs déjà créés ne consomment pas une seconde fois
le quota.

Après admission, les créations externes sont indépendantes. Exemple : 3 succès,
1 résultat incertain et 1 échec restent cinq états séparés. Les succès conservent
leurs liens ; le ticket incertain utilise son propre marqueur et sa réconciliation.
Le lot n’efface ni ne rejoue automatiquement les succès. Une réponse HTTP perdue
ou un rechargement retrouve les mêmes jobs par version/destination/index et par
reçu idempotent ; la lecture du résultat ne déclenche aucune création.

### Historique et risque de doublons entre versions

Le contrôle d’unicité protège la même version ; il ne signifie pas qu’un ticket de
version 2 est un nouveau travail par rapport à la version 1. Avant toute nouvelle
création, la prévisualisation recherche pour le même artefact/fournisseur/destination
les jobs de versions antérieures et toute publication documentaire `-1` de la version
sélectionnée. Elle montre leurs états et leurs liens, sans prétendre rapprocher les
entrées par index ou titre.

Par défaut, si des créations antérieures existent ou sont incertaines, l’envoi
nécessite une confirmation supplémentaire : les nouvelles issues s’ajouteront aux
anciennes, elles ne les mettront pas à jour. La commande porte l’empreinte du jeu
de publications antérieures présenté ; le serveur la recalcule dans la transaction
et refuse une confirmation devenue obsolète. Les jobs annulés/échoués restent
visibles dans l’historique et sont distingués des objets effectivement créés ou
incertains. Aucun ancien job n’est réassigné à une autre entrée ou version.

Les anciens reçus idempotents sont rejoués à l’identique, même lorsqu’ils ne
contiennent pas `source_ticket_index`, `title` ou `source_version_number`. La
lecture considère un indice absent comme `-1` (« Document complet ») et recharge
les métadonnées manquantes via `GET /api/publications/{id}`, sous les droits
actuels. Elle ne réécrit jamais `idempotency_records.response_body`, ne reconstitue
pas un titre depuis la version courante et ne transforme pas une lecture en
création. Si le job n’est plus accessible, le reçu reste historique et les détails
sont indisponibles ; aucune métadonnée n’est inventée.

### Graphe et contexte

Chaque observation distante garde son identité existante. Une arête de provenance
relie cette observation à la version d’artefact, avec `source_ticket_index` dans
sa provenance. Le panneau ouvre cette version et met en évidence l’entrée exacte.
Le graphe ne fabrique pas une tâche accomplie à partir d’un job simplement en file.
Les bornes de projection et l’isolation société demeurent applicables ; les listes
de publications donnent accès aux autres éléments si le graphe est tronqué.

## Critères d’acceptation

| Critère | Résultat vérifiable |
| --- | --- |
| TP-001 | Une version fictive contenant 3 entrées sélectionnées produit 3 jobs et, lorsque le faux outil répond avec succès, 3 issues distinctes aux titres/corps attendus, pour Linear puis GitHub. |
| TP-002 | Sous-ensemble, indices non triés, duplications, index hors limites, tableau absent et corps trop grand : normalisation documentée ou rejet avant toute nouvelle écriture externe ; aucun envoi documentaire implicite. |
| TP-003 | Deux commandes concurrentes, même clé ou clés différentes, visant la même version/destination/entrée retournent les mêmes jobs ; après perte de réponse/rechargement aucun job n’est dupliqué. |
| TP-004 | Une réponse de création perdue pour l’entrée 2 laisse uniquement son job à vérifier. Réconciliation du bon marqueur/objet, refus d’un autre ticket ou d’une autre destination ; aucun second appel create pour l’entrée incertaine. |
| TP-005 | Capacité insuffisante pour N nouveaux jobs : zéro inscription partielle, quota inchangé ; une sélection admissible crée exactement son nombre de nouveaux jobs, les existants étant réutilisés. |
| TP-006 | Lecteur, autre société, projet archivé, connexion révoquée, pause ou version modifiée : refus au bon stade. Une révocation entre deux jobs empêche les départs suivants ; l’issue déjà créée n’est pas supprimée ni déclarée inexistante. |
| TP-007 | La migration conserve les anciennes publications à `-1`, reçus et marqueurs inchangés. Le rejeu d’un reçu antérieur dépourvu des nouveaux champs reste lisible comme document complet et hydrate ses métadonnées par GET sans réécriture ni création. Notion crée une seule page complète ; les autres documents conservent leur comportement. |
| TP-008 | Nouvelle version, réordonnancement ou ancien document agrégé : publications antérieures visibles ; aucune assimilation de l’index entre versions ; création supplémentaire uniquement après confirmation explicite et encore actuelle. |
| TP-009 | Chaque issue reçue retrouve document, version, index et sources exactes dans l’API, l’UI, le graphe et l’export. Un autre projet/société non autorisé reste inaccessible. |
| TP-010 | Effacement et maintenance couvrent les jobs/observations individuels, préservent les autres projets et interdisent une purge pendant un job actif. Aucun secret ou jeton de bail dans l’export. |
| TP-011 | Parcours web fictif PM → validation → 2 tickets Linear puis lead → validation → 3 tickets GitHub : prévisualisation, confirmation, états individuels et reprise sont utilisables sans saisie technique. |
| TP-012 | Le titre et le contenu métier envoyés correspondent exactement à l’aperçu confirmé ; seul le marqueur technique documenté est ajouté. Empreinte stable sans ID de job futur ; modification de version, contenu, sélection, destination, connexion/révision ou historique → nouvel aperçu exigé. |

## Preuves attendues

- Tests purs de l’extraction, des corps, de la sélection, de l’empreinte de
  confirmation et des limites ; tests HTTP simulés des deux connecteurs.
- Intégration sur la stack jetable gardée : migration avec jobs historiques,
  concurrence, RLS, quotas atomiques, bail/réconciliation et graphe/export/effacement.
- Tests UI puis navigateur sur API réelle et fournisseurs locaux fictifs pour
  le parcours et la réponse perdue. Les fixtures portent `[FICTIF]`.
- Les services réels et le déploiement restent les gates CC-G2/G3 ; le test de
  batch synthétique ne les ferme pas. CC-G1 exige ces nouveaux critères logiciels.

## Risques et questions ouvertes

- Le titre et l’index ne constituent pas une correspondance métier entre versions.
  La V1 exige donc une confirmation des créations supplémentaires, sans mise à jour
  automatique des tâches existantes.
- Aucun quota n’est augmenté pour rendre un exemple de 30 tickets admissible.
  L’interface doit expliquer la capacité et permettre une sélection plus petite.
- Le numéro de migration et la fenêtre d’application sont attribués par
  l’intégrateur après la recette gelée. **Aucune édition code/SQL ni migration
  exécutée dans ce lot de conception.**
