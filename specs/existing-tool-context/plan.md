# Plan d'implémentation — Contexte issu des outils existants

> Statut : design seulement ; réalisation après Tickets distincts et signal du coordinateur
> Spec liée : [spec.md](spec.md) ; 2026-09-23
> Aucune migration réservée, aucun code/SQL produit modifié par ce plan

## 1. Écart démontré et approche

Le 23 septembre, le code expose publication/refresh/reconcile, sans route de
rattachement Notion/Linear (`routes/work_tools.rs:12`). Le lecteur de publication
charge son job et vérifie son marqueur (`work_tools/publications.rs:182,224`).
Le transport de lecture existe (`work_tools/client.rs:236`), mais `notion_page`
exige le parent de publication et une propriété nommée title (`:316`).

Les observations sont visibles dans `company/graph_nodes.sql` et
`steward/company_sources.sql`. Les sources de `scope_context/chat.sql` et du
compilateur `scope_context.rs` sont limitées aux connaissances et artefacts ;
`service/artifact_generation.rs::exact_sources` reproduit cette limite.
Un simple bouton « importer » sans extension de cette chaîne serait insuffisant.

Retenir un module borné `work_tools/sources` avec deux tables dédiées, un lecteur
entrant distinct et une projection typée commune aux sources de contexte. Réemployer
authentification, `work_tool_connections`, chiffrement, HTTP officiel, quotas,
commandes idempotentes, audit, événements, graphe, steward et stockage PostgreSQL.
Pas de nouvelle dépendance, file externe, worker spécialisé ou fournisseur.

### Pourquoi deux tables dédiées

| Option | Constat | Décision |
| --- | --- | --- |
| Étendre `external_references` / `external_reference_observations` | `01_domain.sql:762` impose fournisseur GitHub, dépôt obligatoire, URL GitHub et connexion `tool_connections` (GitHub App). Réutilisation exigerait deux familles de credentials et une migration de plusieurs invariants historiques. | Rejetée pour ce lot : abstraction plus large que le besoin. |
| Réemployer `publication_jobs` / `publication_observations` | Job lié à une version d'artefact, un envoi, un marqueur et un reçu canonique. | Rejetée : inventer une publication affaiblirait la reprise des écritures. |
| `tool_source_references` / `tool_source_observations` | Objets existants, lectures seules, même famille de credentials que les connecteurs actifs. | Retenue : deux petites entités avec FKs et historique explicites, projection vers les consommateurs existants. |

Ne pas renommer les références GitHub ni faire migrer leurs données. Les observations
de publication existantes accèdent aux nouveaux consommateurs par un second type
source exact ; aucune copie dans les nouvelles tables et aucun faux rattachement.

## 2. Modèle de données et admission

La migration additive sera numérotée par le coordinateur après intégration du lot
tickets. Ajouter `allow_existing_reads boolean default false` à la connexion ;
seul son propriétaire autorisé peut l'activer. Son changement incrémente la révision.
La capacité porte sur les objets accessibles à cette connexion, explicitement
partagés avec le scope choisi ; pas de permission implicite issue d'une destination.

`tool_source_references` :

- `id bigint`, `public_id uuid`, `workspace_id`, `project_id` ; le scope société
  utilise son projet société existant, pas un scope nullable parallèle.
- `provider` Notion/Linear, `object_kind` page/issue, `external_id` UUID canonique,
  `canonical_url`, `connection_id`, `connection_revision`, auteur et dates.
- `status active|detached`, `revision`, `current_observation_id`,
  `last_checked_at`, `last_check_status`, `last_check_error_code` nettoyé.
- Unicité `(workspace_id,project_id,provider,external_id)`, sans connexion dans la
  clé : changer la clé d'accès ne crée pas un doublon. Une référence existante liée
  à une autre connexion est signalée ; un rebind explicite par owner, suivi d'une
  nouvelle lecture, conserve la même identité et incrémente la révision.

`tool_source_observations` append-only :

- Identifiants, société, projet, référence, `version` locale strictement croissante,
  identité distante/URL, révision de connexion au moment de la capture.
- `observed_at timestamptz`, `remote_updated_at timestamptz nullable`,
  `title`, `body_markdown`, métadonnées réduites (état Linear, parent Notion).
