# Assemblage frontend T17 — suivi de réalisation

> Mis à jour le 2026-09-30. Frontend T17 réalisé et figé pour la qualification
> d'ensemble : 21 scénarios navigateur avec API fictive passent, ainsi qu'une
> première recette navigateur sur API/PostgreSQL réels avec fournisseur fictif.
> Les limites et l'état précis des preuves sont détaillés au §7. Références :
> [contrat API](api-contract.md), [parcours UX](ux-plan.md),
> [revue du design](design-review.md).

`apps/web/tsconfig.app.json` inclut tout `src` : un composant non importé y serait
déjà compilé. Le gel T16 a été levé après sa qualification. Le découpage retenu
s'appuie sur les [cas JSON fictifs](fixtures/frontend-source-cases.json).
Ces fixtures ne sont ni une preuve backend ni une lecture de vrais comptes.

## 1. Assemblage réalisé et coutures de test

Tous les chemins suivants sont relatifs à `apps/web/src`. Les composants et pages
sont raccordés. La colonne de droite conserve les coutures envisagées dans le
plan initial : elle ne prétend pas que chacun de ces fichiers de test a été créé.
Les preuves réellement exécutées sont celles du §7. Les comportements de pages
et de composants passent principalement par la recette navigateur.

| Fichier livré | Responsabilité bornée | Couture de test prévue dans le plan |
| --- | --- | --- |
| `api/tool-source-types.ts` | DTO du contrat sans renommage de discriminants ; unions de commandes et réponses paginées | Vérifiés par compilation et fixtures typées |
| `api/tool-sources.ts` | GET liste/référence/observation/historique/reçu ; POST attach/refresh/detach/rebind ; identité et erreurs conservées | `api/tool-sources.test.ts` : route exacte, action du reçu, clé POST, aucun POST pour une lecture |
| `components/tool-sources/source-labels.ts` | Traductions fermées couverture/confiance/motifs ; code inconnu rendu comme limite, pas caché | `source-labels.test.ts` seulement pour motifs/états inconnus ou ambigus |
| `components/tool-sources/source-observation.tsx` | Lecteur pur d'un `SourceObservationDetail` : texte/date/couverture/fraîcheur, origine exacte et détails repliés | `source-observation.test.tsx` : variantes du corpus, pas d'action réseau |
| `components/tool-sources/source-history.tsx` | Liste locale paginée par curseur ; lien de chaque version vers son UUID | `source-history.test.tsx` : suite curseur, version historique exacte, panne locale |
| `components/tool-sources/source-command.ts` | Validation/stockage d'une seule commande figée par slot ; clé, identité, payload, aucun corps source | `source-command.test.ts` : corruption, bornes, séparation acteur/scope/action, pas d'expiration client |
| `components/tool-sources/use-source-command.ts` | Commande figée, suivi de reçu, reprise explicite et invalidation des vues après réponse retrouvée | `use-source-command.test.tsx` et recette navigateur de réponse perdue |
| `components/tool-sources/source-page-frame.tsx` | Focus initial du titre, sans nouveau déplacement lors des mises à jour de reçu ; texte long contenu dans la largeur | Recette clavier et mobile à titre continu de 455 caractères |
| `components/tool-sources/source-command-status.tsx` | GET reçu, états de reprise et bouton explicite ; pas d'envoi au montage | `source-command-status.test.tsx` : réponse perdue, résultat tardif, expiration et `not_received` |
| `components/tool-sources/attach-source-form.tsx` | Choix outil/connexion, lien, partage non précoché, admission par clic et conservation de commande | `attach-source-form.test.tsx` : rôles, absence de capacité, changement de consentement et double clic |
| `components/tool-sources/source-actions.tsx` | Refresh/detach/rebind sur référence actuelle, révision/head attendus et confirmation locale | `source-actions.test.tsx` : refus révision, owner requis pour rebind, pas de réactivation implicite |
| `components/tool-sources/source-citation.tsx` | Carte compacte du même discriminant exact, couverture et lien local ; aucune lecture distante | Régression dans les surfaces consommatrices, sans test miroir du JSX |
| `pages/tool-sources-page.tsx` | Liste de projet ou scope société résolu ; filtres/cursor/counts et formulaire | `tool-sources-page.test.tsx` : périmètre réel, pagination et erreur distincte du vide |
| `pages/tool-source-page.tsx` | Référence actuelle + observation + historique + actions ; source refetch après reçu | `tool-source-page.test.tsx` : changement/retrait, head indisponible et ancien contenu séparé |
| `pages/source-observation-page.tsx` | Exact reader pour les deux familles, sans fallback vers head ni réemploi des actions de référence | `source-observation-page.test.tsx` : identité/type/404/403, historique et publication sans référence |
| `test-fixtures/tool-sources.ts` | Fixtures synthétiques typées tirées du corpus documentaire ; hashes de fixture explicitement synthétiques | Réemployées par tests composants et navigateur |

