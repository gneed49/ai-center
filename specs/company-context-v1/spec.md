# Spécification — Company Context V1

> Statut : active — implémentation autorisée, acceptations en attente  
> Responsable : propriétaire produit et équipe de réalisation AI Center  
> Dernière mise à jour : 2026-09-21

## Intention

AI Center devient l'espace web partagé d'une entreprise pour préparer, conserver
et transmettre le contexte de son travail. Une équipe configure sa société,
retrouve ses projets, échange avec des agents adaptés à son métier, produit des
artefacts organisés et relie les résultats de ses outils existants. Chaque
projet possède un graphe ; ces graphes forment une vue d'entreprise contrôlée
par les permissions. Un agent de cohérence asynchrone fait ressortir les
contradictions et les écarts entre décisions, spécifications, tickets et
éléments de code effectivement observés.

Le propriétaire a explicitement demandé le 21 septembre de réaliser tous les
lots en autonomie, avec développement, tests, revue et préparation à la
production. Cette spécification formalise cette autorisation ; elle n'ajoute
pas de nouvelle attente d'approbation du plan avant développement. Les achats,
offres payantes et consommations externes sans budget autorisé, les accès
absents et le temps d'usage humain restent des dépendances distinctes.

## Résultat attendu

Une personne configure sa société, invite ses collègues et crée deux projets.
Les fonctions commerciales, produit et techniques utilisent la même instance.
Depuis l'espace société, elles interrogent un agent généraliste sur les
connaissances qui leur sont accessibles. Dans un projet, elles choisissent un
agent PM, Commercial, Lead technique ou Développeur, reprennent leurs échanges
et produisent un kickoff, une spécification ou des tickets versionnés.

Les artefacts sont classés par projet et type. Leur destination peut être AI
Center, Notion, Linear ou GitHub selon les capacités du connecteur configuré.
L'agent PM prépare les spécifications et tickets produit ; le lead technique
prépare les tickets techniques et le contexte d'implémentation. Le code est
réalisé dans les outils externes. Une référence GitHub peut ensuite être reliée
à un ticket et une exigence. Le graphe permet d'expliquer ces liens, leurs
versions et leurs sources. L'agent de cohérence signale un écart sourcé,
propose une action et laisse une personne confirmer sa résolution.

## Contexte et sources

- [Instructions du dépôt](../../AGENTS.md) et modèles `specs/_template/`.
- [Vision produit](../../docs/product/02-product-vision.md),
  [personas](../../docs/product/03-personas-and-use-cases.md),
  [périmètre précédent](../../docs/product/04-mvp-scope.md).
