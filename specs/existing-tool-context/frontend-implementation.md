# Assemblage frontend T17 — préparation avant levée du gel

> 2026-09-24. Plan prêt à exécuter après signal du coordinateur. Aucun nouveau
> fichier produit n'est branché ni compilé par cette préparation. Références :
> [contrat API](api-contract.md), [parcours UX](ux-plan.md),
> [revue du design](design-review.md).

`apps/web/tsconfig.app.json` inclut tout `src` : un composant non importé y serait
déjà compilé. Le gel T16 est donc respecté en conservant ici le découpage et les
[cas JSON fictifs](fixtures/frontend-source-cases.json). Ces cas ne sont ni une
preuve backend ni une lecture de vrais comptes.

## 1. Fichiers à créer, dans l'ordre utile

Tous les chemins suivants sont relatifs à `apps/web/src`. La tranche Linear doit
traverser toute la filiation avant d'ajouter le reste ; ne pas livrer une bibliothèque
générique de connecteurs ou un deuxième store global.

| Fichier futur | Responsabilité bornée | Test proche |
| --- | --- | --- |
| `api/tool-source-types.ts` | DTO du contrat sans renommage de discriminants ; unions de commandes et réponses paginées | Vérifiés par compilation et fixtures typées |
| `api/tool-sources.ts` | GET liste/référence/observation/historique/reçu ; POST attach/refresh/detach/rebind ; identité et erreurs conservées | `api/tool-sources.test.ts` : route exacte, action du reçu, clé POST, aucun POST pour une lecture |
| `components/tool-sources/source-labels.ts` | Traductions fermées couverture/confiance/motifs ; code inconnu rendu comme limite, pas caché | `source-labels.test.ts` seulement pour motifs/états inconnus ou ambigus |
| `components/tool-sources/source-observation.tsx` | Lecteur pur d'un `SourceObservationDetail` : texte/date/couverture/fraîcheur, origine exacte et détails repliés | `source-observation.test.tsx` : variantes du corpus, pas d'action réseau |
| `components/tool-sources/source-history.tsx` | Liste locale paginée par curseur ; lien de chaque version vers son UUID | `source-history.test.tsx` : suite curseur, version historique exacte, panne locale |
| `components/tool-sources/source-command.ts` | Validation/stockage d'une seule commande figée par slot ; clé, identité, payload, aucun corps source | `source-command.test.ts` : corruption, bornes, séparation acteur/scope/action, pas d'expiration client |
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

## 2. Intégration différée dans les fichiers existants

Ces fichiers **ne sont pas modifiés pendant le gel**. Les points ci-dessous
serviront de checklist au feu vert.

| Fichier existant | Modification prévue | Limite à préserver |
| --- | --- | --- |
| `App.tsx` | Import lazy des trois pages et cinq routes proposées | Aucune collision avec `projects/:projectId/sources/:kind/:sourceId` |
| `components/app/app-shell.tsx` | Liens Sources société / Sources du projet | Résoudre vrai scope société ; ne pas transmettre une valeur littérale `company` au serveur |
| `api/work-tool-types.ts` | Champs optionnels de compatibilité capacité/délai et entrée `allow_existing_reads?` | Champ absent ≠ true ; pas de secret en GET ; préserver anciens payloads |
| `api/client.ts` ou transport effectivement central | Conserver les champs optionnels d'erreur `retryable` et `retry_after` | Inspecter le transport exact avant modification ; pas de réécriture de la stratégie globale de retry |
| `components/work-tools/connection-form.tsx` + page Connexions | Capacité owner, aucune clé requise pour sa modification | SaveConnection réactive actuellement une connexion désactivée : confirmation explicite existante, pas toggle caché |
| `lib/graph-layout.ts` | Autoriser seulement les nouvelles routes internes exactes validées par root | Aucun chemin distant ou `source_kind` inconnu transformé en route locale |
| Cartes de sources conversation/artefacts/pack/constat | Branches explicites pour les deux nouvelles familles | Même UUID historique, pas remplacement par la dernière observation |

Routes UI à confirmer par root pour les `app_path` SQL :

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

| Lecture | Query key proposée, après frontière d'identité globale | Déclencheur |
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

Enveloppe locale proposée, sans changer les corps de requête du contrat :

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

## 6. Ordre d'exécution au feu vert

1. Root confirme routes SQL `app_path`, discriminants et coutures de fraîcheur ;
   backend expose les types/routes. Les interfaces du contrat suffisent pour
   développer le lecteur pur et les fixtures en parallèle.
2. Brancher DTO + lecteur exact + historique et leurs tests, puis liste et liens.
3. Brancher stockage/reçu + attach Linear et test de perte de réponse ; activer
   capacité owner sans ressaisie de secret, en préservant anciennes commandes.
4. Ajouter actions révisionnées et citations sur les surfaces déjà existantes,
   puis Notion et observations de publication selon la tranche définie.
5. Signaler le gel frontend à root avant sa recette DB/navigateur ; traiter les
   défauts prouvés sans lancer de seconde stack. Les ajouts UX d'édition de tickets
   restent le complément distinct décrit dans le plan UX, pas un prérequis caché.