Garder orchestration au niveau page/action ; le lecteur de contenu ne reçoit
aucun callback d'import ou de publication. N'introduire un hook partagé de commande
que si les quatre actions ont réellement un état commun ; la séparation stockage /
reçu / action suffit et suit le modèle durable déjà qualifié dans T16.

## 2. Intégration réalisée dans les fichiers existants

Ces raccordements ont été réalisés après le gel T16. Le gel frontend T17
couvre maintenant leur qualification d'ensemble ; les corrections éventuelles
restent guidées par un défaut reproduit.

| Fichier existant | Modification prévue | Limite à préserver |
| --- | --- | --- |
| `App.tsx` | Import lazy des trois pages et cinq routes proposées | Aucune collision avec `projects/:projectId/sources/:kind/:sourceId` |
| `components/app/app-shell.tsx` | Liens Sources société / Sources du projet | Résoudre vrai scope société ; ne pas transmettre une valeur littérale `company` au serveur |
| `api/work-tool-types.ts` | Champs optionnels de compatibilité capacité/délai et entrée `allow_existing_reads?` | Champ absent ≠ true ; pas de secret en GET ; préserver anciens payloads |
| `api/client.ts` ou transport effectivement central | Conserver les champs optionnels d'erreur `retryable` et `retry_after` | Inspecter le transport exact avant modification ; pas de réécriture de la stratégie globale de retry |
| `components/work-tools/connection-form.tsx` + page Connexions | Capacité owner, aucune clé requise pour sa modification | SaveConnection réactive actuellement une connexion désactivée : confirmation explicite existante, pas toggle caché |
| `lib/graph-layout.ts` | Autoriser seulement les nouvelles routes internes exactes validées par root | Aucun chemin distant ou `source_kind` inconnu transformé en route locale |
| Cartes de sources conversation/artefacts/pack/constat | Branches explicites pour les deux nouvelles familles | Même UUID historique, pas remplacement par la dernière observation |

Routes UI retenues et raccordées aux `app_path` SQL :

| Route | Composant / paramètre |
| --- | --- |
| `/sources` | `ToolSourcesPage`, projet société via `companyApi.overview` |
| `/projects/:projectId/sources` | `ToolSourcesPage`, projet demandé et vérifié |
| `/sources/:referenceId` | `ToolSourcePage` |
| `/source-observations/:observationId` | `SourceObservationPage`, `tool_source_observation` fixé par route |
| `/publication-observations/:observationId` | Même page, `publication_observation` fixé par route |

Le type est donné par la route, puis vérifié contre la réponse. Le détail compare
UUID exact et provenance exclusive (`reference_public_id` ou `publication_public_id`).
Une publication n'affiche pas detach/rebind ; son lien de gestion mène au job
existant. Le lecteur d'observation ne convertit pas une réponse invalide en contenu vide.

## 3. Données, état local et priorités de focus

| Lecture | Query key retenue, après frontière d'identité globale | Déclencheur |
| --- | --- | --- |
| Liste | `['tool-sources', projectId, provider, status, cursor]` | Page/filtres/clic page suivante |
| Référence | `['tool-source', referenceId]` | Page et invalidation locale après mutation terminée |
| Observation | `['source-observation', sourceKind, observationId]` | Citation ou observation exacte reçue du détail |
| Historique | `['tool-source-history', referenceId, cursor]` | Ouverture historique puis clic page suivante |
| Reçu | `['tool-source-command', projectId, action, key]` | Commande enregistrée, montage, reconnexion, demande de suivi |

