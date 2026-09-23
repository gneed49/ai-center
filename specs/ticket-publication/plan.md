# Plan d’implémentation — Tickets distincts

> Statut : prêt pour revue de conception ; code/SQL gelés jusqu’au signal de l’intégrateur
> Spec liée : [spec.md](spec.md)

## Approche

Réutiliser un `publication_job` par entrée de ticket, en ajoutant uniquement son
index dans la version immuable. Une commande batch valide et inscrit les N jobs
atomiquement ; le worker, les adaptateurs de création, les baux et les opérations
de réconciliation continuent à traiter un job à la fois.

Une nouvelle table de batch n’est pas nécessaire : le reçu d’idempotence porte
la sélection et la liste des jobs ; les jobs sont retrouvables par version et
destination. Des identités permanentes de tickets interversions demanderaient un
contrat d’édition et de rapprochement plus large ; elles ne sont pas introduites
pour masquer ce manque. Les créations supplémentaires entre versions sont
explicitement présentées et confirmées.

La relation de provenance réutilise les observations distantes comme nœuds et
ajoute une arête vers la version source, index inclus. Pas de nouveaux objets
« tâches internes », ni de deuxième file d’exécution.

## Découpage

### 0. Sortie du gel et contrats

- Attendre le signal explicite de l’intégrateur avant toute édition code/SQL.
- Réserver alors le numéro de migration suivant ; ne pas modifier les migrations
  déjà intégrées pendant la recette en cours.
- Faire valider par revue les types de requête/réponse, la prévisualisation et
  les règles d’historique décrites ci-dessous avant le travail parallèle.

### 1. Domaine : projection exacte d’une entrée

- Ajouter un module borné dans `work_tools` pour extraire des tickets depuis
  `agent-artifact-v1` validé. Réutiliser le contrat d’artefact et ses contrôles
  de citations ; aucune analyse du Markdown pour deviner un ticket manquant.
- Normaliser les indices par tri ; refuser doublons, liste vide, >30 ou index
  absent. Un indice désigne toujours la version demandée, jamais la dernière
  version implicitement chargée.
- Produire titre et `business_body_markdown` exacts avec description, critères,
  citations de l’entrée et référence document/version/index. L’aperçu est limité à
  ce contenu métier ; informer qu’un identifiant technique de suivi sera ajouté.
  À l’admission seulement, créer le UUID aléatoire de chaque job manquant puis
  ajouter son marqueur technique et les séparateurs au corps persistant. Ne pas
  réserver de job ni dériver une identité globale pendant l’aperçu. Réutiliser
  l’export/provenance existants sans joindre tout le document à chaque issue.
- Contrôler avant inscription chaque titre et chaque corps (60 Kio, avec la
  longueur du marqueur UUID et les séparateurs comprise dès l’aperçu), ainsi que
  toute source/URL affichée. Les résumés globaux restent dans l’artefact lié.
- Conserver `content_hash` comme empreinte de la version d’artefact ; le corps
  du job et l’index fixent l’envoi particulier. Documenter cette sémantique pour
  ne pas prétendre que ce hash est celui du seul ticket distant.

### 2. Persistance compatible

- Ajouter `publication_jobs.source_ticket_index smallint not null default -1`,
  borne `-1..29`. Les lignes existantes deviennent documentaires sans réécriture
  de leur contenu, statut, acteur, marqueur ou identité.
- Remplacer uniquement l’unicité version/fournisseur/destination par
  `(workspace_id, artifact_version_id, provider, target_id, source_ticket_index)`.
  Examiner le nom réel de la contrainte dans la migration ; ne pas supprimer
  un index sans vérifier son rôle. Garder les FK et indices d’accès existants.
- Ajouter une validation d’insertion sous les mêmes droits : un indice >=0
  requiert Linear/GitHub, un artefact de tickets au format attendu et une entrée
  présente dans la version liée de la même société/projet. Aucun nouveau rôle
  ni accès SQL général ; garder les contrôles RLS forcés.
- Préserver les identités d’envoi après insertion, y compris l’index. Les droits
  runtime actuels de modification restent limités aux transitions d’état/reçu.
- Étendre DTO, lectures, export déclaré et tests de schéma. Les tables et FK
  d’effacement restent les mêmes ; recalculer l’empreinte du schéma d’effacement
  seulement après inspection de la base migrée, suivant le protocole existant.
