# Plan — GitHub dans les consommateurs de contexte

> Statut : conception, aucune implémentation T18 par ce document
> Spec liée : [spec.md](spec.md)
> Dernière mise à jour : 2026-09-30

## Approche retenue

Étendre le module de contexte observé de T17, avec trois projections GitHub sur
les tables existantes. Son interface doit retourner un candidat borné, un snapshot
exact et une attestation vérifiable. Les consommateurs n'ont pas à reconstruire
les règles GitHub individuellement. Les lecteurs distants restent distincts de
cette interface locale ; le chat et le compilateur n'effectuent aucune lecture
réseau. Préserver les chaînes de preuves historiques et les UUID existants.

Alternatives rejetées : convertir les fichiers en connaissances confirmées
perdrait leur confiance et leur fraîcheur ; recopier les observations dans
`tool_source_observations` mêlerait deux familles d'autorité ; ajouter un crawler
ou un nouvel outil de développement dépasserait ADR 0005 et le besoin.

## Points d'appui inspectés

| Zone actuelle | Réemploi / écart à fermer |
| --- | --- |
| `external_references.rs`, `integrations/github.rs`, `01_domain.sql` | Métadonnées immuables et GitHub App existants ; attestation de lecture/génération d'autorité manquante. Ne pas transformer les tableaux de chemins/checks en preuve de code. |
| `work_tools/code.rs`, `code_privacy.rs`, `09_github_code.sql` | Lectures explicites de 1–10 fichiers, commit vérifié, limites 64 Kio/fichier et 256 Kio/corpus ; révision de connexion non conservée dans le corpus. Le contrôle avant transaction doit être renforcé jusqu'au commit. |
| `work_tools/publications.rs`, `22_tool_sources.sql` | T17 atteste les reçus ; sa vue de contenu contextuel filtre Notion/Linear et exige un UUID distant. L'identité GitHub doit rester typée dépôt/issue, sans coercition vers UUID. |
| `scope_context/{observations,authority,chat}.*`, `context.rs` | Catalogue et budget communs, candidats T17 et contrôle avant commit ; deux sourcekinds et une famille d'autorité à ajouter. |
| `artifacts/`, `handoff.rs`, `service.rs`, `steward/` | Citations et provenance exactes existantes ; supprimer toute divergence de fraîcheur GitHub entre consommateurs lors de l'extension. |
| `company/{graph_*,project_export_sources.sql,export_columns.rs}` | Nœuds et exports de snapshots déjà présents ; compléter les sources directes des model runs et les nouvelles attestations utiles, jamais les credentials. |

Ces chemins sont des constats sur le travail local en cours, pas des preuves de
recette. Recontrôler leurs interfaces après gel de T17 avant de modifier le code.

## Découpage et ordre de réalisation

### 1. Contrat et premier parcours

- Figer les DTO d'observation/provenance, dont type de source, couverture,
  `authority_kind`, commit/chemin/lignes optionnels selon la famille et identité
  d'issue liée au dépôt. Étendre les enums existants ; ne pas créer de faux IDs.
- Préparer un scénario **[FICTIF]** combinant règle société, métadonnées PR, fichier
  observé et issue publiée ; écrire le test du parcours GHC-002 avant extension.
- Distinguer snapshot historique consultable et candidat courant sélectionnable.
  Ajouter un GET exact par UUID pour les métadonnées : le lecteur actuel renvoie
  seulement la dernière observation. Réutiliser le couple corpus/fichier exact
  déjà disponible pour le code.
- Préserver les citations IA par UUID ; calculer côté serveur le passage transmis
  et ses lignes. Les ancres rendues viennent de cette provenance, jamais de
  numéros libres dans le texte du modèle. Figer l'identité dépôt/type/numéro ou
  dépôt/commit/chemin sans modifier rétroactivement les DTO et packs T17.

### 2. Autorité de lecture et persistance additive

- Ajouter une génération monotone aux connexions GitHub App pour les changements
  d'autorité (statut, installation, capacités et configuration d'accès), sans
  utiliser `updated_at` comme preuve de révocation. Le provisionnement existant
  doit respecter cette génération et ne pas la réinitialiser. Distinguer une
  modification d'affichage d'une modification d'autorité.
- Conserver l'attestation ayant permis chaque nouvelle lecture : connexion et
  génération pour les métadonnées ; connexion et révision pour le corpus de code.
  Les champs historiques restent non attestés. Pour une métadonnée inchangée,
  conserver séparément le reçu de vérification courant afin de ne pas réécrire
  le snapshot. Une réponse 304 ne réatteste que la même identité, effectivement
  vérifiée sous l'autorité courante.
- Comparer aussi sous verrou le head capturé avant HTTP avec le head actuel.
  Refuser une finalisation dépassée, y compris pour 304/réattestation, au lieu
  de permettre à une ancienne réponse de remplacer une lecture déjà terminée.
- Étendre le contrôle T17 aux deux familles avec un discriminateur explicite.
  Ordre commun : projets triés, autorités triées par famille/identité, objets et
  reçus ; aucun verrou/transaction SQL conservé pendant HTTP ou appel modèle.
  Les mutations de connexion participent au même protocole. Revérifier rôle,
  périmètre autorisé et révision dans la transaction finale de lecture.
