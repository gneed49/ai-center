# Contrat API et lecteur — T17 Sources des outils existants

> 2026-09-24 — contrat de conception proposé pour figer les coutures backend,
> schéma, pipeline et UI avant implémentation. Aucun code T17 n'est livré par
> ce document. [Spécification](spec.md), [plan](plan.md), [revue du design](design-review.md).
> T16 reste en recette. Les choix ci-dessous précisent le plan ; toute divergence
> doit être arbitrée dans ce contrat avant d'être codée dans un consommateur.

## 1. Conventions et identité

- JSON en `snake_case`, UUID publics canoniques, dates RFC 3339 UTC ou `null`.
  Aucun identifiant SQL interne, credential, erreur distante brute ni URL signée.
- Fournisseurs entrants : `linear` et `notion`. `object_kind` vaut respectivement
  `issue` et `page`. Le scope société emploie le projet existant
  `projects.scope_kind = 'company'` ; aucune colonne de scope nullable parallèle.
- Le projet délimite le contexte ; les ACL existantes restent celles de la société.
  Une source explicitement rattachée partage son snapshot avec ces lecteurs.
- `source_kind` exact : `tool_source_observation` ou `publication_observation`.
  Les autres types de sources gardent leurs discriminants actuels ; aucune branche
  `else => artifact` ne doit absorber un type inconnu.
- `trust = 'observed_external'`, `mandatory = false`. Le titre et le texte ne
  peuvent changer ces valeurs. Une source externe n'est pas une règle validée.
- `version` est un entier local positif, pas une révision du fournisseur. La
  citation est toujours le couple `source_kind` + UUID d'observation exact.

## 2. Noms SQL proposés à root

Les créations SQL et les migrations restent propriété de root. Les nouveaux
objets de feature peuvent être définis dans `supabase/schemas/22_tool_sources.sql` ;
le nom de migration est généré au moment de son exécution.

### `app.tool_source_references`

| Colonne | Type et rôle |
| --- | --- |
| `id`, `public_id` | bigint identity PK, uuid public unique |
| `workspace_id`, `project_id` | bigint NOT NULL, FK composite vers le projet |
| `provider`, `object_kind` | text NOT NULL ; couples autorisés linear/issue, notion/page |
| `external_id` | uuid NOT NULL, identité distante canonique |
| `canonical_url` | text NOT NULL, lien officiel normalisé et borné |
| `connection_id`, `connection_revision` | bigint + integer NOT NULL ; connexion de société et dernière révision attestée par une lecture réussie |
| `created_by_actor_id`, `created_at`, `updated_at` | uuid + timestamptz, auteur et dates serveur |
| `status`, `revision` | active/detached ; integer positif, contrôle optimiste |
| `current_observation_id` | bigint nullable uniquement pendant insertion/purge autorisée, FK head appartenant à la même référence/projet/société |
| `last_attempt_at` | timestamptz nullable, admission durable avant HTTP |
| `last_checked_at` | timestamptz nullable, dernière vérification terminée ; distincte de l'admission et de la date du contenu |
| `last_check_status` | available/partial/unavailable/failed, nullable avant première capture |
| `last_check_error_code` | text nullable, code serveur fermé et nettoyé |

Unicité `(workspace_id, project_id, provider, external_id)` ; pas de connexion dans
l'identité. Indexer les FKs et les parcours `(project_id, status, created_at, id)`,
`connection_id`. La FK du head est différable pour l'insert et la purge gouvernée.
Une première lecture échouée n'insère pas une référence vide.

`connection_revision` de la référence est une **attestation actuelle**. Une
rotation invalide l'éligibilité tant qu'une nouvelle lecture autorisée n'a pas
attesté la connexion. Si cette lecture est identique, le même head peut être
réattesté sans réécrire l'observation historique ni créer une version artificielle.

### `app.tool_source_observations`

