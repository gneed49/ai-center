# Revue du design T17 contre le code

> 2026-09-23, alignée le 2026-09-24 — revue de cohérence par l'auteur du plan, à distinguer d'une revue
> indépendante finale. Lecture du checkout courant ; aucun code, SQL ou test DB.
> T16 conserve la priorité. Cette note précise le [plan](plan.md) avant exécution.

## Constats à traiter avant le premier parcours

| Priorité | Preuve actuelle et risque | Décision minimale |
| --- | --- | --- |
| P1 | `scope_context/chat.rs:62,87` traite tout type non connaissance comme un artefact ; `context.rs:206` déduit le caractère obligatoire du seul `entry_type`. Ajouter une UNION SQL suffit donc à mal typer la nouvelle source ou à rendre une donnée distante obligatoire. | Discriminants explicites et exhaustifs de source et confiance, type entrant `external_observation`, jamais `business_rule`/`constraint`. Conserver ces métadonnées dans le pack compilé, pas seulement dans la réponse de recherche. Tester qu'une page intitulée « règle obligatoire » reste facultative et observée. |
| P1 | `service/artifact_generation.rs::exact_sources`, `artifacts/validation.rs:38,55`, `artifacts/sources.rs` et **`work_tools/ticket_projection.rs:72`** n'acceptent que les connaissances/versions d'artefact pour les citations de tickets. Étendre seulement le chat donnerait un brouillon ensuite refusé ou impossible à publier. | Faire passer un même UUID d'observation jusqu'au ticket généré, à la validation puis à l'aperçu T16. Ne pas supprimer silencieusement la citation pour faire passer le validateur. |
| P1 | `scope_context::verify_snapshot` ne compare que `projects.graph_version`; `pack_current` ne vérifie que les familles actuelles. La révocation d'une connexion n'incrémente pas les graphes aujourd'hui. | Ajouter un contrôle exact de fraîcheur/éligibilité des observations incluses au début et à la finalisation des appels IA. Vérifier aussi connexion active/capacité/révision attestée, référence active et head exact ; un stamp de graphe seul ne suffit pas. |
| P1 | `company/data.rs::require_active` et `scope_context::verify_snapshot` verrouillent les projets. T16 prend document → projet et lit la connexion sans `FOR SHARE`, compatible avec son éditeur. Une future révocation qui prend connexion → projets introduirait l'ordre inverse. | T17 prend projets triés → advisory connexion partagé → référence → reçu à la finalisation ; admission ajoute le quota avant reçu. `save_connection`/`disable_connection` prennent l'advisory exclusif identique, sans projet ensuite. Aucun privilège owner de connexion n'est accordé à l'éditeur. La fraîcheur immédiate repose sur les prédicats exacts, pas une invalidation synchrone de projets depuis la révocation. Aucun verrou pendant HTTP. |
| P2 | `publication_observations` possède une FK société/job, **pas de colonne project_id**. La FK `(observation_id,source_project_id)` décrite génériquement dans le plan ne peut pas y être appliquée. | Réutiliser l'approche de `validate_steward_scope_source` dans `09_github_code.sql:104` : FK `(observation_id,workspace_id)` existante + trigger qui résout le projet par job et contrôle UUID/projet/société. Pas de backfill ni nouvelle colonne projet dans cette table. |
| P2 | Le head de la nouvelle référence vers son observation forme un cycle. `operator_purge_project` ne traite actuellement explicitement que le head d'artefact avant suppression ; l'inventaire interdit les tables/FKs inconnues. | FK head différable et procédure de purge qui neutralise ce head dans le jeu exact autorisé avant suppression des observations. Ajouter les deux tables, leurs FKs, les prédicats et les empreintes ; vérifier le refus si un autre projet cite l'observation. Ne pas compter sur cascade pour contourner les blockers. |
| P2 | Les observations de publication actuelles ajoutent une ligne même pour `unchanged`; leur fraîcheur/steward compare l'ID maximal. La promesse « inchangé sans dépense IA » n'est donc pas fournie pour cette famille. | Pour les nouvelles références, conserver le head si snapshot identique. Pour les publications, conserver les reçus append-only mais définir une identité de version sémantique parmi les observations équivalentes contiguës ; employer la même règle dans retrieval, fraîcheur et frontier. A→B→A reste une nouvelle version. Garder les contrôles marqueur/destination stricts. |
| P1 | `work_tools/publications.rs::observe` n'atteste actuellement aucune connexion/révision dans l'observation et ne les recontrôle pas au retour HTTP. Le job initial ne prouve pas l'autorité d'une relecture ultérieure. | Décision root du 24 septembre : attestation nullable `connection_id` + `connection_revision` dans `publication_observations`, écrite serveur après contrôles advisory. Legacy sans attestation reste historique jusqu'à relecture autorisée ; pas de backfill ni mutation du reçu. Le groupe sémantique expose son autorité actuellement attestée et `publication_observation_current(bigint)` la vérifie. |