- `availability available|unavailable`, `coverage complete|partial|none`,
  `omission_reasons`, `projection_version`, `snapshot_hash`, `content_hash`.
- `content_hash` engage la projection métier normalisée (titre/corps/état),
  `snapshot_hash` engage aussi identité, métadonnées distantes, couverture,
  omissions et date distante ; exclure date locale, acteur et identifiants de
  transport. Comparaison uniquement avec la dernière observation.
- Texte ≤64 Kio, snapshot normalisé ≤128 Kio. Une observation unavailable ne
  contient pas une ancienne copie présentée comme nouvellement lue.

Une relecture identique conserve `current_observation_id` et la version ; met
à jour `last_checked_at` et inscrit un reçu d'audit pointant la même observation.
Une projection ou couverture différente appende une observation et remplace le
head. A→B→A donne trois versions. Les erreurs temporaires ne remplacent pas le
head : elles modifient seulement le statut de dernière vérification et son reçu.
Un refus d'accès ou un objet indisponible observé remplace le head par une version
sans contenu et invalide les usages actifs. L'UI expose séparément le dernier
contenu historique et l'état courant.

FKs composites `(project_id,workspace_id)`, `(connection_id,workspace_id)`,
`(reference_id,project_id,workspace_id)` et head appartenant à sa référence.
Indexes par scope/date, référence/version, connexion, et chaque FK utilisée lors
des purges. RLS ENABLE/FORCE et privilèges runtime minimum ; aucun droit direct aux
rôles publics. Vérification de l'identité publique dans les tables de provenance.

## 3. Lecteurs entrants, limites et URLs

Créer `ExistingToolReader` (ou module dédié équivalent) ; partager transport et
normalisation simples avec ToolClient sans changer `read`/`notion_page` utilisés
par publication, ni retirer leur vérification de destination/marqueur.

### Notion

- Entrée : UUID ou URL HTTPS sur `notion.so`, `www.notion.so`, `app.notion.com`,
  dont le chemin contient un UUID de page non ambigu. Refuser domaine personnalisé,
  notion.site, userinfo, port, schéma différent et chemins non reconnus ; indiquer
  comment copier le lien/identifiant pris en charge. Ne jamais GET l'URL collée.
- Construire les requêtes sur `api.notion.com` avec la version API épinglée prise
  en charge. Lire métadonnées, Markdown sans transcript, puis métadonnées de
  nouveau (3 appels maximum dans 45 secondes). Si date/identité changent pendant
  la lecture, refuser la capture comme instable ; pas de boucle de retry.
- Résoudre le titre par propriété de type `title` ; accepter les parents page,
  workspace et data source documentés sans requête au parent. Ne pas importer les
  autres propriétés. Date manquante : conserver null, couverture temporelle non
  vérifiée ; ne pas prétendre une lecture atomique garantie par le fournisseur.
- UUID des métadonnées et Markdown identiques à l'objet demandé ; page archivée/
  corbeille indisponible. Canonicaliser le lien depuis l'UUID connu, ou valider un
  lien officiel retourné avec ce même UUID. `public_url` n'est pas une autorisation.
- `truncated`, `unknown_block_ids` et balises unknown → partial avec raisons.
  Ne pas suivre ces identifiants, ni les liens/enfants/propriétés paginés. Les
  omissions connues (transcripts, embeds, bases/propriétés) restent visibles même
  quand la projection Markdown récupérée n'est pas tronquée.

### Linear

- Entrée : UUID, identifiant de type `PROD-42`, ou URL officielle
  `https://linear.app/{workspace}/issue/{identifier}/{slug?}` ; extraction locale
  uniquement, sans requête à cette URL. Variables GraphQL typées, aucune requête
  construite depuis du texte distant.
- Une query `issue(id:...)` limitée à id/identifier/url/title/description/updatedAt,
  team.id et état simple. Identifier/UUID résolus, URL canonique validée HTTPS
  linear.app, identité et workspace du lien cohérents avec l'entrée lorsqu'ils
  étaient fournis. Mémoriser ensuite l'UUID stable pour tous les refresh.