| Colonne | Type et rôle |
| --- | --- |
| `id`, `public_id` | bigint identity PK, uuid public unique |
| `workspace_id`, `project_id`, `reference_id` | bigint NOT NULL, FKs composites de portée |
| `version` | integer positif, unique `(reference_id, version)` |
| `provider`, `object_kind`, `external_id`, `canonical_url` | identité exacte à la capture ; `external_id uuid` |
| `connection_id`, `connection_revision` | identité/révision attestées au moment de cette capture, immuables |
| `observed_at`, `remote_updated_at` | timestamptz serveur NOT NULL et date distante nullable |
| `title`, `body_markdown` | text NOT NULL, contenu UTF-8 retenu ≤64 Kio au total |
| `availability`, `coverage` | available/unavailable ; complete/partial/none |
| `omission_reasons` | jsonb tableau de codes distincts triés, jamais texte distant |
| `projection_version` | text NOT NULL, initialement `existing-tool-text-v1` |
| `content_hash`, `snapshot_hash` | SHA-256 hex minuscule de 64 caractères |
| `metadata` | jsonb objet, contrat réduit ci-dessous |

Observation append-only hors purge autorisée. Snapshot normalisé ≤128 Kio ; une
observation unavailable porte corps vide, `coverage=none` et des métadonnées
minimales d'identité, sans ancien texte recopié. `title` peut alors reprendre un
libellé d'identité, pas simuler un titre nouvellement lu. Les snapshots valides
peuvent être partiels et conserver le passage effectivement observé.

Une page Notion sans titre conserve `title=""` : le libellé « Sans titre » appartient
à l’affichage, sans être inventé dans le snapshot observé.

`metadata` autorisé : Linear `{identifier, team_id, state:{id,name,type}|null}` ;
Notion `{parent_type, parent_id|null}`. Pas de dump des propriétés, du parent ou de
la réponse distante. Les champs d'affichage bornés font partie du budget snapshot.

`content_hash` engage titre, corps et état métier normalisés. `snapshot_hash`
engage également identité, métadonnées, couverture/omissions, date distante et
version de projection ; il exclut date locale, acteur, secrets et transport.
Comparer seulement au head : A→B→A donne trois observations. Un check identique
conserve le head, met à jour la vérification et incrémente `revision`, sans événement
steward de changement ni périmation des dépendances.

### Connexions et prédicat partagé

Ajouter seulement `allow_existing_reads boolean NOT NULL DEFAULT false` à
`work_tool_connections`. Modifier la capacité ou les credentials incrémente
`revision`. La mise à jour d'une connexion ne verrouille ensuite aucun projet.

Le cooldown est dérivé d'audits immuables `work_tool.remote_read.rate_limited`,
avec `connection_id` public et `retry_after` serveur borné à 24 h. Il n'exige ni
colonne `read_retry_after` modifiable par un éditeur ni fonction privilégiée :
les audits société sont déjà lisibles et insérables sous les droits existants.
Un index `(workspace_id, action, occurred_at DESC)` borne la lecture des dernières
24 h. Prendre le maximum des échéances futures valides pour cette connexion,
sans changer sa révision. Ce choix remplace le champ SQL proposé dans le plan
initial ; la valeur exposée en DTO est calculée, jamais un nouveau droit d'écriture.

Couture attendue du schéma :
`app.tool_source_observation_current(observation_id bigint) RETURNS boolean`,
invoker, sous RLS, sans side effect. Root confirme son nom avant code pipeline.
Elle exige projet actif, référence active, head exact, observation available,
connexion active de même société/fournisseur, capacité activée, et révision
courante égale à l'attestation de la référence. La révision historique de
l'observation reste informative ; elle ne bloque pas une réattestation inchangée.
Un échec technique récent n'affirme aucune lecture réussie, mais ne transforme
pas à lui seul le dernier contenu observé en contenu nouveau.

Pour une IA en vol, capturer en plus l'autorité de lecture utilisée
`{reference_public_id, observation_public_id, connection_public_id,
connection_revision}` et la comparer à la finalisation. Un refresh inchangé avec
la même autorité ne provoque pas de faux conflit ; rotation/rebind en vol en
provoquent un même si le corps final est identique. Pour un pack conservé, le
prédicat d'éligibilité actuelle du même UUID suffit : le snapshot reste historique.

## 3. DTO partagés

Ces interfaces décrivent le wire JSON ; elles ne demandent pas une nouvelle
librairie TypeScript. Les dates sont des chaînes et `UUID`/`IsoDate` des alias.