- La configuration opérateur GitHub App (application, installation/capacités) doit être
  prise en compte dans l'autorité effective sans stocker ni exposer la clé privée.
  Une modification qui retire l'accès ne peut laisser un ancien candidat actif.
  Aucune liste locale de dépôts n'existe à réemployer : ne pas inventer cette
  capacité. Un retrait distant reste inconnu avant une vérification GitHub.
- Modifications SQL déclaratives puis migrations générées et relues : clés
  étrangères société/scope, sources exclusives dans artefact/pack, RLS forcée,
  droits minimaux et fonctions privées. Pas de migration destructrice ni de
  rétro-attestation. Si une vue est ajoutée, vérifier `security_invoker` réellement.

### 3. Catalogue GitHub local, budgets et fraîcheur

- Ajouter les projections locales aux mêmes `load`, `enrich` et `snapshot` que
  T17 ; généraliser l'identité de déduplication au type et dépôt. Garder un seul
  budget externe entre tous fournisseurs et sourcekinds.
- Métadonnées : dernière observation réellement disponible, titre/état/SHA/checks
  observés, couverture `metadata_only`. Code : dernière lecture pertinente pour
  le chemin, statut `code_read`, corpus vérifié et autorité valide ; extrait par
  lignes entières, hash exact, couverture du dépôt toujours `selected_file_only`.
  Publication : reçu GitHub exact et attestation valide, contenu textuel disponible,
  filiation version/index du ticket, sans changer les marqueurs de réconciliation.
- Stabiliser l'identité contextuelle des lectures consécutives identiques
  (même commit, contenu et couverture pour le code), tout en conservant tous les
  reçus. Réutiliser le principe des groupes contigus T17 : A → B → A ne fusionne
  pas le dernier A avec le premier. Le groupe courant prend son autorité du reçu
  effectivement vérifié ; un ancien reçu non attesté n'est pas réécrit.
- Centraliser les prédicats de fraîcheur utilisés par chat, packs et steward.
  Les sources incluses devenues invalides périment leurs dépendances ; les autres
  lectures n'invalident pas tout le projet après coup. Les versions de graphes
  restent le garde-fou des opérations en cours. Indexer les recherches par
  référence/chemin/groupe et éviter une reconstruction du catalogue par ligne.

### 4. Consommateurs et navigation

- Étendre types de citation, validation des UUID et plages, sources d'artefacts,
  compilateur et hash/export du pack. Préserver l'observation originale pendant
  les conversions PM → lead/dev et la publication de tickets.
- Capturer puis vérifier l'autorité dans chaque chemin de résultat : réponse
  valide ou invalide de chat/artefact, compilation/handoff et steward. Une sortie
  annulée faute d'autorité ne doit pas laisser son texte dans un enregistrement
  technique du modèle. Ne pas réintroduire une ancienne source via un pack.
- Remplacer les branches GitHub divergentes du steward par les mêmes projections
  de disponibilité et provenance ; conserver son parcours borné, ses quotas et
  son absence de garantie exhaustive. Ne pas élargir ses appels.
- Ajouter aux lecteurs/citations les libellés métier de la spec, commit abrégé
  avec SHA complet accessible, chemin/lignes, omissions et lien canonique exact.
  Préserver l'accès clavier/mobile et la distinction observation/historique.
- Compléter les exports directs de model runs, les références interprojets, les
  inventaires d'effacement et les vérificateurs de privilèges pour les ajouts.
  Les enregistrements sans lien avec le projet exporté restent exclus.

### 5. Recette, revue et livraison

- [ ] Tests unitaires significatifs : identité typée, projection sans instruction,
  budget commun, UTF-8/lignes, ancres calculées et UUID de citations inconnus.
- [ ] Contrats HTTP existants : aucune extension des capacités, relecture et
  révocation concurrente, échecs inchangés, nombre de requêtes borné.
- [ ] PostgreSQL isolé : GHC-001 à 009, migration depuis historiques non attestés,
  RLS/FK, rôle runtime, export/purge et parcours vertical complet.
- [ ] Barrières IA : rotation/révocation/réactivation pendant chat, artefact,
  handoff et steward ; succès et sortie invalide, aucun texte sauvegardé.
- [ ] Navigateur sur API/DB locale : GHC-002/003/010 avec fixtures explicitement
  fictives ; citation après changement de commit et état inaccessible.
- [ ] Non-régression T16/T17, publications et ancien suivi Alpha Context Proof ;
  vérifications de compilation, format, lint et contrôles pertinents du dépôt.
- [ ] Revue indépendante des permissions, fraîcheur, sélection et filiation ;
  corrections puis preuves du candidat final consignées dans ce dossier.
- [ ] Mise à jour de la documentation produit validée et du suivi par le
  coordinateur ; commit, push et CI de la PR avec limites de preuve explicites.

## Mise en service et retour arrière

Appliquer les migrations additives puis le serveur et le client compatibles,
sur un environnement de recette sauvegardé. Les anciennes observations restent
consultables mais portent « à vérifier avant réutilisation ». Aucune lecture
distante de rattrapage automatique ne se lance lors de la migration ou du démarrage.

Avant ouverture, qualifier les trois familles sur les ressources de test
autorisées de CC-G2, puis les exigences d'exploitation/pilote CC-G3/G4. Si le
candidat échoue, arrêter les nouvelles générations et lectures GitHub concernées
et revenir au binaire compatible avec le schéma additif ; conserver observations,
reçus et audit. Ne pas supprimer des données ni modifier rétroactivement les packs.