Conserver la frontière `captureRequestContext()` et le reset de QueryClient à
l'identité. Pour les snapshots, désactiver les retries automatiques d'erreurs de
droits ; une réponse historique 403 ne doit pas maintenir l'ancien corps à l'écran.
Les caches de données restent en mémoire. Les clés query ne contiennent pas le
lien collé, le corps ou un credential. Un changement de filtre remet le curseur à
zéro ; page suivante conserve les filtres, sans deviner la taille totale.

L'état serveur demeure la source du statut, de l'éligibilité et du délai. Seuls
la saisie, la case de partage, les filtres et la commande durable sont locaux.
Une observation est immuable mais sa `freshness` ne l'est pas : recharger la lecture
locale à l'ouverture du détail pour calculer les droits/états courants. Ne pas
persister `eligible=true` comme autorisation de nouvelle action.

Le titre de page ou la source expressément demandée reçoit le focus après
navigation. Un reçu restauré tardivement ne le vole pas. Après une action locale
explicite, le résultat de cette action peut être annoncé/focalisé ; un contrôle
en arrière-plan ne redirige pas le lecteur. Cette règle reprend la régression de
focus corrigée pendant la recette T16.

## 4. Commande durable : union et transitions

Enveloppe locale retenue, sans changer les corps de requête du contrat :

```ts
type SavedSourceCommand = {
  schema: 1
  key: string
  createdAt: number // information locale, jamais autorité d'expiration
  projectId: string
} & (
  | { action: 'attach'; referenceId: null; input: AttachToolSource }
  | { action: 'refresh' | 'detach'; referenceId: string; input: ExpectedSource }
  | { action: 'rebind'; referenceId: string; input: RebindToolSource }
)
```

Slot `ai-center.tool-source:{actor}:{workspace}:{project}:{action}:{reference|new}`.
Un formulaire peut donc retrouver son attach en attente sans connaître l'UUID
de référence futur. Une autre action ne reprend pas la clé du premier slot. Un
refresh/detach/rebind stocke l'identité de la route ; le backend en déduit le scope.
Une divergence scope stocké / reçu / référence est une erreur explicite.

Valider au chargement : version d'enveloppe, action fermée, UUID canoniques,
entiers de révision positifs, taille de l'enveloppe, taille UTF-8 du lien ≤2 Kio,
confirmation strictement true, entrée officielle acceptée sans secret ni URL
arbitraire. La saisie d'attach doit rester identique lors d'une reprise ; on ne la
remplace pas par l'UUID résolu. JSON corrompu ne déclenche aucune mutation.

1. Valider la saisie/consentement ; figer le payload ; créer la clé ; persister.
2. Si le stockage échoue, afficher l'erreur avant tout POST : ne pas perdre la
   possibilité de retrouver le résultat d'une lecture longue.
3. Envoyer uniquement sur clic ; le double clic réutilise la même commande ou
   reste bloqué. Le changement de saisie n'altère pas une demande déjà envoyée.
4. En cas de résultat incertain, garder le slot et consulter le GET reçu avec
   `operation={action}`. Reprise explicite seulement avec `can_retry`, même
   clé/payload ; si le serveur porte un délai, l'afficher sans POST programmé.
5. `completed` ouvre son observation exacte et recharge séparément l'état actuel.
   `failed`/`expired` permettent une préparation nouvelle après lecture locale de
   l'état ; ne jamais transformer l'ancienne clé expirée en nouvelle demande.
6. Tant que le résultat est inconnu ou la commande en traitement, ne pas proposer
   un second attach équivalent comme moyen d'ignorer la demande en cours.

Un polling borné du **GET local** en processing est possible ; arrêt sur erreur,
état terminal ou démontage. Pas de lecture fournisseur au montage/reconnexion,
pas de timer de POST, pas de reprise automatique par React Query. L'expiration
est uniquement celle du reçu serveur, y compris pour une commande locale ancienne.

## 5. Matrice des premiers tests utiles