La dernière ligne est nécessaire à ETC-009/010 : l'existence d'un reçu nouveau
ne doit pas devenir artificiellement un nouveau contenu à examiner. Le découpage
en deux tables reste justifié ; il ne résout pas à lui seul ces consommateurs.

## Premier contrat vertical livrable

1. Une connexion **Linear** autorisée et un projet actif ; rattacher une issue
   précise par UUID/URL, sans marqueur AI Center. Un aller réseau borné lit son
   titre/description/état et produit une observation immuable `observed_external`.
2. Le PM répond avec cet UUID de source ; un artefact de deux tickets garde cette
   citation exacte, passe la validation et présente deux aperçus T16 sourcés.
   Le handoff/pack du lead permet d'ouvrir cette même observation historique.
3. Un refresh inchangé ne crée pas une nouvelle version ni un appel steward.
   Un contenu modifié remplace le head, périme les packs qui l'incluent et signale
   les artefacts fondés sur l'ancien état ; le texte historique reste intact.
4. Une révocation en vol refuse la capture/le résultat IA ; un refresh concurrent
   ne renverse pas les versions ; un reçu perdu se relit sans nouvel appel.
   Export et purge ciblée sont prouvés sur ce même fixture **[FICTIF]**.

Cette tranche comprend la lecture locale/historique et un formulaire simple ;
elle ne nécessite ni catalogue d'outils, ni import en masse, ni nouvelle file.
Notion et les observations de publications sont la seconde tranche du même T17,
obligatoire avant sa clôture. ETC-001..012 et G1 restent ouverts tant qu'elle manque.

Contrat commun à figer avant travail parallèle : `SourceObservation` avec
`source_kind`, `public_id`, `reference_public_id`, `source_project_public_id`,
`provider`, `external_id`, `canonical_url`, `version`, `observed_at`,
`remote_updated_at?`, `content_hash`, `snapshot_hash`, `trust`, `coverage`,
`omission_reasons`, `excerpt` et `freshness`. L'état courant de vérification est
séparé du snapshot historique. La citation conserve l'UUID historique ; son
éligibilité actuelle est recalculée, jamais déduite d'un état stocké par le client.

## Inventaire exact minimal d'impacts