- Anciennes demandes documentaires en file : elles gardent leur traitement
  autorisé initial. Aucun développement du lot ne les divise ni ne les recrée.
- Les JSON des reçus `publication.create` déjà enregistrés ne sont pas migrés.
  Accepter en lecture l’absence de `source_ticket_index` (valeur logique `-1`) et
  l’absence de `title`/`source_version_number`. Hydrater ces derniers exclusivement
  par `GET /api/publications/{id}` autorisé, sans réécrire le reçu ni prendre le
  titre de la version courante. Les nouvelles lectures de jobs renvoient ces
  champs ; l’adaptateur de reçus historiques tolère leur absence. Un GET refusé
  ou introuvable conserve un reçu historique aux détails indisponibles, sans POST.

### 3. Commande atomique et quotas

Contrats proposés, à garder séparés de la route documentaire existante :

- `POST /api/artifacts/{id}/ticket-publications/preview` : lecture seule, version,
  indices, connexion et destination attendue ; retourne contenu de chaque entrée,
  jobs existants, nombre de nouvelles créations, capacité disponible et historique
  pertinent avec empreinte. Retourner également `preview_fingerprint` selon le
  contrat canonique ci-dessous. Cette route ne réserve ni job ni appel fournisseur.
- `POST /api/artifacts/{id}/ticket-publications` avec clé d’idempotence : mêmes
  identités plus `preview_fingerprint`, `prior_publications_fingerprint` et, lorsque requis,
  `confirm_additional_issues: true`. Opération `publication.tickets.create`.
  Réponse : version/destination, liste ordonnée de publications avec index,
  nombres créé/existant. Aucun statut agrégé ne remplace les états individuels.
- Le reçu de commande est consultable en lecture seule sous acteur/société/projet ;
  la liste de publications permet aussi de retrouver les jobs par version et
  destination avec pagination. Ne jamais dépendre uniquement de la première page
  de dix résultats pour empêcher un doublon ou retrouver la sélection.

Dans une courte transaction :

1. Vérifier l’acteur, l’automatisation, le document/projet et verrouiller dans le
   même ordre que la publication documentaire ; relire la version courante
   validée, la destination normalisée et la connexion/révision autorisée.
2. Extraire et valider toutes les entrées et tous les corps métier. Recalculer
   l’historique pertinent, son empreinte et `preview_fingerprint` ; refuser la
   confirmation périmée avant toute inscription. Le client ne fournit aucun corps.
3. Chercher les jobs de même version/provider/target/index et calculer
   `new_count`. Ils restent canoniques quel que soit leur acteur ou leur état ;
   un job échoué/annulé n’est pas réactivé implicitement.
4. Étendre l’admission existante à `admit_publications(tx, new_count)` sous
   l’unique verrou quota société. Vérifier `pending + new_count <= limite` et
   `hour + new_count <= limite`. Un rejeu sans nouveau job ne consomme aucune capacité.
5. Inscrire uniquement les manquants, auditer sélection/version/destination et
   créations supplémentaires confirmées, puis compléter le reçu idempotent
   dans la même transaction. Tout rejet provoque rollback de tous les nouveaux
   jobs ; aucun appel distant ne se fait avant commit.

Le verrou document et l’unicité SQL arbitrent les batchs concurrents, y compris
deux membres et deux clés. La requête ne modifie jamais l’acteur d’un job déjà
présent. L’autorisation restant liée à cet acteur, un job annulé après révocation
est présenté comme tel ; le batch ne transfère pas silencieusement son exécution.

### 4. Exécution et compatibilité documentaire

- Conserver worker, `claim_publication_job`, `finish_publication_job`, une seule
  tentative create et marqueur par job. Les adaptateurs reçoivent toujours un
  titre et un corps ; pas de boucle N appels dissimulée dans un adaptateur.
- Le job porte déjà son corps préparé : le worker ne recompose pas les tickets
  depuis une version plus récente. Les vérifications courante/validée et droits
  existantes restent actives avant l’envoi.
- Réutiliser `refresh`, `reconcile`, `cancel` individuellement. Une erreur/limite
  API produit l’état prévu par la politique existante ; aucune nouvelle relance
  automatique n’est introduite. Montrer les succès et les résultats à vérifier
  côte à côte sans revenir sur les succès.