| Test | Déclencheur / preuve attendue |
| --- | --- |
| Lecture fidèle | Date distante null, confiance observée, texte exact et absence de libellé « règle validée » |
| Partiel et omissions hors périmètre | complete garde commentaires non lus ; partial montre troncature ; code d'omission inconnu reste visible |
| Indisponible | Corps courant vide et motif « absent ou inaccessible », jamais « supprimé » ; lien vers ancien snapshot distinct |
| Historique et éligibilité | Version1 historique reste version1 après head2 ; disabled/detached/403 n'élèvent pas l'ancienne lecture en contexte courant |
| Publication | Provenance publication sans référence, lien job et corps exact ; aucun bouton de nouveau rattachement ou nouveau POST de publication |
| Pagination | Deux pages/historique avec curseurs et filtres conservés ; aucun compteur calculé depuis les lignes visibles |
| Reçu incertain | POST finit côté API mais réponse perdue ; remount = GET, même observation, aucun nouvel appel fournisseur |
| Reprise autorisée | not_received ancien et interrupted/retryable utilisent même clé/payload, seulement après clic |
| Identité | Acteur ou société change pendant GET/POST : ancien résultat/corps non affiché, aucune redirection tardive |
| Quotas | Tous les codes d'admission gardent délai serveur et message français, absence de boucle de retry ; refus avant réseau distingué du 429 distant |
| Focus | Lien exact ouvert puis reçu différé : le reçu ne vole pas le focus ; clavier peut rejoindre les actions |

Les validations du lecteur JSON et du stockage ont une couture unitaire utile ;
les comportements d'accès/réseau passent par composants avec API simulée, puis par
le scénario gardé root. Ne pas écrire un test distinct de chaque libellé statique.

La future recette navigateur doit compter les POST et les appels au faux outil,
pas seulement vérifier un texte de succès. PM → artefact → T16 → handoff utilise
la même observation, et l'actualisation inchangée préserve sa version. Les sources
de fixture sont marquées **[FICTIF]** ; la preuve réelle fournisseur reste séparée.

## 6. Ordre de réalisation suivi

1. Routes SQL `app_path`, discriminants et états de fraîcheur partagés avec le
   backend ; DTO et lecteur exact raccordés.
2. Liste, filtres, historique, détail de référence et observation exacte livrés.
3. Commande durable et reçu GET intégrés ; ajout Linear et Notion sur connexion
   autorisée avec partage explicite, capacité owner dans Connexions.
4. Actions révisionnées et citations raccordées aux conversations, livrables,
   packs et constats existants ; distinction des observations de publication.
5. Recette fictive et corrections de course/focus terminées ; frontend figé pour
   la recette API/base réelle conduite par root. L'édition enrichie des tickets
   reste le complément distinct du plan UX, pas une réalisation T17 implicite.

## 7. Preuves frontend au 30 septembre 2026

### Corrections issues de la recette

- Une réponse perdue pouvait laisser un reçu `not_received` obtenu trop tôt,
  pendant le POST, alors que l'ajout avait finalement réussi. Le suivi GET démarre
  désormais après la fin du POST ; la récupération `completed` invalide les vues
  locales comme une réponse POST reçue. Aucun nouvel appel fournisseur n'est
  lancé par cette récupération ou par un rechargement.
- Une erreur de transport devenue obsolète n'est plus présentée lorsque le reçu
  confirme `completed`. Le scénario navigateur reproduit la coupure puis vérifie
  l'absence d'erreur contradictoire et la mise à jour de la liste.
- Les trois pages de sources placent une fois le focus sur leur titre lors de
  leur ouverture. Les reçus tardifs n'entraînent ni navigation ni reprise du focus.
  Les titres continus longs restent dans la largeur mobile. Une action de lecture
  dont les conditions de connexion/capacité/stockage ne sont plus réunies est
  désactivée plutôt que présentée comme un bouton sans effet.

### Vérifications exécutées

