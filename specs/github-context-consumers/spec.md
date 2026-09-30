# Spécification — GitHub dans le contexte des équipes

> Statut : conception, avant implémentation
> Responsable : coordination Company Context V1
> Dernière mise à jour : 2026-09-30
> Séquence : T18, après qualification de [T17](../existing-tool-context/spec.md)

## Intention et résultat attendu

Le PM et le lead doivent pouvoir s'appuyer sur le travail GitHub déjà observé,
sans recopier son contenu dans des connaissances confirmées. Les observations
existent dans le graphe et le steward, mais ne rejoignent pas encore le catalogue
partagé du chat, des artefacts et des ContextPacks. Ce lot ferme cette rupture du
cycle **travail externe → contexte partagé → prochain livrable**.

Exemple **[FICTIF]** : une règle société impose de conserver les données trente
jours ; une PR et `src/retention.rs` ont été observés au commit exact A. Le PM
questionne l'écart, prépare une spécification citant ces observations, puis
transmet son contexte au lead. Le lead retrouve le fichier, le commit et le
passage réellement lus. Une lecture ultérieure au commit B rend à réexaminer les
dépendances de A, sans modifier leurs citations historiques ni exécuter du code.

## Contexte, décisions et écarts constatés

- [ADR 0005](../../docs/decisions/0005-context-control-not-tool-replacement.md) :
  GitHub reste canonique ; AI Center ne devient pas un IDE ou un runner.
- [ADR 0007](../../docs/decisions/0007-company-context-graph.md) : portées autorisées,
  provenance exacte et invalidation des sources incluses.
- [ADR 0008](../../docs/decisions/0008-observed-tool-context.md) : données observées,
  autorité de lecture et historique immuable ; les familles de connexions restent
  séparées. T18 étend les consommateurs GitHub, pas le rattachement Notion/Linear.
- [Company Context V1](../company-context-v1/spec.md), CC-020/023/029/031/043/054 ;
  [parcours web](../../docs/product/07-company-context-web.md).
- Écart de réalisation : `ContextSourceKind` et `scope_context/observations.sql`
  n'admettent que les observations T17 Notion/Linear. Les champs historiques de
  sélection GitHub ne prouvent pas que le parcours web courant les utilise.
- Écart d'autorité : les références GitHub App ont une connexion, mais pas de
  génération d'autorité enregistrée ; les corpus de code n'enregistrent pas la
  révision du secret ayant permis la lecture. T17 atteste les publications, mais
  sa projection de contexte exclut GitHub. Ajouter seulement trois branches de
  sélection ne satisferait donc pas ADR 0008.
- Aucun changement des décisions durables n'est nécessaire. La limite historique
  « métadonnées seulement » d'Alpha Context Proof a déjà été amendée explicitement
  par Company Context V1 ; elle ne doit pas être réintroduite pour éviter le code
  ciblé, ni interprétée comme une autorisation de parcourir tout le dépôt.

## Périmètre

**Inclus** : observations existantes de dépôt/PR/commit, fichiers choisis à un
commit vérifié, issues créées par les publications GitHub existantes ; sélection
pour les agents globaux et projet ; citations, artefacts, packs, relais PM → lead
→ dev, graphe, steward, export et effacement correspondants. Les lectures restent
les commandes explicites actuelles, renforcées pour attester leur autorité.

**Exclus** : crawl, clone, nouvel import générique d'issues, recherche de compte,
nouveau fournisseur, élargissement des permissions distantes, commentaires ou
diffs non déjà observés, synchronisation automatique, exécution et certification
du dépôt. Aucun accès distant supplémentaire n'est déclenché par une conversation,
un export, une citation ou le montage d'une page.

## Identités et provenance

| `source_kind` | Source immuable et contenu admissible | Provenance spécifique obligatoire |
| --- | --- | --- |
| `external_reference_observation` | Projection de métadonnées effectivement conservée pour un dépôt, une PR ou un commit. | Référence, type, dépôt, identité canonique, état observé ; base/head/commit et contrôles seulement s'ils existent dans le snapshot. `metadata_only`, aucune ligne de code inventée. |
| `github_code_file_observation` | Texte d'un fichier `code_read`, dans un corpus au commit vérifié. | Corpus, dépôt, SHA complet du commit, chemin, SHA du blob, hash du contenu, nombre de lignes, passage effectivement transmis et ses lignes exactes. `selected_file_only` pour le dépôt. |
| `publication_observation` avec `provider=github` | Titre et corps observés d'une issue issue d'une publication existante. | Dépôt et identité d'issue, publication, version d'artefact et index de ticket conservés ; aucune assimilation de l'issue à une PR ou à une preuve de code. |