```ts
type ToolProvider = 'linear' | 'notion'
type SourceKind = 'tool_source_observation' | 'publication_observation'
type Coverage = 'complete' | 'partial' | 'none'
type VerificationStatus = 'available' | 'partial' | 'unavailable' | 'failed'

type SourceFreshness = {
  is_current: boolean
  eligible: boolean
  reasons: Array<'historical' | 'detached' | 'unavailable' |
    'connection_disabled' | 'read_capability_disabled' |
    'connection_unverified' | 'project_inactive' | 'invalid_identity'>
  current_observation_id: UUID | null
  last_checked_at: IsoDate | null
  last_check_status: VerificationStatus | null
  last_check_error_code: string | null
}
type SourceObservation = {
  source_kind: SourceKind
  public_id: UUID
  reference_public_id: UUID | null
  publication_public_id: UUID | null
  source_project_public_id: UUID
  provider: ToolProvider
  object_kind: 'issue' | 'page'
  external_id: UUID | null // null uniquement pour un ancien reçu mal formé
  canonical_url: string
  version: number
  observed_at: IsoDate
  remote_updated_at: IsoDate | null
  title: string
  excerpt: string
  availability: 'available' | 'unavailable'
  content_hash: string
  snapshot_hash: string
  projection_version: string
  trust: 'observed_external'
  mandatory: false
  coverage: Coverage
  omission_reasons: string[]
  freshness: SourceFreshness
}
type SourceObservationDetail = {
  observation: SourceObservation
  body_markdown: string
  metadata: Record<string, unknown> // seulement les champs fermés ci-dessus
}
type ToolSourceReference = {
  public_id: UUID
  project_id: UUID
  provider: ToolProvider
  object_kind: 'issue' | 'page'
  external_id: UUID
  canonical_url: string
  connection_id: UUID
  connection_revision: number
  status: 'active' | 'detached'
  revision: number
  current_observation_id: UUID
  created_at: IsoDate
  updated_at: IsoDate
  last_attempt_at: IsoDate | null
  last_checked_at: IsoDate | null
  last_check_status: VerificationStatus
  last_check_error_code: string | null
}
type ToolSourceDetail = {
  reference: ToolSourceReference
  observation: SourceObservation
}
type SourceCommandResult = ToolSourceDetail & {
  action: 'attach' | 'refresh' | 'detach' | 'rebind'
  effect: 'created' | 'existing' | 'changed' | 'unchanged' | 'detached' | 'rebound'
  verification_status: VerificationStatus | 'not_performed'
}
```

L'extrait de DTO est borné à 8 Kio ; le détail local conserve le corps retenu.
Liste : résumé sans extrait (champ vide), aucun corps complet. Les erreurs/données
non autorisées ne renvoient pas ces DTO. `reference_public_id` est non null pour
une observation rattachée, `publication_public_id` est non null pour une observation
de publication ; exactement un des deux. Ne pas fabriquer une référence pour un job.

`freshness` est recalculée à chaque GET sous les droits courants. Elle ne fait pas
partie du hash immuable ni de l'autorité d'une commande client. Le snapshot stocké
dans une provenance contient les métadonnées observées et l'état à la capture ;
il ne prétend pas posséder une fraîcheur dynamique stockée définitivement.

Les codes d'omission initiaux sont : `comments_not_read`, `attachments_not_read`,
`related_objects_not_read`, `properties_not_read`, `embedded_content_not_read`,
`transcripts_not_read`, `unknown_blocks`, `provider_truncated`, `local_text_limit`,
`remote_date_unavailable`. Ils expliquent une limite, jamais une absence prouvée.
`complete` signifie complet pour la projection textuelle annoncée ; les omissions
hors projection restent affichées. Troncature/bloc inconnu/coupe locale ou couverture
temporelle non vérifiable donnent `partial`. `unavailable` donne `none`.

## 4. Routes et commandes

Toutes les mutations requièrent authentification, rôle editor/owner, scope actif
et `Idempotency-Key`. Aucun POST de ce lot n'a l'exception read-only de T16.
Les GET exigent les droits courants, y compris les snapshots historiques/reçus.