- Route documentaire : Notion et les artefacts non tickets restent à `-1`.
  Pour un nouvel envoi de tickets structurés vers Linear/GitHub, orienter vers
  la commande batch ; les anciens reçus/jobs `-1` restent lisibles/rejouables.
  Une ancienne publication agrégée entre dans l’avertissement de confirmation.

### 5. Interface, reprise et provenance

#### Contrat minimal consommé par l’interface

- Ajouter une lecture de couverture `GET /api/artifacts/{id}/ticket-publications`
  filtrée par `version_id`, `provider` et `target_id`. Elle retourne toutes les
  entrées de cette version (au plus 30), leur indice, titre et publication
  existante éventuelle. Cette couverture est indépendante de l’historique paginé ;
  son chargement incomplet ou son erreur bloque une nouvelle préparation.
- Une publication retournée porte `source_ticket_index`, `title`, `version_id`,
  `source_version_number` et les champs existants de statut, objet et lien. `-1`
  est affiché comme « Document complet ». Les numéros visibles valent indice + 1 ;
  les indices ne servent jamais à apparier automatiquement deux versions.
- L’aperçu serveur retourne les identités figées de version/destination/connexion,
  `requested_count`, `new_count`, `existing_count`, `items` ordonnés contenant
  `source_ticket_index`, `title`, `business_body_markdown`, `existing_publication`
  (job existant ou null), puis `preview_fingerprint` et
  `prior_publications_fingerprint`. Il indique également
  `requires_additional_confirmation`. Aucun marqueur futur n’apparaît dans
  `business_body_markdown`. La capacité reste une indication datée ; elle n’est
  pas une réservation ni une garantie de départ.
- La commande fournit seulement les identités, `ticket_indexes` distincts et
  l’acquittement requis par l’aperçu. Le client ne transmet jamais son propre titre,
  Markdown ou état de publication. La réponse contient les publications ordonnées
  dans `publications`, avec `created_count` et `existing_count` ; ces nombres
  décrivent l’inscription locale, jamais le succès des créations distantes.
- Ajouter un reçu en lecture seule par clé de commande, par exemple
  `GET /api/artifacts/{id}/ticket-publication-commands/{key}`, borné à
  acteur/société/artefact et à l’opération `publication.tickets.create`. Il distingue
  traitement, résultat enregistré, reprise autorisée, échec et expiration. La
  lecture ne crée aucun job et ne déclenche aucun appel externe. Les collaborateurs
  consultent les publications accessibles ; ils ne reprennent pas la commande
  personnelle d’un autre acteur.

#### Empreinte de confirmation backend figée

- Format nommé `ticket-publication-preview-v1`, SHA-256 hexadécimal minuscule sur
  une sérialisation canonique à champs fixes. Inclure `actor_id`, `workspace_id`,
  `project_id`, `artifact_id`, `version_id`, le `content_hash` de la version,
  `ticket_indexes` triés, les `title`/`business_body_markdown` exacts correspondants,
  `provider`, `target_id` normalisé, `connection_id`, `connection_revision` et
  `prior_publications_fingerprint`. Les identifiants du périmètre sont publics.
- L’aperçu renvoie les mêmes identités, indices canoniques, `connection_revision`,
  `source_version_number` et `content_hash`. La commande renvoie les identités de
  sa demande, `ticket_indexes`, `preview_fingerprint`,
  `prior_publications_fingerprint` et `confirm_additional_issues` lorsque requis.
  La révision de connexion effectivement relue participe à la recomposition du
  hash : une rotation entre aperçu et confirmation impose un nouvel aperçu.
- Exclure UUID/marqueur du futur job, capacité, dates de lecture, `new_count`,
  `existing_count` et états des jobs de la sélection courante. Ces jobs doivent
  pouvoir apparaître entre deux batchs concurrents sans empêcher leur réutilisation.
  L’historique des autres versions/documents agrégés reste lié par son empreinte.
- Canoniser la sélection et la destination avant l’empreinte idempotente de la
  commande ; un même payload logique ne change pas d’identité à cause de l’ordre
  des indices. Refuser les duplications avant ce calcul. La clé personnelle du
  navigateur et le payload accepté restent identiques lors d’une reprise.