- Pas de connexion GraphQL paginée : commentaires, relations et fichiers restent
  hors projection ; `description:null` signifie vide, pas erreur de lecture.
  Les erreurs GraphQL sont examinées même avec HTTP 200 ; données/erreurs ambiguës
  ne produisent pas une capture complète. Aucun appel `mutation`.

### Bornes partagées

Conserver HTTP sans redirection/proxy utilisateur, 5 secondes de connexion,
30 secondes par requête, 45 secondes cumulées, 256 Kio réseau par réponse et
64 Kio UTF-8 retenus au total. Une coupe locale valide indique partial ; une
réponse JSON hors limite/décodage invalide n'est pas tronquée puis parsée.
Markdown rendu sans HTML actif, scripts, chargement d'images distantes ou liens
exécutables. Un lien dans le texte ne devient jamais une requête serveur.

Réutiliser le budget `work_tool.remote_read.admitted` et son verrou société,
120 opérations/h par défaut, y compris tentatives échouées ; une opération est
bornée à 3/1 appels selon fournisseur. Pas d'augmentation silencieuse de ce quota.
Ajouter une borne de 3 lectures réseau admises simultanées/société via audits
de tentative, un espacement minimum de 60 secondes/source, et 200 références actives
par scope (sous verrou), afin d'éviter accumulation non bornée. Les GET locaux
et rejeux terminés ne consomment pas le quota distant.

Les délais Retry-After Notion et limitations GraphQL Linear sont traduits en
`retry_after` nettoyé ; pas de sleep long ni retry interne. Le cooldown persiste
dans les audits `work_tool.remote_read.rate_limited`, avec connexion publique et
échéance serveur bornée à 24 h. Prendre le maximum des échéances futures valides,
sous le verrou de quota commun, à chaque nouveau départ. **Aucune colonne SQL
`read_retry_after` n'est ajoutée** : le champ du DTO est calculé depuis ces audits.
La révision de connexion ne change pas ; aucun droit d'écriture owner n'est accordé
à l'éditeur. Index d'audit `(workspace_id,action,occurred_at DESC)` pour la fenêtre
bornée. Il s'agit d'une borne AI Center, pas d'une promesse sur le débit distant.