| Route | Entrée et réponse |
| --- | --- |
| `GET /api/projects/{id}/tool-sources` | Query `provider?`, `status=active|detached|all` (défaut active), `limit=1..50` (défaut 25), `cursor?`. `{items:ToolSourceDetail[],next_cursor:string|null,total_count:number,active_count:number,limit:number}`. |
| `POST /api/projects/{id}/tool-sources` | `AttachToolSource` ci-dessous → `SourceCommandResult`. |
| `GET /api/tool-sources/{id}` | `ToolSourceDetail`, aucun réseau distant. |
| `GET /api/tool-sources/{id}/observations` | `limit=1..50,cursor?` → `{items:SourceObservation[],next_cursor,total_count,limit}` ; extrait vide dans la liste. |
| `GET /api/tool-source-observations/{id}` | `SourceObservationDetail` exact ; jamais résolution silencieuse vers le head. |
| `POST /api/tool-sources/{id}/refresh` | `ExpectedSource` → `SourceCommandResult`. |
| `POST /api/tool-sources/{id}/detach` | `ExpectedSource` → `SourceCommandResult`, sans HTTP. |
| `POST /api/tool-sources/{id}/rebind` | `RebindToolSource` → `SourceCommandResult`, owner seulement. |
| `GET /api/projects/{id}/tool-source-commands/{key}?operation={action}` | `action=attach|refresh|detach|rebind` obligatoire ; reçu ci-dessous. |

```ts
type AttachToolSource = {
  connection_id: UUID
  expected_connection_revision: number
  provider: ToolProvider
  source: string // URL officielle ou identifiant, 2 Kio maximum
  confirm_scope_sharing: true
}
type ExpectedSource = {
  expected_revision: number
  expected_observation_id: UUID
}
type RebindToolSource = ExpectedSource & {
  connection_id: UUID
  expected_connection_revision: number
  confirm_scope_sharing: true
}
type SourceCommandReceipt = {
  status: 'not_received' | 'processing' | 'interrupted' | 'retryable' |
    'completed' | 'failed' | 'expired'
  can_retry: boolean
  retry_after: IsoDate | null
  result: SourceCommandResult | null
  error: {code:string,message:string} | null
}
```

Un curseur opaque encode la dernière identité triée par date+id décroissants,
avec bornes de taille/validation ; une page suivante emploie le même filtre. Les
comptages exacts sont sous RLS et dans la même transaction que la page ; ils sont
une photographie de cette lecture, pas une promesse de liste immuable sous écriture.
L'historique d'une référence est trié par version décroissante. Un curseur invalide
est refusé, pas interprété comme un offset arbitraire.

Attach d'une identité connue active et de la même connexion : retourne `existing`
sans nouveau réseau ni prétendue vérification (`not_performed`). Connexion différente :
conflit explicite ; utiliser rebind owner. Référence retirée : conserver son identité,
proposer rebind avec la connexion voulue (éventuellement identique) ; aucune réactivation
silencieuse. Refresh d'une référence retirée est refusé. Rebind ne change l'autorité
et ne réactive qu'après sa lecture validée, atomiquement. Detach déjà effectué avec
la même clé est rejoué ; une nouvelle clé avec ancienne révision est un conflit.

Première lecture indisponible/incorrecte : réponse d'erreur sans référence ni
contenu. Refresh disponible mais partiel : succès explicite `partial`. Refus d'accès
ou 404 sur une référence connue : nouvelle observation sans contenu, état
`unavailable` (« absent ou inaccessible », pas « supprimé »). Timeout/429/5xx :
conserver le head, mettre à jour le reçu/statut de vérification, erreur nettoyée.

### Capacité de connexion sans nouvelle saisie de secret

`GET /api/work-tools` conserve son enveloppe ; chaque `connections[]` ajoute
`allow_existing_reads:boolean` et `read_retry_after:IsoDate|null` calculé depuis
les audits. `capabilities[]` ajoute `read_existing:boolean` (Notion/Linear true,
GitHub false dans ce lot). Ces champs ne contiennent aucun secret.