- Vérifier titre/corps métier entre aperçu et admission. Ajouter ensuite seulement
  le marqueur unique de chaque job nouveau, persister son corps final et conserver
  sans réécriture ceux des jobs existants. Ne pas inclure de secret dans un hash,
  un corps métier, une réponse d’aperçu ou un reçu navigateur.

#### Préparation explicite

- Présenter les tickets sous forme de cartes/listes lisibles sur mobile : case à
  cocher nommée « Sélectionner Ticket 2 — titre », titre, critères et bouton
  d’aperçu du corps envoyé. Aucune sélection initiale ; « Tout sélectionner »
  sélectionne explicitement les entrées admissibles et annonce leur nombre.
- Les tickets possédant déjà un job pour cette version et destination affichent
  leur état et leur lien/contrôle individuel. Ils ne sont pas proposés comme de
  nouvelles créations, même si le job a échoué, a été annulé ou reste à vérifier.
  Leur reprise éventuelle passe par la politique explicite du job existant.
- Afficher le nombre sélectionné et la capacité connue, sans sélectionner
  silencieusement les premiers tickets ni découper une demande en arrière-plan.
  Exemple : « 30 sélectionnés, 25 places actuellement disponibles ». La personne
  peut réduire sa sélection ou attendre ; le serveur vérifie le quota global.
- « Préparer les N tickets » appelle uniquement l’aperçu. Une région de confirmation
  affiche version, fournisseur, destination lisible, connexion, contenu de chaque
  entrée et nombre exact de nouvelles issues. Le bouton final dit « Confirmer la
  création de N tickets dans Linear/GitHub ». Une sélection vide ne permet pas de
  préparer ; un aperçu contenant zéro nouvelle création mène aux demandes existantes.
- Modifier sélection, version, destination ou connexion invalide l’aperçu et son
  acquittement. Un conflit de version/destination/historique recharge les données
  et demande un nouvel aperçu ; aucune adaptation silencieuse du payload confirmé.
  Le bouton de confirmation est bloqué pendant l’inscription et si l’aperçu visible
  ne correspond plus aux paramètres affichés.
- Si des publications antérieures ou un document agrégé existent, montrer leurs
  versions, états et liens. Ajouter une case non précochée « Je confirme la création
  de nouveaux tickets ; les tickets existants ne seront pas mis à jour ». Son
  acquittement porte l’empreinte serveur présentée. Les jobs annulés/échoués restent
  distincts des objets créés ou incertains. Un nouveau titre identique ne prouve
  ni continuité ni absence de doublon métier.

#### Reprise et résultats partiels

- Réutiliser la reprise durable avec identité acteur/société/projet/artefact,
  payload figé contenant version/destination/connexion/sélection/empreinte, clé et
  date. Aucun token, secret ou contenu distant n’est stocké dans ce reçu navigateur.
  L’interface doit aussi retrouver une commande en attente d’une version antérieure
  du même artefact : changer de version ne la transforme pas en une nouvelle demande.
- Après montage, rechargement, navigation ou réponse HTTP perdue, relire d’abord
  le reçu et la couverture. Ne jamais déclencher automatiquement le POST. Une
  reprise explicitement autorisée conserve exactement clé et payload ; une demande
  expirée n’est pas remplacée silencieusement. L’historique reste consultable avant
  toute nouvelle confirmation. Les caches et requêtes obsolètes sont invalidés au
  changement d’acteur/société comme pour les autres commandes.
- Pendant le traitement, afficher « Demandes enregistrées » puis les états réels
  par ticket. Exemple : « 3 publications confirmées, 1 résultat à vérifier, 1 échec ».
  Chaque carte conserve titre, version source, destination et lien individuel.
  Aucun titre global « Publié » si un ticket demeure en attente ou incertain.
- Reprendre `PublicationDetailPanel` pour l’annulation avant départ, l’observation
  et la réconciliation d’un seul job. Un résultat incertain affiche « Vérifier et
  rattacher le ticket existant », jamais « Republier le lot ». La réconciliation
  reste contrôlée par marqueur et destination ; un ticket voisin ne peut pas
  servir de reçu à cette entrée.