Chaque source conserve aussi son UUID exact, sa société et son scope d'origine,
son URL canonique, les dates disponibles, son hash de projection, sa couverture,
les raisons d'omission et l'autorité utilisée. Les dates absentes restent nulles.
`observed_external` et `mandatory=false` restent vrais après validation d'un
artefact. Le numéro d'observation local n'est jamais présenté comme un numéro de
version GitHub. La date de lecture ne désigne pas la date du commit.

Une citation ouvre le snapshot exact, même après nouvelle lecture. Pour le code,
le lien externe utilise le SHA complet et le chemin encodé ; ses ancres ne couvrent
que les lignes transmises. Un extrait privilégie des lignes entières et indique
sa troncature ; une ligne trop longue ne devient pas une ligne complète fictive.
Un fichier vide est signalé comme tel, sans inventer une plage de lignes.

Les citations IA restent des UUID de sources. Les plages et ancres sont calculées
par le serveur à partir de l'extrait transmis, puis affichées avec la provenance.
Ce lot ne certifie pas les numéros de ligne éventuellement écrits dans le texte
libre du modèle. Une future plage choisie par l'IA exigerait un contrat structuré
et une validation distincts. Les métadonnées historiques disposent d'un lecteur
exact par UUID d'observation ; la route retournant seulement la dernière lecture
d'une référence ne satisfait pas cette exigence.

## Autorisation, fraîcheur et limites de connaissance

1. La société reste la frontière d'accès. Les scopes parcourus et les liens
   interprojets sont ceux de T17 ; un lecteur étranger ou un membre révoqué
   n'accède ni aux snapshots ni aux citations. Les agents n'ajoutent aucun droit.
2. L'autorité distingue explicitement `github_app` (`tool_connections`) et
   `work_tool` (`work_tool_connections`). Une connexion active aujourd'hui ne
   prouve pas l'autorisation d'une lecture historique. La révision ayant permis
   la lecture et l'identité d'installation/capacité pertinentes sont attestées.
   Les observations anciennes non attestées restent historiques jusqu'à une
   nouvelle vérification autorisée ; aucune migration ne leur attribue une preuve.
3. Rotation, révocation, changement d'installation ou de périmètre gouverné
   empêchent une nouvelle utilisation avant relecture. Les droits et l'attestation
   sont capturés avant l'appel IA et revérifiés sous verrou avant tout résultat
   persistant, y compris une sortie modèle invalide. La perte d'autorité annule
   l'opération sans conserver le texte de sortie dans le modèle, le chat ou un
   artefact. Cette règle couvre aussi le steward et les générations via un pack.
4. « Courant » signifie dernière observation admissible connue localement, jamais
   état temps réel de GitHub. Une relecture identique sous la même autorité garde
   la même identité contextuelle, sans invalider les packs ni réveiller inutilement
   le steward. A → B → A constitue trois états successifs. Une nouvelle attestation
   peut réautoriser le même contenu, mais ne valide pas un appel commencé sous
   l'ancienne autorité.
5. Un nouveau commit ou un état indisponible remplace le contexte courant du
   chemin explicitement relu. Lire d'autres chemins ne retire pas ceux qui n'ont
   pas été demandés. Le snapshot historique reste lisible sous les droits locaux.
   Deux lectures concurrentes ne peuvent renverser leur ordre de finalisation :
   la seconde compare le head capturé avant HTTP au head courant sous verrou,
   y compris pour une réponse 304 et une réattestation.
6. Métadonnées, fichiers absents/inaccessibles, erreurs et parties omises sont
   des limites de lecture ; ils ne démontrent pas l'absence d'une implémentation.
   Un SHA présent dans une PR ne prouve pas que le fichier lu ailleurs appartient
   à ce SHA. Le code n'est une preuve que du texte observé, pas de son exécution,
   de tests réussis ni de la conformité de tout le dépôt.
7. Aucun texte GitHub ne devient une instruction système, une règle confirmée,
   une capacité d'outil ou un ordre de publication. Les protections existantes
   contre l'import de secrets et les contenus exclus restent appliquées.

L'autorité opérateur locale correspond aux paramètres effectivement configurés
(application, installation et capacités). Les retraits distants de droits GitHub
ne sont connus qu'après une vérification distante ; ce lot ne promet pas leur
détection immédiate et n'invente pas une liste locale de dépôts autorisés.

## Sélection, dépendances et expérience