| Tables / types | Impact de T17 |
| --- | --- |
| **Nouvelles** `tool_source_references`, `tool_source_observations` | Identité/head, observation, FKs composites, RLS, append-only, index et purge. |
| `work_tool_connections` | Seulement `allow_existing_reads`; capacité/rotation changent revision. `read_retry_after` est un champ DTO dérivé des audits, aucune colonne ni droit UPDATE ajouté pour le cooldown. |
| `artifact_version_sources` | Deux types `tool_source_observation`, `publication_observation`, colonnes FK exclusives et validation d'identité/portée. |
| `context_pack_scope_sources` | Les mêmes deux types/FKs ; décisions inclus/exclus, snapshots exacts et contrôle d'identité. |
| `steward_scope_sources` | Nouveau type/FK `tool_source_observation`; `publication_observation` existe déjà. |
| `steward_scan_sources` | Étendre le CHECK source_kind et la projection/canonicalisation de version, pas nouvelle file. |
| `publication_jobs`, `publication_observations` | Aucun changement de job pour T17. Sur observations : attestation nullable connexion/révision (ensemble), FK société ; pas de colonne projet ni réécriture historique. Projection de groupes sémantiques et prédicat courant partagés. |
| `context_pack_sources`, `context_pack_selection_items`, `deliverable_sources` | **Pas d'extension structurelle nécessaire au vertical** : les externes sont enregistrés dans `context_pack_scope_sources`; le livrable garde son lien exact au pack. Tester cette filiation transitive. Ajouter des FKs directes ici seulement si un parcours exige réellement une citation directe distincte du pack. |
| `projects`, `context_packs`, `deliverables`, `domain_events`, `audit_events`, `idempotency_records` | Réemployer champs/états/événements ; nouveaux consommateurs/commandes, sans nouvelle table. Audits durables de slots réseau et cooldown, index fenêtre société/action/date ; pas de lecture des reçus personnels d'autrui pour compter les slots. Un changement de head incrémente le graphe ; un contrôle inchangé ne l'incrémente pas. |

Fonctions/modules concernés, à considérer comme liste de vérification :

- **Outils** : nouveau `work_tools/sources` et lecteur entrant ; transport partagé,
  `work_tools::{save_connection,disable_connection,settings}`, `reliability` ;
  routes dédiées et reçu idempotent qualifié par projet/opération. Réutiliser le
  contrôle de bail atomique, pas seulement un test avant attente de verrou.
- **Pipeline** : `scope_context::{load,load_for_query,persist,pack_current,
  pack_source_ids,verify_snapshot}`, `scope_context/chat.sql`, `context::{ContextCandidate,
  compile_context_pack,is_contract_required}`, `agent.rs`, `service.rs` et
  `service/artifact_generation.rs::exact_sources` ; `artifacts::{sources,validation,
  conversion,generation_contract}` et `work_tools/ticket_projection.rs`.
- **Graphe/steward/export** : `company/{graph,graph_nodes.sql,graph_edges.sql,
  graph_source,graph_source.sql,export_columns,project_data,project_export_sources.sql}` ;
  `steward/{company,company_sources.sql,progress}` ; serveur de détail d'insight
  déjà capable d'afficher `source_snapshot`, à vérifier avec le nouveau type.
- **SQL contexte** : `validate_scope_source_identity`,
  `context_pack_scopes_current`, `validate_steward_scope_source`,
  `steward_scope_sources_current`, `graph_endpoint_exists` et nouveau validateur
  de provenance d'artefact. Coutures retenues :
  `tool_source_observation_current(bigint)` et
  `publication_observation_current(bigint)`, invoker/RLS, sans effets de bord.
  Le booléen de fraîcheur d'un pack n'est pas l'autorité d'un appel IA en vol :
  capture puis comparaison de l'autorité sous advisory sont précisées dans
  [pipeline-plan.md](pipeline-plan.md).
- **Maintenance** : `operator_purge_tables`, `operator_workspace_manifest`,
  `operator_purge_workspace`, `operator_project_predicate`,
  `operator_project_manifest`, `operator_project_preserved_fingerprint`,
  `operator_purge_project`, `operator_project_schema_fingerprint`/empreinte attendue,
  `operator_erasure_row_allowed` et exceptions append-only. Vérifier les procédures
  de travail abandonné pour les nouveaux noms de commande ; aucune capacité de
  réactivation privilégiée d'un import n'est nécessaire.
- **Inventaires** : `lib.rs` runtime, `99_runtime_extensions.sql`, vérificateur
  `scripts/sql/verify-runtime-db-role.sql`, exports/projections de sources directes,
  fixtures d'effacement et upgrade. Migration et grants restent propriété de root.