Réutiliser `POST /api/work-tools/connections` owner avec clé idempotente :
`{id,provider,name,expected_revision,allow_existing_reads:true|false}` ; omettre
`api_key` laisse le credential chiffré intact. Le booléen entrant est optionnel :
absent sur une ancienne requête, il préserve la valeur actuelle (false à la
création). Le sérialiseur l'omet lorsqu'il est absent, afin de préserver le hash
idempotent des anciennes commandes de connexion. Une modification explicite
incrémente revision ; même révision attendue et règles owner que la sauvegarde
actuelle. Aucun formulaire de source ne demande la clé.

### Refus de capacité et délais

Le corps d'erreur garde `code` et `message`, et ajoute pour ces commandes
`retryable:boolean` et `retry_after:IsoDate|null`. Le reçu reporte la même échéance
absolue ; elle ne glisse pas lors d'un GET/rejeu. Un éventuel en-tête HTTP
`Retry-After` indique les secondes restantes, mais l'UI n'en dépend pas seule.
La couche API frontend conserve ces deux champs optionnels, sans les inventer.

| Limite | HTTP / code | Reprise |
| --- | --- | --- |
| 3 lectures déjà admises actives | 429 `source_read_in_progress` | même commande, échéance du premier slot libérable, ≤60 s |
| 60 s depuis tentative de cette source/saisie | 429 `source_refresh_too_soon` | même commande, last_attempt +60 s |
| quota distant société épuisé | 429 `source_read_quota_exceeded` | même commande, sortie du plus ancien audit bloquant de la fenêtre horaire, ≤1 h |
| cooldown fournisseur | 429 `remote_rate_limit` | même commande après échéance auditée, ≤24 h |
| 200 références actives/scope | 409 `source_limit_reached` | non retryable ; retirer une source puis préparer une nouvelle commande explicite |

La commande refusée avant HTTP ne consomme pas une lecture distante. Une tentative
réseau suivie d'un 429 distant la consomme. Toute reprise explicitement admissible
repasse les limites ; aucune minuterie client n'envoie automatiquement un POST.
Une réponse 429 porte le marqueur serveur retryable compatible avec le protocole
idempotent existant. Les autres conflits gardent leurs causes courtes et demandent
une nouvelle préparation, jamais une correction silencieuse du payload figé.

### Identité durable et reprise

Opération interne exacte `tool_source.{action}:{project_uuid}` (<128 octets),
opération métier stable `tool_source.{action}`. Le projet est résolu par le serveur
pour les routes référence, jamais accepté depuis un corps client. Le hash engage
l'action, projet, référence éventuelle, payload canonisé et confirmation de scope.
L'identité d'entrée est normalisée localement **avant** ce hash ; une reprise garde
le même payload, sans substituer l'UUID résolu après le premier appel.

GET reçu : tuple exact société + acteur + projet + action qualifiée + clé UUID.
Aucun `LIKE`/préfixe large, aucune visibilité sur le reçu d'un autre acteur. Un
résultat terminé reste historique ; sa fraîcheur actuelle vient du GET source,
sans réécrire le `response_body` enregistré. GET inconnu ne crée rien. `can_retry`
est false après expiration, retrait de rôle, archive du scope ou action owner
indisponible ; le POST reste le validateur définitif des autres paramètres.

Ajouter cette famille qualifiée à la politique d'expiration non réinitialisable
et au fencing de renouvellement/finalisation de T16. Une commande expirée ne
redevient jamais une nouvelle lecture avec sa vieille clé. Reprise explicite après
crash avant commit : même clé/payload si bail récupérable, droits/autorité encore
valides ; nouvelle tentative réseau budgétée. Après commit : rejeu exact sans HTTP.
Les réponses 429/502/503/504 réellement transitoires portent le marqueur serveur
retryable ; les refus de portée, de version ou de connexion ne le portent pas.

Le navigateur conserve seulement identité/payload nécessaires à la commande, clé
et date, séparés par acteur/société/projet/action/référence. Pas de texte du document,
credential ou réponse distante. Montage/reconnexion = GET reçu ; POST uniquement
après clic. Un payload arrivé d'un ancien scope n'actualise jamais le scope visible.

## 5. Lecteur entrant Rust et limites