GitHub partage le budget externe T17 : **20 sources, 8 Kio d'extrait par source,
64 Kio au total**, dans l'enveloppe de contexte existante de **160 000 octets**.
Ce ne sont pas vingt sources supplémentaires par fournisseur. Les règles société
obligatoires restent prioritaires ; les omissions et leur raison sont visibles.

La déduplication distingue projet, fournisseur, type et identité canonique : une
issue 42 dans deux dépôts, une issue et une PR, des métadonnées et un fichier ne
s'écrasent pas. Pour un fichier, la filiation identifie toujours le commit et le
chemin exacts ; les lectures identiques d'un même état peuvent partager une seule
identité contextuelle, sans réécrire ni supprimer les reçus d'origine.

Chat, artefacts, compilateur, pack transmis et steward réutilisent les mêmes
conditions de disponibilité et d'autorité. Une source incluse modifiée, rendue
indisponible ou révoquée rend ses dépendances à réexaminer ; une source exclue ou
une lecture identique ne périme pas un pack indépendant. Le graphe et l'export
conservent les mêmes UUID, empreintes et filiations, y compris interprojets. Un
export ne collecte pas les autres conversations ou les secrets des connexions.

L'interface indique « métadonnées observées », « fichier observé au commit… » ou
« issue publiée observée », avec date, couverture et accès au snapshot historique.
Une source exclue faute d'attestation propose la vérification explicite appropriée.
Une citation introuvable est signalée ; elle n'ouvre jamais silencieusement la
dernière observation d'un autre commit. Aucun jargon de stockage n'est requis.

## Critères d'acceptation et preuves

| ID | Attendu | Preuve attendue |
| --- | --- | --- |
| GHC-001 | Les trois familles alimentent les mêmes consommateurs, avec identité, confiance et scope exacts. | Tests de sélection chat/pack et sources d'artefacts ; aucune requête GitHub pendant leur consommation. |
| GHC-002 | Un parcours PM → spécification → tickets produit → pack → plan/tickets techniques conserve les trois filiations, dont le code au SHA et aux lignes exacts. | Scénario vertical PostgreSQL + modèle déterministe **[FICTIF]**, puis navigateur avec API réelle locale. |
| GHC-003 | Une issue GitHub publiée garde version/index et reçu ; un document sourcé remonte jusqu'au contenu externe réellement observé. | Régression T16/T17 et navigation historique après nouvelle version. |
| GHC-004 | Révocation, rotation, réactivation, changement de périmètre et perte d'accès en vol ne valident pas un résultat sous l'ancienne autorité. | Barrières concurrentes avant/après HTTP et IA, succès/sortie invalide ; absence de texte persistant. |
| GHC-005 | Historiques sans attestation exclus du contexte courant, puis utilisables après relecture autorisée ; aucune preuve fabriquée à la migration. | Fixture de mise à niveau et vérification des snapshots/reçus historiques inchangés. |
| GHC-006 | Même état inchangé, A → B → A, nouveau commit, chemin indisponible et chemin non demandé ont les effets définis sur les dépendances incluses. | Tests packs, graphe et steward ; compteurs d'appels et identité de citation. |
| GHC-007 | Métadonnées seules, code partiel/vide, chemin absent, injection et CI observée ne produisent aucune preuve automatique d'absence, d'exécution ou de conformité. | Contrats de projection, ancres serveur limitées aux lignes transmises, UUID inconnus refusés et évaluations synthétiques. |
| GHC-008 | Budgets partagés, UTF-8, lignes, collisions d'identité et obligations société respectés. | Tests de sélection aux limites et d'extraits avec lignes longues/multioctets. |
| GHC-009 | Isolation, exports, purge gouvernée et provenance interprojets restent exacts sans collecte voisine ni secret. | RLS/FK, export et effacement sur base isolée ; inventaires runtime et maintenance. |
| GHC-010 | Snapshot et filiation accessibles au clavier et sur mobile, sans lecture distante automatique ni substitution historique. | Parcours navigateur et revue de l'affichage, erreurs/révocation comprises. |

## Risques et validation

T17 doit être qualifié avant ce lot. Les tests locaux utilisent des données et
des serveurs HTTP **[FICTIF]** ; ils ne démontrent pas une lecture d'un compte réel.
La preuve distante exige les dépôts, comptes et budgets autorisés de CC-G2 ;
l'exploitation et le pilote restent CC-G3/G4. Ce document ne ferme aucun gate.
Le [plan](plan.md) détaille les modifications minimales et la recette attendue.