Le scope du projet est une frontière de contexte, **pas une ACL privée nouvelle** :
les politiques actuelles autorisent les membres de la société. T17 conserve les
portées de parcours de graphe et cette politique existante ; il ne doit pas promettre
une confidentialité par projet non implémentée. Un token de société peut lire
plus que la personne distante : le rattachement explicite partage le snapshot
avec les lecteurs autorisés AI Center, comme indiqué dans le formulaire.

## Gap GitHub existant, sans nouveau lot de synchronisation

`scope_context/chat.sql`, `scope_context.rs::load` et `exact_sources` ne prennent
ni `external_reference_observation` ni `github_code_file_observation`. Ces données
sont présentes dans le graphe/steward, mais un lead/dev ne les reçoit pas comme
sources exactes pour ses questions/générations. Le parcours utilisateur attend
également ce contexte existant : le gap ne disparaît pas avec Notion/Linear.

La projection `observed_external` peut ensuite accueillir ces **deux types déjà
stockés**, sans nouveau connecteur/table de corpus. Préserver pour le code
repository/commit vérifié/path/hash/lignes et couverture de fichier sélectionné ;
les références GitHub restent des métadonnées, pas du code lu. Cette extension
demande les mêmes validateurs/FKs de provenance et la fraîcheur par version exacte.
Elle est à arbitrer explicitement dans la séquence par root, pas déclarée couverte
par ETC-001..012 ni assimilée à une lecture exhaustive du dépôt.

## Répartition pour trois développeurs

| Propriétaire | Fichiers / responsabilité exclusive | Dépendance d'intégration |
| --- | --- | --- |
| Backend outils | `work_tools/sources/**`, lecteur entrant, routes outil dédiées, contrats HTTP/DB des commandes, DTO API, quotas/reprise/connexion ; ne modifie pas les schémas ou `service.rs`. | Reçoit noms SQL/DTO figés de root ; expose `SourceObservation` et résultats de commandes. Coordonne le petit changement partagé de configuration des connexions avant édition. |
| UI | API/types frontend, page Sources, formulaire ciblé, détail/historique, commandes persistées et panneaux de sources ; tests composants/navigateur. | Fixtures DTO communes ; aucun contenu/état inventé. Remontage = GET reçu, jamais refresh automatique. Ne masque pas un source_kind inconnu comme une connaissance. |
| Root schéma + pipeline | Migrations/RLS/FKs/grants/inventaires/maintenance ; `scope_context`, `context`, `agent`, `service`, artefacts, **ticket_projection**, graphe/steward/exports et tests de filiation. | Première preuve Linear verticale avant Notion/polish. Root assure les contrôles à travers les modules et arbitre GitHub. Il possède seul le statut de clôture et la recette DB. |

Ordre : contrat et migration minimale → lecteur Linear et projection source en
parallèle avec UI sur DTO → recette verticale → Notion/publications → recette
globale/revue indépendante. Pas de « sources importées » annoncé livré tant que
le PM et le lead ne citent pas effectivement l'observation exacte.

## Ajustements opérationnels retenus le 24 septembre

Le [contrat API](api-contract.md#6-admission-cooldown-et-ordre-de-verrous) remplace
les deux propositions initiales imprécises : cooldown en colonne connexion et
simultanéité par simple comptage des commandes. Compter les tentatives réseau
admises via audits avec slot de 60 s ; seul un finished correspondant ou l'expiration
libère ce slot. Deadline monotone admission + HTTP de 45 s, sans renouvellement de
slot par heartbeat. Cooldown distant ≤24 h dérivé des audits, sans changement de
révision. Les autres limites restent 120 lectures/h, 60 s par source/saisie et
200 références actives par scope. Aucune de ces données n'ouvre une permission
privée de projet ni une nouvelle capacité d'écriture de connexion pour l'éditeur.

La préparation détaillée du pipeline est désormais [pipeline-plan.md](pipeline-plan.md).
Elle ne constitue pas une livraison du code ; l'implémentation attend le feu vert
de root après le commit et la qualification T16.