- Si la version courante de l’artefact change, les cartes historiques gardent leur
  version exacte et affichent « Une version plus récente existe ». Les créations
  déjà confirmées restent liées à leur source ; ni leur titre ni leur contenu ne
  sont réinterprétés depuis le nouveau tableau. La nouvelle version demande une
  validation et une préparation explicites avec l’avertissement précédent.
- Prévoir navigation clavier, focus vers la confirmation puis vers le suivi après
  admission, région de statut sobre pour les mises à jour et absence de défilement
  horizontal sur écran étroit. Les identifiants techniques restent dans les détails.

#### Lecture des livrables et entrée exacte depuis le graphe

- Réutiliser `typed-draft` pour afficher les cinq formats `agent-artifact-v1`
  comme des documents lisibles : résumé, sections, cartes de tickets, listes de
  critères d’acceptation et points à clarifier. Le contenu métier reste intégral ;
  les marqueurs Markdown ne constituent plus l’affichage principal de ces formats.
- Mapper les clés canoniques aux libellés français du produit, notamment objectif,
  périmètre, parties prenantes, étapes, risques, problème, exigences, règles métier,
  critères d’acceptation, hors périmètre, priorisation, architecture, livraison,
  dépendances et validation. Le titre métier d’un ticket reste son titre enregistré.
  Aucune modification du JSON ni du Markdown canonique n’est requise pour lire un
  ancien brouillon dont le titre de section contient encore une clé anglaise.
- Replier UUID, empreintes, références techniques et JSON de provenance sous
  « Détails techniques ». Montrer en premier les titres des sources, leurs versions
  et les liens accessibles. Les preuves exactes restent consultables et exportables.
- Ne pas ajouter de moteur ou dépendance Markdown pour ce lot. Les exports Markdown
  et JSON existants restent exacts ; les documents manuels ou historiques sans
  contrat typé conservent leur lecture de compatibilité clairement identifiable.
- Prendre en charge `/artifacts/{id}?version={version}&ticket={index}`. Charger
  d’abord la version explicitement demandée ; seul un entier canonique 0..29
  présent dans ce tableau peut désigner une entrée. Ouvrir et mettre en évidence
  cette carte, avec son titre et « Ticket N · version V », et permettre le focus
  clavier après chargement. Ne jamais remplacer une version introuvable ou une
  entrée absente par la version courante ou un ticket de même position ailleurs.
- Une valeur invalide ou hors tableau affiche un message compréhensible, sans
  lancer de mutation. Le changement de version retire la sélection d’entrée ou
  exige une nouvelle navigation explicite : un index n’est pas une filiation entre
  versions. La vue mobile doit rester lisible à 390 px sans défilement horizontal.
- Tests : rendu des cinq contrats, libellés français pour les anciennes clés,
  absence de `##` et d’UUID/hash dans la lecture principale, présence des preuves
  dans les détails, export inchangé, navigation historique avec `ticket=0` et 29,
  entrée absente/invalide, focus clavier et écran étroit.

#### Provenance et vérification

- Historique : empreinte canonique sur les identités version/job, indices,
  états et identités distantes pertinents, triés. Exclure dates mouvantes sans
  importance. Si un état change entre aperçu et confirmation, refaire l’aperçu.
- Étendre la projection des arêtes de graphe : observation de publication →
  version d’artefact, `derived_from`, provenance avec job et index exacts. Réutiliser
  le nœud d’observation et les bornes existantes ; le panneau navigue vers la
  version et son entrée. Les filtres ne produisent pas d’extrémités pendantes.
- Adapter les exports de publications pour inclure l’index, et conserver les
  citations effectivement utilisées par l’entrée dans le document externe. Ne pas
  recopier les autres tickets ou le document entier dans chaque issue. Ne pas ajouter
  de tableau de tâches ou de statut local prétendant piloter l’avancement externe.
- Tests UI minimaux : sélection 2 sur 3 et absence de POST avant confirmation ;
  aperçu périmé ; couverture de 30 tickets malgré un historique paginé de 10 ;
  succès/échec/incertain simultanés ; réponse perdue + rechargement sans deuxième
  POST ; changement d’acteur/société ; nouvelle version sans création automatique ;
  avertissement après ancien document agrégé ; clavier et écran mobile.

### 6. Validation et revue