Les slots réseau comptent les audits `work_tool.remote_read.admitted` non terminés
et non expirés, identifiés par `{command_public_id,lease_generation,attempt_id}`.
`slot_expires_at` vaut `clock_timestamp()+60 s` après acquisition du verrou. Une
commande simplement créée ne réserve rien. `work_tool.remote_read.finished` libère
le slot uniquement après terminaison/annulation du futur réseau ; en crash, il
expire conservativement. La deadline monotone de 45 s commence avant admission et
couvre admission puis HTTP ; aucun appel ne démarre après cette deadline. Les
reprises explicites consomment une nouvelle admission ; les GET/rejeux/existing
résolus localement n'en consomment pas. Le premier alias Linear non encore résolu
peut nécessiter une query budgétée pour découvrir une référence déjà existante,
sans en modifier le head ni simuler un refresh. Voir [api-contract.md §6](api-contract.md#6-admission-cooldown-et-ordre-de-verrous).

## 4. Contrat API, concurrence et reprise

| Route proposée | Contrat |
| --- | --- |
| `GET /api/projects/{id}/tool-sources` | Liste locale paginée, filtre fournisseur/état, curseur stable, ≤50 lignes, couverture/count exact autorisé. |
| `POST /api/projects/{id}/tool-sources` | Connexion + fournisseur + lien/identifiant + confirmation du scope ; commande `tool_source.attach`, clé idempotente obligatoire. |
| `GET /api/tool-sources/{id}` | État courant, identité, connexion sans secret, dates et dernière observation visible. |
| `GET /api/tool-sources/{id}/observations` | Historique local paginé ; jamais réseau distant. |
| `GET /api/tool-source-observations/{id}` | Snapshot exact et métadonnées de fraîcheur actuelles, sous droits courants. |
| `POST /api/tool-sources/{id}/refresh` | `expected_revision` + head attendu ; commande `tool_source.refresh`. |
| `POST /api/tool-sources/{id}/detach` | Retrait actif idempotent avec révision attendue ; conserve historique et source distante. |
| `POST /api/tool-sources/{id}/rebind` | Owner, nouvelle connexion autorisée + révision attendue ; nouvelle lecture obligatoire avant activation. |
| `GET /api/projects/{id}/tool-source-commands/{key}?operation={action}` | Reçu de l'acteur/société/projet/action qualifiée, états habituels ; aucun appel externe. |

Un attach réussi retourne `reference`, `observation`, `created|existing`,
`verification_status`, couverture et dates. Un identifiant déjà rattaché retourne
la référence existante sous mêmes droits/connexion sans créer une seconde source ;
un échec de premier accès n'inscrit aucun contenu ni faux objet vérifié.

Admission : vérifier acteur/scope actif, capacité de connexion/révision, arrêt
opérateur, clé/empreinte, quota et référence attendue ; capturer l'autorité.
Fermer la transaction avant HTTP. Après réponse, finaliser dans une transaction
courte : revérifier appartenance/scope/connexion/révision, verrouiller l'identité
ou référence, comparer head/révision capturés, vérifier le bail idempotent et son
expiration, puis source/observation/audit/invalidation/événement/reçu atomiques.
Transition du bail avec jeton/expiry et `rows_affected=1` ; sinon rollback total.

Deux refresh de commandes distinctes ne peuvent renverser l'ordre : le premier
qui finalise incrémente `revision`, même si inchangé ; le second devient conflit
sans écrire une observation ancienne. Une identité initiale unique arbitre les
attach concurrents. Rebind/detach incrémentent aussi revision ; le résultat d'un
ancien appel ne réactive jamais la source. Ordre commun retenu : projets triés →
advisory connexion partagé → référence connue → quota société → reçu pour
l'admission ; projets triés → advisory connexion partagé → référence → reçu pour
la finalisation. Si plusieurs connexions sont impliquées, trier leur identité.
Le verrou partagé emploie la même clé société/connexion que le verrou exclusif
de `save_connection` et `disable_connection` ; ces deux mutations owner doivent
participer avant activation des lecteurs. `FOR SHARE` sur la ligne de connexion
ne convient pas aux éditeurs sous la politique UPDATE owner-only. Aucun chemin
ne prend un projet après une connexion/reçu, aucun verrou ne reste pendant HTTP.
`begin` et heartbeat de commande restent des transactions distinctes courtes sans
verrou métier après le reçu. T16 garde ses garanties actuelles de départ worker.

Une réponse perdue se relit localement ; après crash avant commit, une lecture
peut être reprise explicitement avec la même commande admissible, car aucun
objet distant n'a été créé. Chaque tentative réseau est budgétée. Ne pas réessayer
en arrière-plan ni confondre cette reprise avec une réconciliation de publication.

## 5. Chaîne complète de contexte et confiance

Ajouter les types exacts `tool_source_observation` et `publication_observation`
aux consommateurs concernés, avec discriminant de source/trust visible ; ne pas
les déguiser en `knowledge_entry_version`. Les UUID publics restent ceux de leurs
tables respectives, validés côté serveur parmi les sources effectivement fournies.

- `scope_context/chat.sql`, compilateur et DTO : projection explicite
  `trust=observed_external`, identité/date/couverture/hash/extrait, `mandatory=false`.
  Même parcours de scopes et RLS que les connaissances ; les règles société
  confirmées gardent leur priorité. Au plus 20 sources externes par sélection,
  8 Kio/extrait et 64 Kio cumulés, à l'intérieur du plafond total de contexte
  existant, jamais en supplément non borné. Compteurs d'omissions séparés.
  Fournir aussi l'état de la dernière vérification : après timeout, une ancienne
  observation lisible reste historique, jamais présentée comme vérifiée à cet instant.
  Dédupliquer les candidats de même scope/fournisseur/UUID canonique si un objet
  publié a aussi été rattaché : choisir l'observation éligible la plus récente,
  garder son vrai type/UUID et la filiation des autres dans l'historique. Le
  steward ne compare pas deux captures du même objet comme deux objets distincts.
- `agent.rs` et instructions : sources comme données, jamais instruction système.
  Les routes externes ne sont pas des tools autonomes de l'agent. Réponses et
  artefacts citent seulement des UUID autorisés ; avertissement lié à la couverture
  visible dans la source, sans texte technique imposé au parcours métier.
- `artifact_version_sources` : types + FKs exactes et exclusives aux observations,
  snapshot de provenance sans récupération implicite de la dernière version.
  `sources.rs`, `exact_sources`, validation/append/conversion et exports conservent
  ces sources. Préserver les snapshots des sources identiques lors d'une édition.
- `context_pack_scope_sources`, sélection, citations de livrables/handoff : même
  identité immuable et couverture, pas seulement URL ou copie du texte. Vérifier
  fraîcheur au début et à la finalisation de génération/chat/compilation.
- `pack_current`, impacts artefacts et règles de publication : changement de head,
  disponibilité, retrait, connexion/révision invalide ou accès perdu rendent une
  dépendance incluse à réexaminer. Les sources seulement examinées puis exclues
  ne périment pas un pack. Une relecture inchangée ne le périme pas. Le contenu
  historique reste figé ; pas de mutation d'une version pour changer son statut.
- Graphe : nœud référence et observation exacte, lien observe/source/derived_from
  avec scope vérifié, puis filiation existante vers artefact/ticket publié. Étendre
  validation des endpoints, sélections et source panels, sans nœud métier inventé.
- Steward : ajouter la projection bornée/partielle au catalogue et aux FKs exactes
  de `steward_scope_sources`. Émettre l'événement existant seulement sur nouveau
  head/retrait/invalidation. Réemployer frontier, déduplication et budget ; les
  observations partielles restent preuves insuffisantes pour une absence.

Pour les publications, ajouter `connection_id` et `connection_revision` nullable
ensemble à `publication_observations`, avec FK société exacte et attestation
serveur lors du règlement/relecture. Les observations historiques sans attestation
restent historiques jusqu'à un refresh/reconcile explicitement autorisé ; aucun
backfill d'autorité supposée. Les champs d'attestation sont hors hash métier.
Le worker et `publication::observe` vérifient l'autorité au retour HTTP sous le
protocole advisory ; un reçu de création réellement obtenu doit rester conservé
comme reçu historique même s'il n'est plus éligible au contexte actif.

Grouper les observations d'un job par état métier/couverture/identité équivalents
**contigus** ; l'UUID du premier reçu du groupe est sa source sémantique canonique.
Le détail exact d'un autre reçu conserve son propre UUID. La fraîcheur et l'autorité
active du groupe proviennent d'une relecture attestée, jamais de la révision du
job initial supposée encore valable. A→B→A forme trois groupes. Root fournit la
projection partagée et `app.publication_observation_current(bigint)` ; les nouvelles
références utilisent `app.tool_source_observation_current(bigint)`. Le flag
`allow_existing_reads` concerne les rattachements, pas les objets publiés par
AI Center. Les contrôles destination/marqueur des lecteurs de publication restent
stricts. Les deux nouveaux types sont raccordés ensemble au pipeline décrit dans
[pipeline-plan.md](pipeline-plan.md), avec capture d'autorité avant IA puis contrôle
avant toute persistance de sa sortie, y compris les sorties invalides.

## 6. Interface et exploitation

Ajouter « Sources » au projet/société, réutiliser le panneau de sources du graphe :
liste paginée, ajout ciblé, détail/historique et bouton Actualiser. Aucune demande
de clé dans ce formulaire ; lien vers la configuration owner si capacité absente.
Afficher scope partagé, texte lu, date de dernière vérification et cause concise
de couverture partielle. Détail technique/empreinte replié ; liens historiques
ouverts sur l'observation exacte. Lecteur : consultation seule.

Persister la commande en attente par acteur/société/projet/type ; au montage,
consulter son reçu sans POST. Changement d'acteur ou de société invalide l'état
local exposé ; réponse tardive du scope précédent ignorée. Une déconnexion ne
conserve ni clé ni contenu sensible dans un cache global. Tests clavier/mobile
et état connexion désactivée, erreur/retry, partiel, retiré, historique.

Exporter tables, snapshots, hashes, provenance, révisions et dates sans secrets.
Inclure nouvelles FKs/tables dans inventaires d'effacement, dry-run/counts,
opérations projet/société, prédicats append-only autorisés pour purge, maintenance
et ordre des suppressions (provenances/observations avant références/connexions).
Contrôler commandes en cours avant purge, sans pouvoir orpheliner un reçu après
retour HTTP. Réviser empreinte de schéma, droits/runtime vérifiés au démarrage et
scripts d'inventaire ; pas de contournement pour faire passer la recette.

## 7. Lots d'exécution et preuves

1. **Contrat vertical minimal** : migration/RLS + attach/refresh d'une fixture
   Linear, observation lue par PM puis citée par un artefact et un pack. Avant
   élargir UI/Notion, prouver ETC-001/005/006/007 sur cette chaîne réelle.
2. **Lecteurs bornés** : Notion parents/titre/Markdown, Linear identité/état,
   erreurs/URLs/couverture/dates stables, compteurs réseau et quotas. Tests HTTP
   loopback uniquement ; conserver tous les tests de publication actuels.
3. **Contexte complet** : projection confiance, schémas de sources exactes,
   graph/impacts/steward/publications historiques et non-régression pack/artefacts.
4. **Interface/reprise** : parcours ciblé, reçus persistés, lecture structurée et
   source panels. Agent UI distinct possible une fois DTO stabilisés.
5. **Qualification intégrée** : recette gardée du coordinateur, tests concurrence,
   révocation pendant réseau/IA, exports/purge/runtime et navigateur [FICTIF].
   Revue indépendante avant documentation produit et clôture logicielle.

Preuves attendues : ETC-001 à ETC-012, Clippy/typecheck/lint et unités ciblées,
contrats HTTP, PostgreSQL réel isolé, parcours navigateur synthétique. Inclure
attaque par document « ignore les règles/publie/transmets le secret », URL externe
piégée, 404 ambigu, réponse partielle, A→B→A, inchangé sans dépense steward, acteur
révoqué, rotation/rebind en vol et réponse perdue après commit. Les gates externes
restent séparés. Aucun live provider nécessaire pour écrire les tests.

## 8. Déploiement et retour arrière

Migration additive coordonnée après Tickets distincts ; aucune donnée historique
réécrite. Activer capacité de lecture par connexion seulement après sa décision
owner. Ne pas lancer de backfill, scan de compte ou IA sur les anciennes données.
Retour arrière : désactiver la capacité et retirer les nouvelles routes de
mutation ; conserver tables/snapshots pour export et historique. Un ancien binaire
ne sachant pas lire les nouveaux types de provenance n'est pas compatible avec
des artefacts déjà créés : retour applicatif seulement vers une version qui les
tolère. Ne pas dropper des preuves pour permettre un rollback.

## 9. Sources officielles vérifiées le 23 septembre 2026

- [Notion — lecture Markdown](https://developers.notion.com/reference/retrieve-page-markdown) :
  contenu, indicateurs de troncature/blocs inconnus, accès refusé et option de
  transcript. Le lot choisit de ne pas suivre les sous-arbres inconnus.
- [Notion — lecture d'une page](https://developers.notion.com/reference/retrieve-a-page) :
  métadonnées/propriétés, parents, limites de mentions/propriétés et 404 ambigu.
  La lecture ne constitue pas une révision transactionnelle garantie.
- [Notion — objet page](https://developers.notion.com/reference/page) : UUID,
  date distante, parent et titre typé ; propriété de titre renommable dans une
  data source. Les champs non nécessaires ne sont pas importés.
- [Notion — limites](https://developers.notion.com/reference/request-limits) :
  limites du service et Retry-After ; aucune hypothèse de quota distant fixe dans
  l'application. Les bornes 45 s/3 appels/64 Kio sont des décisions de ce lot.
- [Linear — GraphQL](https://linear.app/developers/graphql) : lecture ciblée
  `issue(id:...)`, notamment par identifiant lisible ; conserver ensuite l'UUID.
  La sélection exacte des champs sera figée par contrat sans introspection live
  nécessaire au runtime, et les champs optionnels resteront explicitement optionnels.
- [Linear — limitations](https://linear.app/developers/rate-limiting) : erreurs
  GraphQL `RATELIMITED` et limitation de complexité, distinctes du succès HTTP.

Ces pages documentent les possibilités du fournisseur. Elles ne prouvent pas les
permissions d'une connexion réelle ni l'implémentation de ce plan.