Module proposé `work_tools/sources/reader.rs`, DTO de wire dans `sources/models.rs`.
Transport borné réutilisable, sans assouplir `ToolClient::read`, destination ou
marqueur des publications. Le lecteur n'accède ni à la DB ni aux agents.

```rust
pub(crate) enum SourceLocator {
    NotionPage { id: Uuid },
    LinearIssue { key: LinearIssueKey, expected_workspace_slug: Option<String> },
}
pub(crate) enum LinearIssueKey { Id(Uuid), Identifier(String) }
// Parsing local pur, erreurs nettoyées ; aucune résolution HTTP de l'URL.
pub(crate) fn parse_locator(provider: &str, source: &str) -> AppResult<SourceLocator>;

pub(crate) struct ExistingToolSnapshot {
    pub external_id: Uuid,
    pub canonical_url: String,
    pub title: String,
    pub body_markdown: String,
    pub remote_updated_at: Option<DateTime<Utc>>,
    pub coverage: SourceCoverage,
    pub omission_reasons: Vec<OmissionReason>,
    pub metadata: SourceMetadata,
}
pub(crate) enum ExistingReadOutcome {
    Available(Box<ExistingToolSnapshot>),
    Unavailable { code: ExistingReadErrorCode },
}
pub(crate) struct ExistingReadFailure {
    pub code: ExistingReadErrorCode,
    pub retry_after_seconds: Option<u32>,
    pub retryable: bool,
}
// secret est emprunté, non sérialisable. Aucun titre/URL distante dans Debug/error.
pub(crate) async fn read_existing(
    &self, secret: &SecretString, locator: &SourceLocator,
) -> Result<ExistingReadOutcome, ExistingReadFailure>;
```

Codes fermés : `remote_authentication`, `remote_permission`, `remote_not_found`,
`remote_rate_limit`, `remote_timeout`, `remote_transport`, `remote_unavailable`,
`remote_response_invalid`, `remote_response_too_large`, `remote_identity_mismatch`,
`remote_changed_during_read`, `remote_redirect_blocked`. Aucune copie du message
GraphQL/HTML. Le service connaît la référence lors de refresh ; le lecteur ne
fabrique jamais l'identité d'un objet dont la première lecture échoue.

Le service calcule les dates serveur/hashes et applique la politique d'observation.
Le lecteur valide identité, projection, limites et URL canonique, uniquement sur
les endpoints officiels (loopback explicite réservé aux tests). Il respecte les
limites du plan : 3 requêtes Notion ou 1 query Linear, 45 s cumulées, 30 s/requête,
connexion 5 s, 256 Kio/réponse, 64 Kio texte. Aucun retry/crawl/pagination interne.
Notion est lu métadonnées → Markdown sans transcripts → métadonnées, avec refus
si l'identité/date distante change. Linear sélectionne seulement les champs du
plan ; un HTTP 200 avec erreurs GraphQL n'est jamais un succès complet implicite.

## 6. Admission, cooldown et ordre de verrous

Préparer sous autorité courante, puis fermer la transaction avant HTTP. L'admission
prend projets triés → connexion → référence connue → verrou quota société → reçu.
La finalisation prend projets triés → connexion → référence → reçu, sans quota à
réadmettre. Ici « connexion » désigne une exclusion de mutation à définir avec
root : `FOR SHARE` direct ne convient pas à l'éditeur sous UPDATE-RLS owner-only.
Pour T17 et son HTTP entrant, la couture proposée est un verrou advisory partagé
par le lecteur et exclusif par save/disable, sur la même clé société+connexion ;
les deux mutations owner doivent participer avant activation du lecteur. Aucune
extension de droits de modification n'est accordée aux éditeurs. T16 conserve sa
lecture de métadonnées sans ce nouveau protocole, avec recontrôle au départ worker. Aucun chemin n'acquiert ensuite un projet après avoir verrouillé une
connexion ou un reçu. `begin` de la commande et le heartbeat restent des transactions
courtes séparées ; ils n'acquièrent aucun verrou métier après le reçu.