- Tests unitaires d’extraction et de corps ; tableaux 1/3/30, Unicode, source
  invalide, mauvaise version, indice borné, empreinte stable et limite 60 Kio
  incluant le marqueur final. Vérifier aperçu/admission : contenu métier identique,
  seul marqueur ajouté ; changement de connexion/révision ou de contenu refusé,
  arrivée d’un job courant équivalent réutilisée sans deuxième création.
- HTTP fictif Linear et GitHub : trois titres/corps distincts, trois appels et
  reçus ; Notion reste un appel documentaire. Résultat ambigu de l’élément 2,
  réconciliation croisée refusée, aucun create rejoué pour l’élément 1 ou 2.
- PostgreSQL gardé : migration legacy, batch concurrent/deux acteurs/deux clés,
  quota 25 avec 24 places occupées et N=2 → aucun nouveau job, 1 existant +
  1 nouveau correctement comptés, société étrangère, lecteur, archive/révocation,
  pause et version obsolète. Faire varier l’historique avant confirmation.
- Rejouer un reçu `publication.create` réellement ancien, sans les trois nouveaux
  champs : JSON enregistré inchangé, index affiché `-1`, métadonnées obtenues par
  GET autorisé, aucun nouveau job/appel ; tester aussi le refus de ce GET.
- Tester exports et effacement projet/société avec plusieurs items et reçus,
  refus pendant travail actif, empreinte schéma, droits runtime et restauration.
- Web : aperçu → confirmation N → états partiels → refresh/reload, aucune
  création automatique, liens individualisés, changement d’acteur/société,
  parcours clavier et confirmation nouvelle version/ancien document agrégé.
- Recette PM 2 tickets → Linear, lead 3 tickets → GitHub avec fournisseurs locaux
  fictifs ; revue indépendante des invariants et du nombre réel de créations.

## Impacts

| Surface | Changement borné |
| --- | --- |
| Domaine/données | Un attribut d’index sur publication_jobs ; unicité et validation de source adaptées. Pas de table batch/task. |
| API | Aperçu/batch/lecture de reçu ; index ajouté aux publications. Route documentaire conservée avec limite explicite pour nouveaux tickets Linear/GitHub. |
| Frontend | Sélection, confirmation de N créations, reçus individuels et historique ; primitives de reprise existantes. |
| Sécurité | RLS, acteur d’origine et vérifications worker inchangés ; validation serveur de la sélection ; aucun secret dans aperçu/export. |
| Coûts/débit | Chaque nouvelle issue compte dans les quotas existants ; aucune augmentation implicite, aucune boucle IA. |
| Graphe/contexte | Arête de provenance issue/observation → version + index ; citations individuelles conservées. |
| Données/Ops | Index dans exports ; inventaires de tables inchangés ; empreinte, grants runtime et reprise/maintenance vérifiés après migration. |

## Validation

- [ ] Signal de fin du gel reçu ; numéro de migration attribué.
- [ ] Types partagés et corps/previews relus ; TP-001 à TP-004.
- [ ] Quotas, concurrence et permissions ; TP-005/006.
- [ ] Compatibilité historique/Notion et créations interversions ; TP-007/008.
- [ ] Graphe, exports, effacement/restauration ; TP-009/010.
- [ ] Parcours web et revue indépendante ; TP-011.
- [ ] Format, lint, tests ciblés, intégration, CI et image au commit livré.

## Déploiement et retour arrière

Avant migration : terminer la recette candidate en cours, sauvegarder l’état
de référence, vérifier les jobs existants et arrêter l’admission des nouvelles
publications pendant la transition. Appliquer la migration additive puis le
serveur/UI compatibles dans la même fenêtre ; aligner l’empreinte contrôlée au
démarrage et les inventaires runtime avant reprise. Tester les anciennes demandes
`-1` et un lot synthétique sans service distant réel.

Après inscription de plusieurs indices pour une version, l’ancienne contrainte
unique ne peut plus être restaurée sans perdre ou fusionner des reçus. Le rollback
normal désactive l’entrée batch dans l’interface/serveur et conserve le schéma,
les jobs et leurs capacités de lecture/réconciliation. Une restauration complète
reste un exercice contrôlé avec sorties désactivées : aucune restauration ne doit
recréer les issues déjà présentes dans les outils. Préférer une correction avant
de réintroduire un binaire qui ignore la granularité des jobs.