- Décisions applicables : [clients et serveur](../../docs/decisions/0001-cross-platform-control-plane.md),
  [persistance](../../docs/decisions/0002-supabase-postgres-persistence.md),
  [moteur serveur](../../docs/decisions/0003-server-side-agent-engine.md),
  [outils externes canoniques](../../docs/decisions/0005-context-control-not-tool-replacement.md),
  [connexions IA personnelles](../../docs/decisions/0006-personal-provider-connections.md),
  [graphe d'entreprise](../../docs/decisions/0007-company-context-graph.md).
- Socle conservé : [Alpha Context Proof](../alpha-context-proof/spec.md),
  [connexions IA](../provider-connections/spec.md),
  [preuves synthétiques](../synthetic-context-cases/spec.md).
- [État documenté](../../docs/current-state-audit.md) et
  [dossier opérationnel](../../docs/operations/private-alpha-handoff.md).

### Amendements explicites au périmètre antérieur

| Décision ou limite antérieure                                                    | Règle Company Context V1                                                                                                                                                                                                                                                |
| -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Utilisateur unique, entreprise multi-projets différée                            | Société/workspace partagé, membres et projets font partie de la livraison.                                                                                                                                                                                              |
| Template limité à Produit et Tech                                                | Profils génériques société et PM/Commercial/Lead technique/Développeur par projet. Les projets existants gardent leurs données.                                                                                                                                         |
| Fédération inter-projets, fonctions internes et personnalisation différées       | Graphe fédéré au sein de la société, destination interne d'artefacts et réglages des destinations inclus.                                                                                                                                                               |
| Premier connecteur uniquement GitHub en lecture, aucun fichier/diff importé      | Notion, Linear et GitHub ont des capacités explicitement limitées. Lecture ciblée de sources GitHub autorisées pour analyse d'écarts, sans exécution de code. Publication documentaire ou de tickets seulement par action explicite et dans une destination configurée. |
| Aucun commentaire, ticket ou document externe créé par l'application             | Les capacités d'écriture déclarées des destinations sont permises pour publier un artefact validé ; aucune écriture arbitraire ni production de code par AI Center.                                                                                                     |
| E2E et essais de comptes réservés au propriétaire dans les lots du 7–8 septembre | Les tests E2E, exploration UI et recettes nécessaires au nouvel objectif sont inclus dans le travail autorisé. Les accès réels et budgets manquants restent à obtenir par les moyens sécurisés.                                                                         |
| Client Tauri et smoke natif nécessaires au jalon Alpha                           | La V1 cible le navigateur ; le natif est conservé sans devenir un gate de cette livraison.                                                                                                                                                                              |
| Campagne comparative et semaines d'usage requises avant promotion Alpha          | Les preuves Alpha restent attachées à ce jalon. La V1 possède les gates ci-dessous ; aucun ancien gate n'est déclaré clos implicitement. Les pilotes réels restent nécessaires à la qualification opérationnelle.                                                       |
| `AGENTS.md` décrit encore les frameworks comme futurs                            | Le travail conserve React/Vite, Rust/Axum et PostgreSQL/Supabase déjà choisis par les décisions acceptées.                                                                                                                                                              |

La doctrine de production externe, la confirmation humaine des connaissances,
la provenance, l'isolation et l'absence de secret dans les clients restent
applicables. Les décisions durables et la documentation produit seront
alignées au cours des tickets concernés, sans modifier le corpus historique
ni les fichiers synchronisés.

## Périmètre

### Inclus

- Configuration guidée de la société, accès, rôles, membres et projets.
- Espace général de l'entreprise et espaces de projet ; conversations durables
  dont le scope et le profil d'agent sont visibles.
- Profils d'agents configurés comme données, avec instructions métier,
  connaissances accessibles et contrats de livrables.
- Artefacts versionnés, organisés, modifiables avant validation et reliés aux
  conversations, connaissances et exigences qui les fondent.
- Destinations internes et connecteurs Notion, Linear et GitHub limités et
  configurables ; état des publications et synchronisations explicite.
- Graphe de projet et graphe fédéré d'entreprise en base, dans les services,
  les compilations de contexte et l'interface graphique.
- Steward asynchrone sur événements, analyse prudente des contradictions et de
  la couverture, résolution humaine et invalidation ciblée.
- Fiabilité, coûts/quotas, administration, données, exploitation web et
  qualification fonctionnelle jusqu'aux gates réellement exécutables.

### Exclu

- IDE, terminal, exécution de code distant, édition autonome de dépôt,
  réalisation ou déploiement du code client par AI Center.
- Suite remplaçant Notion, Linear ou GitHub ; synchronisation exhaustive de
  toutes leurs fonctions ou de tous les contenus d'un compte.
- Fédération entre sociétés indépendantes, transfert implicite d'un projet
  privé vers le contexte global, autonomie de publication du steward.
- Application mobile, certification native, marketplace, SSO/SCIM et paiement
  public automatique dans cette livraison.
- Garantie de conformité du code ou de détection exhaustive, zéro faux
  positif, résultat commercial, tarif ou disponibilité fournisseur inventés.

## Modèle et invariants

1. **Société** désigne le workspace existant, frontière de données et de
   membres. Un client indépendant possède sa propre société ; aucune seconde
   hiérarchie « société » n'est ajoutée sans besoin métier distinct. Son espace
   général utilise un conteneur unique `projects.scope_kind = 'company'`,
   conservant les contraintes `project_id` des sessions et versions. Ce
   conteneur est absent des listes de projets métier ; ceux-ci portent le type
   `project`. L'interface n'affiche jamais un faux projet « société ».
2. **Projet** appartient à une société. Les projets V1 sont partagés avec les
   membres de cette société selon leurs rôles owner/editor/viewer ; aucune
   permission privée par projet n'est promise par une simple sélection de
   scope. Une société indépendante est nécessaire pour isoler un autre client.
   Toute restriction plus fine ajoutée ultérieurement devra s'appliquer aussi
   aux compilations, agents, artefacts et graphes. Les rôles métier d'un agent
   ne sont jamais des permissions d'utilisateur.
3. **Scope de contexte** vaut société ou projet. Une conversation conserve son
   scope, son profil et les versions utilisées ; changer de scope ouvre ou
   reprend une conversation de ce scope au lieu de déplacer silencieusement
   l'historique. Un agent global n'obtient pas davantage de droits que l'acteur.
   Les clés de profils sont `general`, `product` (PM), `sales`, `tech` (lead
   technique) et `dev` (rôle développeur). Le profil `tech` conserve le ContextPack explicite
   requis par le handoff ; l'ajout des autres profils ne contourne pas ce gate.
4. **Connaissance** reste typée, confirmée, sourcée et versionnée. Une réponse
   de chat, un artefact brouillon et un signal du steward ne deviennent pas des
   vérités confirmées par leur seule génération.
5. **Graphe** est une projection des entités et arêtes persistées. Les arêtes
   inter-projets demeurent dans une société, portent leur provenance et ne
   deviennent visibles que si les deux extrémités sont autorisées. Les
   agrégations ne révèlent pas les noms, nombres ou contenus privés.
6. **Artefact** possède un type, un scope, un auteur, une filiation, des versions
   immuables et des sources. Ses révisions, publications et destinations sont
   distinctes : changer de destination ne réécrit ni la version ni l'historique.
7. **Destination** expose ses capacités réelles : interne ; page Notion ; issue
   Linear ; référence ou issue GitHub selon configuration. Une destination ne
   remplace pas un type de livrable. Un type non pris en charge est refusé ou
   reste interne avec explication, jamais annoncé comme publié.
8. **Publication externe** relie une version AI Center à un objet canonique
   distant avec identifiant, URL, version observée et état. Le contenu courant
   externe reste canonique dans son outil ; le snapshot interne conserve ce
   qui a été transmis ou observé. Un conflit est visible et n'écrase pas une
   modification distante à l'insu de la personne.
9. **Preuve** distingue proposition, observation, source manquante, validation
   humaine, obsolescence et indisponibilité. Un test vert ou une présence de
   code ne prouve pas à lui seul qu'une exigence est satisfaite.
10. **Travail asynchrone** porte société, scope, acteur et identité logique.
    Avant exécution ou publication, l'autorisation est recontrôlée. Retirer un
    membre ou une connexion empêche un job différé d'utiliser ses anciens droits.

## Parcours et comportements

### CC-U01 — Configurer la société et rejoindre l'équipe

Le premier propriétaire authentifié crée ou retrouve sa société, renseigne
son nom et choisit les fonctions utiles. Il configure un chemin IA disponible
et les destinations ; il peut terminer ce choix plus tard sans perdre sa
saisie. Il invite des collègues, leur attribue un rôle et suit les invitations.
Un lien expiré ou un compte sans accès affiche une action compréhensible.
Le retrait d'un membre agit sur ses requêtes suivantes et travaux différés.

### CC-U02 — Travailler dans l'espace général

Une personne ouvre un agent généraliste, commercial ou produit de la société.
L'écran explique ce qu'il peut consulter. Les réponses citent les sources
accessibles ; une demande concernant un projet privé ne révèle aucune donnée
ni sa présence, notamment s'il appartient à une autre société. La V1 ne
présente pas les projets partagés de la même société comme privés. Une
proposition utile peut devenir une connaissance générale
après confirmation, avec sa provenance et une portée de partage explicite.

### CC-U03 — Passer du cadrage au travail technique

Dans un projet, l'agent PM aide à comprendre et brainstormer, propose kickoff,
spécification et tickets produit. Après validation, l'agent Lead technique
reçoit un contexte sourcé et produit un plan et des tickets techniques. Les
agents Commercial et Développeur lisent le contexte autorisé selon leur rôle,
challengent et produisent leurs artefacts. L'agent Développeur prépare ou
explique le travail ; l'implémentation se déroule dans un outil externe.

### CC-U04 — Organiser et publier les artefacts

Une bibliothèque par projet expose types, versions, auteur, état et destination.
La personne choisit les destinations par défaut, peut les remplacer avant
publication et prévisualise le contenu transmis. Une action de publication
explicite crée ou met à jour l'objet distant autorisé. Le lien, la version et
l'état sont restitués. Retry, réponse perdue, expiration et conflit ne créent
pas des doublons silencieux. Sans connexion valide, l'artefact reste disponible
en interne et exportable ; aucune publication distante n'est simulée.

**Amendement de granularité — tickets distincts :** pour un artefact structuré
de tickets produit ou techniques, une sélection humaine de N entrées vers
Linear/GitHub produit N demandes de création distinctes et, si elles réussissent,
N issues avec leurs propres liens et états. Le document ne doit pas devenir
implicitement une seule issue contenant N tickets. Notion conserve la publication
du document complet. L’identité d’une entrée est sa version source + son index ;
une nouvelle version ne met pas à jour ni ne recrée silencieusement les anciennes
issues. Prévisualisation, reprise, quotas et filiation sont précisés dans
[la spécification Tickets distincts](../ticket-publication/spec.md).
Cette précision répond au parcours utilisateur ; elle n’était pas couverte par
le précédent critère de publication documentaire. Le lot reste à implémenter
après le gel d’intégration et participe à la clôture de CC-G1.

### Amendement — Partir du contexte existant dans les outils

Le parcours doit également fonctionner lorsque la page Notion et l'issue Linear
existent avant AI Center. L'audit du 23 septembre confirme que les connecteurs
actuels relisent uniquement leurs propres publications ; les observations
externes visibles dans le graphe/steward ne sont pas encore des sources directes
du contexte PM/génération. C'est un écart P1 de l'objectif utilisateur, omis des
premiers critères documentaires CC-025 à CC-027.

Le lot [Contexte issu des outils existants](../existing-tool-context/spec.md)
précise le rattachement explicite d'un objet, sa lecture bornée, son actualisation
et son observation exacte réutilisable dans les questions, artefacts, transmissions
et graphes. La confiance reste « observé dans un outil externe » ; elle ne devient
jamais implicitement une règle confirmée. Aucun crawl, import massif, nouvelle
intégration ou synchronisation globale n'est ajouté. Ce lot est conçu après l'audit
et se réalise après Tickets distincts ; il participe également à CC-G1.

### CC-U05 — Comprendre les graphes

La personne navigue de la société au projet et sélectionne une décision,
exigence, conversation, artefact, ticket ou preuve. Un panneau explique les
liens, versions et sources. Recherche et filtres limitent le graphe visible.
Une liste accessible offre la même information et les mêmes actions. La
navigation ne nécessite pas de comprendre le terme « graphe » pour travailler.

### CC-U06 — Examiner une contradiction ou un écart

Une connaissance confirmée, un artefact révisé ou une nouvelle observation
externe déclenche un travail asynchrone durable. Le steward compare les seules
sources autorisées et disponibles. Exemple fictif : une spécification interdit
l'expiration des crédits, un ticket Linear prévoit 90 jours, et un extrait
GitHub observé mentionne une purge. Il cite chaque version et emplacement et
signale un écart potentiel ; un nom de fichier ou des métadonnées seules ne
deviennent pas une preuve du comportement exécuté. Une source manquante donne
« impossible à vérifier ». La personne rejette, accepte temporairement ou
résout le signal avec une révision ; les projections dépendantes sont marquées
à réexaminer. Le steward ne corrige pas automatiquement le code ou les outils.

### CC-U07 — Reprendre et quitter sans perte

Un message lent conserve son identité et son état après rechargement. La
personne retrouve conversations et brouillons, peut réessayer sans doublon et
voit si un service est indisponible. L'opérateur peut suspendre les appels,
exporter une société ou un projet et réaliser une suppression ciblée selon la
politique de rétention. Une restauration séparée rétablit données, droits et
accès aux clés sans envoyer de nouveaux messages ou appels externes.

## Critères d'acceptation

Toutes les lignes sont **en attente** à la création de cette spécification.
Leur numérotation est stable ; l'implémentation ne vaut pas acceptation.

| ID     | Exigence vérifiable                                                                                                                                                         | Preuve minimale                                                                                                                               |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| CC-001 | Créer/retrouver une société et terminer un setup reprenable sans doublon.                                                                                                   | API/DB et E2E premier propriétaire, reload et retry.                                                                                          |
| CC-002 | Inviter, accepter, réinviter, attribuer un rôle et retirer un membre dans l'interface ; invitations expirées et déjà acceptées gérées.                                      | Contrats, intégration et E2E de deux comptes ; SMTP réel pour gate externe.                                                                   |
| CC-003 | Les permissions owner/editor/viewer et l'appartenance société/projet sont appliquées à chaque lecture/mutation, y compris jobs différés ; le partage société est explicite. | Matrice RLS/API, session existante après retrait, job en attente après révocation.                                                            |
| CC-004 | Société et projet actifs sont réels, visibles et sélectionnables ; changer de société purge les données mises en cache de la précédente.                                    | E2E A/B avec URLs forgées et caches.                                                                                                          |
| CC-005 | Créer, renommer, lister et archiver un projet préserve ses historiques et ses liens.                                                                                        | Tests de concurrence, reload, accès projet archivé.                                                                                           |
| CC-010 | Profils génériques société et PM/Commercial/Lead technique/Développeur projet sont disponibles avec scope et capacités compréhensibles.                                     | API/DB et E2E de sélection de chaque profil ; aucun rôle agent ne confère de droit.                                                           |
| CC-011 | Conversations durables par scope/profil, reprises après reload ; un message ne change pas silencieusement de scope.                                                         | Intégration et E2E entre deux projets et espace société.                                                                                      |
| CC-012 | Le contexte inclut uniquement les sources autorisées, leurs versions et les contraintes obligatoires ; budget explicite, jamais dump global implicite.                      | Contrats de compilation, sources forgées et corpus multi-projets synthétique.                                                                 |
| CC-013 | Réponses et mutations proposées restent distinctes des connaissances confirmées ; rejet sans mutation.                                                                      | Tests API/DB, confirmation/révision/rejet dans l'UI.                                                                                          |
| CC-014 | Un fournisseur/modèle de référence produit effectivement conversation et artefacts contractuels ; erreur réelle jamais remplacée par une simulation.                        | Tests doubles puis génération réelle sur compte autorisé ; coût et limite consignés.                                                          |
| CC-020 | Kickoff, spécification, tickets produit, plan et tickets techniques sont structurés, organisés par projet et liés à leurs sources.                                          | Workflow PM → lead avec artefacts persistés et sources valides.                                                                               |
| CC-021 | Édition du brouillon, validation, nouvelle version, historique et comparaison sont disponibles ; versions publiées immuables.                                               | Intégration de concurrence et E2E de deux révisions.                                                                                          |
| CC-022 | Bibliothèque et recherche donnent accès à tous les artefacts/connaissances autorisés ; pagination ou chargement borné explicite.                                            | Jeu de données dépassant une page et consultation des plus anciens éléments.                                                                  |
| CC-023 | Copier/télécharger Markdown et JSON donne la version attendue, ses sources et un état d'obsolescence visible.                                                               | Comparaison des exports, reload et pack périmé.                                                                                               |
| CC-024 | Destination par défaut par type/projet, héritage société explicite et remplacement ponctuel avant publication.                                                              | Contrats de résolution et E2E changement de destination sans mutation historique.                                                             |
| CC-025 | Notion, Linear et GitHub exposent des capacités limitées, destination autorisée, état de connexion et secrets serveur ; clés non relues par le client.                      | Tests de stockage, permissions et endpoints ; interface masquée et révocation.                                                                |
| CC-026 | Publication explicite idempotente : Notion page, Linear issue, GitHub issue/référence selon capacité ; lien canonique et résultat exact conservés.                          | Contrats des trois adapters, reprise après réponse perdue, intégration ; écriture réelle limitée à destinations autorisées pour gate externe. |
| CC-027 | Une modification distante, perte d'accès ou suppression produit conflit/stale/unavailable sans écrasement ni perte d'historique.                                            | Simulations HTTP et scénario réel ciblé par connecteur.                                                                                       |
| CC-028 | Les tickets structurés sélectionnés et validés sont publiés individuellement dans Linear/GitHub ; N entrées donnent N issues réussies distinctes, avec reprise et provenance par version/index, sans doublons silencieux entre versions. | TP-001 à TP-011 de [Tickets distincts](../ticket-publication/spec.md), dont perte de réponse, quotas atomiques, états partiels et confirmation des créations supplémentaires. |
| CC-029 | Une page Notion ou issue Linear préexistante peut être rattachée explicitement, observée et actualisée ; son observation exacte est utilisable par les agents, artefacts, handoffs, graphe et steward, avec portée, date, couverture et confiance externe conservées. | ETC-001 à ETC-012 de [Contexte issu des outils existants](../existing-tool-context/spec.md), dont sources sans marqueur, droits/révocation, contenu partiel, injection, fraîcheur, export et effacement. |
| CC-030 | Les entités/arêtes persistées possèdent scope, type, provenance et versions ; aucune arête inter-sociétés acceptée.                                                         | Contraintes SQL, RLS et tests API négatifs.                                                                                                   |
| CC-031 | Un graphe projet expose connaissances, conversations, artefacts, tâches/références et preuves liés ; navigation vers les objets.                                            | Intégration projection et E2E graphe/listes.                                                                                                  |
| CC-032 | Le graphe société fédère les projets autorisés et connaissances générales sans recopier leurs sources ni révéler les objets privés.                                         | Acteurs de permissions différentes, réponses API et agrégations comparées.                                                                    |
| CC-033 | Recherche, filtres, sélection et détail sourcé fonctionnent dans la vue graphique et son alternative clavier/liste.                                                         | E2E navigateur, clavier, absence de blocage de navigation et contrôle visuel.                                                                 |
| CC-034 | Réviser une connaissance inter-projets invalide uniquement ses projections dépendantes accessibles et conserve l'historique.                                                | Graphe synthétique à deux branches/projets ; hashes et filiations.                                                                            |
| CC-040 | Le steward s'exécute via événements persistants, déduplication, bail, retries bornés et état d'échec actionnable.                                                           | Crash/reprise/concurrence, retrait de droits avant traitement.                                                                                |
| CC-041 | Les contradictions intra/inter-projets citent deux versions sources et distinguent contradiction, compatible, ambigu et données insuffisantes.                              | Corpus synthétique annoté puis cas réels revus humainement.                                                                                   |
| CC-042 | L'analyse spec/ticket/code exige des sources réellement observées, dates, SHA/version et emplacements ; source absente interdit un verdict confirmé.                        | Fixtures avec métadonnées seules, extrait incomplet, source périmée et fichier absent.                                                        |
| CC-043 | Lecture GitHub de contenu bornée aux dépôts/fichiers autorisés ; aucune exécution, secret importé, redirection arbitraire ou instruction privilégiée issue du contenu.      | Faux serveur, taille/délai, URLs/SSRF, source malveillante et contrat de permissions.                                                         |
| CC-044 | L'inbox permet inspection, rejet, acceptation temporaire justifiée et résolution par révision ; aucune correction externe automatique.                                      | E2E de chaque décision et audit différencié.                                                                                                  |
| CC-045 | La couverture sépare présence d'un lien, observation, évaluation et validation humaine ; jamais « implémenté » sur un score IA seul.                                        | Contrats et UI avec preuve candidate/stale/unavailable.                                                                                       |
| CC-050 | Identité stable des commandes, opérations longues et reprise via le proxy empêchent doublons et perte de saisie après timeout/reload.                                       | E2E réseau perturbé et intégration DB ; même identité pour retry du même travail.                                                             |
| CC-051 | Limites d'appels/concurrence par société/acteur, arrêt opérateur et suspension des connexions couvrent tous les chemins IA actifs.                                          | Refus avant appel, jobs différés, connexions personnelles et par défaut testés.                                                               |
| CC-052 | Usage et coûts connus/inconnus, erreurs et opérations en cours sont visibles sans contenu sensible ; aucun statut moteur codé en dur.                                       | Contrats et UI sans fournisseur, échec, succès, coût absent.                                                                                  |
| CC-053 | Setup, conversations, artefacts, graphes et erreurs utilisent un vocabulaire métier et un prochain geste clair.                                                             | Recette web visuelle et parcours clavier ; pas de saisie SQL/prompt technique demandée à l'utilisateur.                                       |
| CC-054 | Export complet et suppression ciblée d'une société/projet sont réalisables avec autorisation, trace et politique de sauvegarde/rétention.                                   | Exercice sur données fictives et contrôle de non-impact autre société.                                                                        |
| CC-055 | Configuration déployable, HTTPS, Auth/SMTP, secrets, migrations et retour arrière sont documentés et vérifiables.                                                           | Build, CI du commit livré et recette de la cible réellement disponible.                                                                       |
| CC-056 | Sauvegarde automatique, restauration séparée avec droits/identités/clés, readiness et alertes sont testés ; RPO/RTO définis et mesurés.                                     | Exercice daté et alerte effectivement reçue, sans envoi sortant lors de restauration.                                                         |
| CC-057 | Développement, revue corrective, tests unitaires/intégration/E2E et livraison distante portent le même commit identifié.                                                    | Rapport par lot et CI/revue du candidat, pas reprise de preuves d'un ancien HEAD.                                                             |

## Preuves attendues

- **Tests :** contrats métier/fournisseurs, PostgreSQL isolé et RLS, migrations,
  concurrency/replay, E2E navigateur sur API simulée puis API/DB réelles. Les
  intégrations utilisent uniquement la stack jetable gardée du checkout.
- **Captures/démonstration :** setup, société/projet, conversation, bibliothèque,
  publication, graphe projet/entreprise et inbox ; états vides/chargement/erreur
  et permissions. Captures sans donnée ou secret réel.
- **Artefacts :** commit, rapports datés, migrations, résultats de revue,
  inventaire des capacités par connecteur, version de schéma et instructions de
  livraison/reprise ; fixtures explicitement `[FICTIF]`.
- **Validation humaine :** valeur des contradictions, utilité des artefacts,
  temps et frictions sur de vrais projets, acceptation d'une première équipe.
  Un double déterministe ne peut pas produire cette preuve.

### Gates de livraison

| Gate                 | Conditions                                                                                                                                                                    | Ce qu'il permet d'affirmer                                                           |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| CC-G1 Logiciel       | Exigences implémentées, tests locaux/CI/revue, parcours complets et preuves synthétiques ; limites résiduelles listées.                                                       | Candidate logicielle disponible, pas production ni connecteurs live certifiés.       |
| CC-G2 Services réels | Compte IA et destinations autorisés, budget convenu, Auth/SMTP, Notion/Linear/GitHub et traitement complet exécutés sur le candidat.                                          | Chemins effectivement exercés qualifiés pour les comptes/modèles/capacités indiqués. |
| CC-G3 Exploitation   | HTTPS, isolation déployée, quotas/arrêt, sauvegarde/restauration, alertes/support et données vérifiés sur la cible.                                                           | Ouverture contrôlée à une équipe/premiers clients selon limites publiées.            |
| CC-G4 Usage          | Dix boucles sur trois projets pendant au moins sept jours propriétaire, puis deux à trois participants actifs pendant au moins quatorze jours ; incidents et valeur analysés. | Décision V1 éprouvée ou prolongation motivée ; aucun résultat temporel simulé.       |

L'absence de clé, d'accès, de budget ou de cible ne dispense pas de livrer et
vérifier les autres lots. Elle laisse les seules preuves dépendantes ouvertes,
avec motif et action précise. Les semaines pilote ne sont pas des tests que
l'agent peut accélérer. L'ancienne campagne comparative reste disponible et
ses éventuelles conclusions sont distinctes des gates CC-G1 à CC-G4.

## Risques et questions ouvertes

- **Accès et budget :** comptes/destinations autorisés, domaine, région,
  hébergeur, SMTP et enveloppes doivent être identifiés sans demander ni
  consigner leurs secrets dans le chat ou les rapports. Aucune dépense non
  budgétée n'est présumée autorisée par la réalisation logicielle.
- **Qui finance l'IA :** conserver les profils personnels ; une connexion
  entreprise éventuelle doit être explicitement administrée et plafonnée.
  L'abonnement CLI local n'est pas utilisable comme authentification partagée.
- **Sources de code :** le produit peut constater un écart dans des sources
  accessibles, pas certifier le comportement d'une application non exécutée.
- **Fédération :** autorisation avant sélection, génération et affichage ; une
  synthèse ne doit pas contourner les restrictions de ses sources.
- **Synchronisation :** les APIs tierces, limites et capacités effectives sont
  vérifiées dans leurs documentations officielles lors de l'implémentation.
- **Amplitude :** la livraison se fait par lots intégrables. Une interface
  seule, un catalogue d'agents ou un graphe décoratif ne ferment pas le périmètre.