Sous le verrou quota existant, vérifier arrêt opérateur/cooldown, nombre actif et
quota 120 lectures/h (configuration actuelle), puis écrire l'audit durable
`work_tool.remote_read.admitted`. Trois lectures réseau admises actives maximum/société. Sous ce même verrou,
compter les audits d'admission avec slot non expiré, sans audit
`work_tool.remote_read.finished` correspondant à la même tentative. Identités :
`command_public_id`, `lease_generation`, `attempt_id` UUID ; pas de lecture des
reçus personnels d'un autre acteur. Une commande simplement créée ne réserve
aucun slot, ce qui évite que plusieurs demandes non admises se bloquent entre elles.

Chaque audit d'admission porte `slot_expires_at = clock_timestamp() + 60 s`,
calculé après l'attente du verrou, et `input_fingerprint`/référence éventuelle,
sans texte ni credential. La borne monotone de 45 s commence **avant** la phase
d'admission et couvre admission puis HTTP : un délai de scheduling/verrou après
l'inscription ne permet pas de démarrer un appel encore actif après le slot.
L'audit finished libère plus tôt la place une fois le réseau terminé, même si la
finalisation métier devient ensuite conflit. En crash/révocation empêchant ce
reçu local, le slot expire conservativement à 60 s. Un heartbeat idempotent ne
prolonge pas une lecture réseau au-delà de cette borne. Ne pas émettre finished
avant que le futur HTTP soit terminé ou annulé.

Inclure les
tentatives échouées, pas les GET/rejeux/detach/réponses existing sans réseau.
Une reprise de la même commande fait une nouvelle admission réseau.

Pour une référence connue, `last_attempt_at` est mis à jour avant HTTP et impose
60 s minimum. Pour le premier attach sans référence, auditer une empreinte de
l'entrée normalisée (`input_fingerprint`) et imposer le même espacement à cette
entrée/connexion/scope. Ne pas journaliser l'URL brute. Un alias Linear jamais
résolu ne révèle son UUID qu'après la query : cette première résolution bornée
peut découvrir une référence existante. Elle consomme le quota puis retourne
`existing` sans modifier son head ; elle ne simule pas un refresh hors cooldown.
La borne avant résolution porte donc sur la saisie normalisée, pas sur une identité
canonique encore inconnue. UUID/alias déjà connus sont résolus localement sous la
même connexion avant admission ; jamais par correspondance de titre.

Maximum 200 références actives/scope contrôlé sous verrou de projet ; vérifier à
la préparation et à la finalisation d'un attach/rebind. `Retry-After` est traduit
en date serveur bornée (maximum 24 h), enregistré dans l'audit
`work_tool.remote_read.rate_limited` sans rotation de révision. Pas d'attente longue du serveur. Retrait de droits pendant
réseau : aucun texte/observation n'est persisté. Une erreur de cleanup ne doit
jamais contourner RLS pour conserver le contenu ou réussir la commande ; son bail
court expire et son reçu reste non terminé tant qu'aucune clôture étroite n'est possible.

Comparer la révision et le head capturés à la finalisation de toute vérification,
y compris inchangée/échouée. Le premier finalisé incrémente revision ; l'autre
retourne conflit et ne remplace jamais le head par une lecture arrivée tardivement.
Une unicité SQL arbitre les attach sans référence préalable. En conflit d'identité,
retourner existing sous la même connexion ou conflit de connexion, sans rafraîchir
la source existante depuis cette ancienne réponse.

## 7. Couture pipeline et observations de publication

Candidats, pack et provenance emploient les DTO d'observation avec source_kind,
trust/coverage et UUID exact. Artifact `SourceInput.kind` admet les deux nouvelles
valeurs littérales et ses FKs exclusives ; aucune conversion en `knowledge`.
Snapshot de provenance : conserver aussi `kind` (valeur identique à source_kind)
et `project_id` (UUID identique à source_project_public_id), pour les consommateurs
existants. Le compilateur peut garder son nom historique `version_public_id`, mais
sa valeur est cet UUID d'observation et son type/trust restent explicites.

La whitelist de citations de `exact_sources`, `generation_contract`, validation,
conversion et `ticket_projection` est étendue ensemble. La preview T16 ne résout
pas une citation historique vers la dernière version. Les limites externes
(20 sources, 8 Kio/extrait, 64 Kio cumulés) s'appliquent à l'intérieur du budget
existant total. Le détail local exact garde le corps complet retenu.