| Preuve | Résultat et portée exacte |
| --- | --- |
| `npm run test -w @ai-center/web` | 192 tests passent dans 50 fichiers après correction de la course de reçu, avant l'ajout final du cadre de focus. Celui-ci est ensuite couvert par la recette navigateur ; ne pas présenter les 192 tests comme une exécution après ce dernier ajout. |
| `npm run lint -w @ai-center/web` et `npm run build -w @ai-center/web` | Passent après les dernières modifications produit de focus et de largeur. |
| `tool-sources.spec.ts`, matrice Playwright existante | **21/21 passent en 50,2 s** : sept scénarios sur Chromium desktop (1440×900), Chromium compact (1024×768), Firefox desktop, avec contrôles à 390×844. API entièrement fictive pour cette matrice. |
| Accessibilité et mise en page | Aucun défaut axe détecté dans les surfaces auditées (`wcag2a`, `wcag2aa`, `wcag21aa`), aucun débordement horizontal, contenu et URL attendus, absence de surcouche framework, captures inspectées. Le focus du titre puis la navigation par Tab sont vérifiés. Ce n'est pas une certification exhaustive d'accessibilité. |
| Console et demandes réseau | Aucune erreur inattendue dans les scénarios concernés. Le seul échec réseau toléré est l'abandon volontaire de la réponse du POST sur son URL exacte. Le nombre de POST reste inchangé après récupération et rechargement. |
| Scénario `source-real-e2e/workflow.spec.ts` conduit par root | **1/1 passe en 11,7 s** sur la vraie API, les routes de production, l'autorisation locale et PostgreSQL isolé. Seul le fournisseur distant est remplacé par des réponses HTTP loopback fictives. Le harness Rust vérifie **1 requête Linear + 6 requêtes Notion**, **2 références et 3 observations**. Ce résultat ne prouve pas l'accès à de vrais comptes Notion/Linear. |

Les sept scénarios fictifs couvrent : ajout Linear ; ajout Notion ; récupération
après réponse perdue ; passage de la lecture historique à la lecture actuelle
partielle puis révocation avec disparition du texte ; indisponibilité sans reprise
d'un ancien texte et rôle lecteur ; origine d'une observation de publication ;
focus clavier et titre continu long sur mobile. Le consentement est initialement
absent et remis à zéro après changement de connexion. Le rechargement consulte
le reçu sans nouveau POST. Cette matrice ne remplace pas les tests backend de
concurrence, quotas, réactivation/changement de connexion et frontières d'autorité.

La recette réelle ajoute Linear avec perte de réponse, retrouve son reçu, ajoute
Notion, avance le contenu du faux fournisseur puis déclenche une vérification
explicite. La lecture V1 demeure consultable exactement ; V2 est distincte et
partielle. Les captures initiales sont `/tmp/t17-real-linear-desktop.png` et
`/tmp/t17-real-notion-mobile.png`.

### Cadrage mobile de la recette réelle

La première capture mobile `fullPage` conservait une position de défilement après
la navigation et le changement de format : le header `sticky` apparaissait plus
bas dans l'image et recouvrait le fil contextuel. Le titre focalisé restait sous
le header. L'examen du CSS et un diagnostic navigateur séparé n'ont pas établi de
défaut de focus caché : à l'origine de page en 390×844, le header se termine à 65px,
le contexte commence à 89px et le titre à 157px.

Seul `visualCheck` du scénario réel a été renforcé : retour explicite à
`scrollTo(0,0)` avant capture, puis vérification que titre et contexte sont dans le
viewport et sous le header. **La recette réelle réussie précède cet ajustement du
cadrage ; son réexamen et les captures renouvelées restent à exécuter par root.**
Aucun nouveau changement produit n'a été introduit pour ce cadrage.

### Localisation des traces et limites

Les journaux privés locaux sont `/tmp/t17-browser-final-matrix.log`,
`/tmp/t17-frontend-tests.log`, `/tmp/t17-frontend-final-lint.log`,
`/tmp/t17-frontend-final-build.log`. La preuve initiale API/base réelle apparaît
dans `.run/tool-sources-full-database-2026-09-30.log` (phase navigateur réussie ;
ce fichier contient d'autres phases et ne constitue pas à lui seul une assertion
que toute la recette d'ensemble est verte). Les captures des scénarios fictifs
sont dans `/tmp/t17-*.png`. Ces fichiers locaux ne sont pas versionnés.

Environnement frontend fictif : `http://127.0.0.1:5197` ; Playwright du dépôt,
raison de ce choix : **Browser plugin not available**. Aucune nouvelle dépendance,
aucune stack concurrente et aucun appel fournisseur réel ou facturé n'ont été
introduits par cette recette frontend. La publication, l'hébergement et la preuve
sur de vrais comptes restent distincts de ces succès locaux.