Les observations Notion/Linear de publication sont projetées depuis leurs tables
actuelles, sans nouvelle référence ni import. Leur `reference_public_id` est null
et `publication_public_id` leur job. Leur détail de source exact doit être exposé
par `GET /api/publication-observations/{id}` → `SourceObservationDetail` ; leur
actualisation reste la commande actuelle du job. Aucun nouveau POST de publication.

La projection forme des groupes contigus de snapshots métier/couverture/identité
équivalents, ordonnés par reçu dans le job. Le premier UUID du groupe est l'identité
citée par retrieval/steward ; un reçu unchanged supplémentaire ne change pas cette
identité. A→B→A forme trois groupes. Le détail d'un UUID historique de reçu reste
accessible et conserve cet UUID ; le numéro de version est l'ordinal du groupe. Un
ancien identifiant distant non décodable en UUID reste null dans ce DTO de source,
avec `freshness.eligible=false` et raison `invalid_identity` ; il ne devient pas
citable. Le reçu brut reste consultable dans l'historique de publication existant.
La fraîcheur compare l'appartenance au groupe courant, pas simplement l'ID maximal.
Root doit employer la même projection dans contexte, freshness et frontier.

La projection SQL canonique est propriété de root et expose le premier UUID du
groupe, son dernier reçu, son numéro sémantique et l'autorité du dernier reçu
attesté dans ce même état. Elle ne récupère pas une ancienne autorité après un
dernier état indisponible. Le prédicat partagé retenu est
`app.publication_observation_current(observation_id bigint)`.

Ajouter sur `publication_observations` les colonnes nullable `connection_id` et
`connection_revision` cohérentes ensemble, attestées par le serveur au retour
HTTP sous advisory connexion et recontrôle exact. Elles sont hors hash métier.
Les anciens reçus sans attestation restent historiques jusqu'à refresh/reconcile
explicite autorisé ; aucune autorité n'est reconstruite depuis une date ou le job.
`finish_publication_job` (root) atteste le receipt créé seulement si sa connexion
et sa révision restent autorisées ; il conserve sinon le résultat distant sans
l'élever en source active. Le worker Rust conserve son receipt métier actuel.

Les connexions de publications gardent leurs règles actuelles de lecture/reconcile ;
`allow_existing_reads` ne bloque pas rétroactivement l'observation d'un objet créé
par AI Center. Ce flag protège le **nouveau rattachement ciblé**. En revanche une
connexion désactivée/révision non attestée bloque l'inclusion active des observations
publiées aussi. Le périmètre/date/complétude réellement présents dans les anciens
reçus sont conservés ; les champs absents ne sont pas inventés.

Dédup du même objet dans un scope : candidat éligible le plus récent par date de
capture, puis ordre stable de type/UUID, en gardant son vrai type et sa filiation.
La sélection ne modifie aucune provenance historique. Les familles GitHub déjà
stockées restent un arbitrage distinct signalé dans la revue ; ne pas les annoncer
couvertes par ce contrat Linear/Notion.

## 8. Preuves d'assemblage attendues

- Attach Linear → observation exacte citée par PM → artefact deux tickets validé →
  deux previews T16 → pack/handoff avec même UUID et couverture ; aucun faux savoir.
- Métadonnées/texte partiels, injection, URL piégée et identité incohérente : contenu
  traité comme données, pas nouvelles instructions/actions ou vérité obligatoire.
- Même clé après réponse perdue = zéro réseau supplémentaire ; expiration = refus ;
  interruption avant commit = reprise explicite budgétée ; GET reçu = zéro réseau.
- Deux refresh, rotation/rebind/detach et révocation en vol : pas de texte sous
  autorité périmée, pas de renversement du head ni de versions dupliquées.
- Inchangé (sources et publications) = même identité sémantique, pack courant,
  compteur IA steward inchangé. Changement/retrait/inaccessibilité = invalidation
  des dépendances incluses, sans modifier leur texte historique.
- Rôles, quotas, simultanéité, cooldown après erreur, 200 références, export/purge,
  inventaires et grants prouvés sur la pile gardée par root. Aucun fournisseur réel
  ni migration exécutée par l'auteur de ce contrat documentaire.
