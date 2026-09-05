# AI Center — Périmètre du MVP

> Statut : scope consolidé — version 0.3
>
> Date : 24 août 2026
>
> Périmètre : première preuve utilisable du plan de contrôle contextuel, de l’intention jusqu’à la preuve.

## Résumé exécutif

Le MVP d’AI Center ne doit être ni un clone de ChatGPT, ni un gestionnaire de projets enrichi de fichiers Markdown, ni un IDE, ni un lanceur d’agents de code. Il doit prouver qu’un plan de contrôle contextuel peut relier les décisions d’une entreprise, compiler le bon contexte pour chaque agent ou outil externe, puis vérifier que les résultats observés restent cohérents avec l’intention initiale.

AI Center se greffe sur les systèmes déjà adoptés — GitHub, Linear, Notion, Figma, bases de données, Codex, Claude Code, Cursor ou CLI — au lieu de leur demander de migrer. Ces systèmes conservent leur rôle de source ou d’outil de production ; AI Center possède le contexte partagé, les handoffs, les contrats, la provenance et les contrôles transverses.

La première version doit permettre à un utilisateur de :

1. créer un projet à partir du template système **Software Product Delivery**, comportant deux nœuds racines, **Produit** et **Tech** ;
2. cadrer une évolution avec l’agent Produit et transformer la conversation en décisions, règles, exigences et questions ouvertes structurées ;
3. satisfaire un premier `Gate` produit et produire un `Deliverable` de type `FeatureBrief` ;
4. compiler automatiquement un `ContextPack` adapté à l’agent Tech ;
5. effectuer un passage de relais explicite, sans demander à l’utilisateur de reformuler le contexte ;
6. produire côté Tech un `TechnicalDeliveryPlan` relié aux exigences et accompagné d’une matrice de couverture ;
7. détecter proactivement une contradiction entre une règle Produit et une décision Tech ;
8. comprendre, accepter, rejeter ou résoudre cette contradiction ;
9. rattacher chaque livrable et chaque preuve, y compris une référence externe, aux connaissances qui les ont provoqués ;
10. retrouver le même état durable du projet depuis une interface web desktop et le client Linux.

Le graphe, le `ContextPack`, les contrats de livrables, les preuves et la notion de référence externe sont présents dès le MVP. Sont différés : la personnalisation des templates, l’agent global inter-projets, la visualisation avancée du graphe, les applications mobiles et toute infrastructure interne de production de code.

## Thèse produit testée

> Un utilisateur obtient davantage de valeur de plusieurs agents lorsque le système conserve l’intention du projet, compile pour chacun un contexte fiable, formalise leurs passages de relais et vérifie leurs résultats, plutôt que lorsqu’il lui laisse reconstruire lui-même ce workflow dans des chats, des prompts et des outils séparés.

Le MVP doit tester conjointement quatre hypothèses :

- **Structuration** — les décisions et exigences extraites d’une conversation sont plus réutilisables qu’un transcript ou un document isolé ;
- **Handoff** — le `ContextPack` permet de passer de Produit à Tech sans reformulation et sans chargement massif du projet ;
- **Contrôle** — un contrat de livrable et ses preuves permettent de savoir ce qui est réellement couvert, manquant ou incertain ;
- **Proactivité** — une contradiction détectée et expliquée sans demande explicite constitue un moment de valeur suffisamment fort pour différencier AI Center.

La mobilité et l’exécution interne ne sont pas des hypothèses fondatrices. La consolidation est testée sur le web desktop et le client Linux, sans Android. Un premier adapter externe étroit sera ajouté après validation de la boucle contextuelle ; il démontrera que le contexte peut sortir vers un outil existant et que ses preuves peuvent revenir sans duplication de sa fonction.

## Promesse du MVP

> Je peux partir d’une intention produit, la transmettre à un agent technique sans reconstruire le contexte, obtenir un livrable vérifiable et être averti lorsque le résultat ou une nouvelle décision n’est plus cohérent avec ce qui avait été décidé.

## Les trois moments « waouh »

### 1. Moment contextuel

> « Je viens de prendre une décision côté Produit. AI Center a retrouvé seul une règle incompatible côté Tech, m’a expliqué le conflit et a relié les deux éléments. »

### 2. Moment de passage de relais

> « Je passe de Produit à Tech et l’agent technique comprend immédiatement l’objectif, les décisions, les contraintes et les critères d’acceptation, sans que je lui réexplique quoi que ce soit. »

### 3. Moment de contrôle

> « Le livrable paraît terminé, mais AI Center me montre qu’une exigence ne possède encore aucune preuve et qu’une décision récente invalide une partie du plan. »

Ces trois moments forment la preuve différenciante : cohérence, continuité et contrôle. La production de code, de tickets, de designs ou de documents reste effectuée par les outils spécialisés ; AI Center ferme la boucle en leur transmettant du contexte et en réintégrant leurs résultats.

## Scénario de référence

Le scénario suivant sert de test d’acceptation principal :

1. L’utilisateur crée le projet `Credits v2` avec le template `Software Product Delivery`.
2. Dans le nœud Produit, il dicte que « les crédits achetés n’expirent jamais ».
3. L’agent Produit challenge la règle, puis propose une entrée de type `business_rule`.
4. L’utilisateur la confirme ; elle est commitée dans le graphe du projet.
5. Dans le nœud Tech, une autre session établit qu’un job supprimera les crédits inutilisés après 90 jours.
6. L’agent Tech propose et commite une entrée de type `technical_rule`.
7. L’agent Produit produit un `FeatureBrief`. Le `ProductReadyGate` vérifie que l’objectif, la règle métier, les critères d’acceptation et les questions bloquantes sont traités.
8. AI Center compile un `ContextPack` et propose le handoff vers le nœud Tech.
9. L’agent Tech reçoit ce contexte avec sa provenance et produit un `TechnicalDeliveryPlan`.
10. Chaque section du plan est reliée aux exigences couvertes ; une matrice montre les exigences couvertes, partielles ou sans preuve.
11. Le commit de la règle technique déclenche le steward du projet.
12. Celui-ci sélectionne les entrées potentiellement liées, détecte le conflit sémantique et crée une proposition d’arête `contradicts`.
13. L’interface affiche un warning en reliant clairement les deux règles, leur origine et l’explication du conflit.
14. L’utilisateur modifie la règle technique, accepte temporairement la contradiction avec justification, ou rejette la détection comme faux positif.
15. Le `ContextPack`, le plan et la matrice de couverture sont invalidés ou revalidés en fonction de la résolution.
16. L’utilisateur retrouve le projet, les sessions, le handoff, la contradiction et les preuves après avoir fermé puis rouvert l’application.

## Tranche verticale fonctionnelle

```mermaid
flowchart TD
    A["Intention Produit"] --> B["Connaissances structurées"]
    B --> C["Gate et Feature Brief"]
    C --> D["ContextPack Tech"]
    D --> E["Handoff"]
    E --> F["Technical Delivery Plan"]
    F --> X["Outil externe spécialisé"]
    X --> G["Références, preuves et couverture"]
    G --> H["Contrôle de cohérence"]
    H --> B
```

## Limites structurelles du MVP

| Dimension | Choix MVP | Préparation du futur |
| --- | --- | --- |
| Utilisateurs | Un seul utilisateur | Identifiants d’auteur et d’acteur présents dans les événements |
| Surfaces | Web desktop et Tauri Linux | Autres clients possibles sans changer le domaine ; mobile différé |
| Workspaces | Un workspace actif | Toutes les entités portent un `workspaceId` |
| Projets | Plusieurs projets créables, un projet actif par session | Identifiants globaux permettant des relations inter-projets futures |
| Template | Un template système `Produit + Tech` | Template, définitions de nœuds et profils d’agents modélisés comme données |
| Graphe | Un graphe scopé par projet | Endpoints d’arêtes compatibles avec des cibles externes au projet |
| Agents spécialisés | Produit et Tech | Association explicite `ContextNode → AgentProfile` |
| Agent global | Un steward par projet | Couche Center inter-projets différée |
| Proactivité | Contradictions et couverture manquante sur événements | Moteur extensible à d’autres insights |
| Handoff | Produit vers Tech | Handoffs arbitraires définis par les templates |
| Interopérabilité | Références externes manuelles et contrat d’adapter | Premier connecteur réel après validation du core |

## Modèle conceptuel minimal

### `Workspace`

Frontière durable de connaissance. Le MVP n’expose pas encore la gestion avancée de plusieurs workspaces, mais toutes les données appartiennent explicitement à un workspace.

### `Project`

Porte un graphe de connaissances, un template appliqué, un résumé condensé, des sessions et des exécutions.

### `ProjectTemplate`

Décrit les nœuds racines, leur hiérarchie initiale, les profils d’agents associés et les types de livrables attendus.

Le template MVP est fourni par le système et non modifiable. Il ne doit toutefois pas être codé uniquement dans l’interface : sa représentation doit permettre plus tard la création, la duplication et la personnalisation de templates.

### `ContextNode`

Représente un scope du graphe, par exemple Produit ou Tech. Un nœud peut avoir un parent et des enfants. Il porte :

- un titre et une description ;
- un résumé condensé ;
- un profil d’agent ;
- des règles de contexte ;
- des types d’entrées et de livrables privilégiés.

### `AgentProfile`

Décrit le rôle, les instructions, les outils accessibles, la politique de retrieval et les formats de sortie de l’agent lié au nœud.

Dans le MVP, les profils Produit et Tech sont fournis par le système. Leur personnalisation par l’utilisateur est hors périmètre, mais l’association ne doit pas être enfouie dans un `if product / if tech` irréversible.

### `KnowledgeEntry`

Atome de connaissance lisible, typé, versionné et rattaché à un nœud.

Types indispensables :

- `decision` ;
- `business_rule` ;
- `technical_rule` ;
- `requirement` ;
- `acceptance_criterion` ;
- `constraint` ;
- `open_question`.

Champs minimaux : identifiant, type, titre, énoncé, justification, statut, auteur, origine, nœud, version et dates.

### `Edge`

Relation typée et orientée entre deux objets du graphe.

Types indispensables :

- `references` ;
- `depends_on` ;
- `informs` ;
- `supersedes` ;
- `contradicts` ;
- `derived_from` ;
- `satisfies` ;
- `evidenced_by` ;
- `implemented_by`.

La relation `tracked_by` et un objet générique `ExternalReference` font partie du modèle utile. Leur saisie manuelle suffit au MVP ; la synchronisation automatique et le catalogue de connecteurs sont différés.

### `Session`

Porte la continuité conversationnelle, le scope courant, les messages, les propositions de mutation du graphe et les exécutions déclenchées.

### `ContextPack`

Projection versionnée et immutable du graphe, compilée pour une tâche, un agent et un instant donnés. Il contient l’objectif, les décisions applicables, les exigences, les contraintes, les dépendances, les questions ouvertes, les sources, les permissions et le contrat de résultat.

Un `ContextPack` n’est jamais la source de vérité : il référence les versions exactes des connaissances qui l’ont composé. Toute modification pertinente peut le rendre `stale` et déclencher une recompilation.

### `DeliverableContract`

Décrit ce qu’un agent doit produire et comment le résultat sera évalué. Il définit au minimum :

- les sections ou champs obligatoires ;
- les critères de complétude ;
- les types de preuves acceptés ;
- les relations attendues avec les exigences ;
- les conditions nécessitant une validation humaine.

Le MVP possède deux contrats système : `FeatureBrief` et `TechnicalDeliveryPlan`.

### `Deliverable`

Instance versionnée d’un contrat de livrable. Son contenu peut être affiché comme un document, mais reste composé de blocs structurés et reliés au graphe.

### `Gate`

Condition de passage entre deux étapes du modèle opératoire. Un gate évalue des règles déterministes et, lorsque nécessaire, une appréciation sémantique expliquée. Il peut être `pending`, `passed`, `passed_with_warning` ou `blocked`.

Le `ProductReadyGate` du MVP vérifie notamment la présence d’un objectif, de critères d’acceptation, des décisions structurantes et l’absence de question bloquante non assumée.

### `Evidence`

Élément attestant qu’une exigence ou une partie d’un livrable est couverte : citation de livrable, décision validée, résultat de test, fichier, commit, ticket, maquette, URL ou validation humaine. Dans la première tranche, les preuves peuvent être saisies ou référencées manuellement ; le premier connecteur automatise ensuite une partie de leur collecte.

### `Task`

Travail explicite dérivé d’une décision, exigence ou résolution. Une tâche référence le `ContextPack` utilisé et le `DeliverableContract` attendu. Elle peut rester un objet AI Center ou pointer vers un ticket Linear, Jira ou équivalent, sans copie obligatoire.

### `Execution`

Tentative confiée à un agent ou un outil spécialisé externe. Elle expose l’exécutant, le `ContextPack`, le contrat attendu, son état et les références de résultat. Une `Execution` est une enveloppe de pilotage et d’audit ; elle n’implique pas qu’AI Center exécute lui-même du code ou héberge le fournisseur.

### `Artifact`

Résultat produit ou référencé : livrable, rapport, document, fichier, test, commit, ticket, maquette, URL de preview ou pull request. AI Center conserve sa provenance et ses liens avec le graphe ; l’objet canonique peut continuer à vivre dans GitHub, Linear, Notion, Figma ou un autre système.

### `ExternalReference`

Pointeur durable vers un objet appartenant à un système externe. Il contient au minimum le fournisseur, le type d’objet, l’identifiant externe, l’URL éventuelle, le dernier état observé, la date de synchronisation et la provenance. Il évite de dupliquer l’intégralité d’un ticket, d’un document, d’un design ou d’un artefact de code dans AI Center.

### `Insight`

Observation proactive produite par le steward : contradiction, risque, information manquante ou absence de preuve. Dans le MVP, les sous-types `contradiction` et `coverage_gap` sont implémentés.

## Invariants du domaine

1. Chaque projet possède son propre graphe logique.
2. Chaque nœud appartient à un seul projet et peut former une hiérarchie.
3. Chaque nœud racine possède un profil d’agent explicite.
4. Une entrée de connaissance appartient à un nœud et possède un historique de versions.
5. Une arête référence des identifiants stables ; elle ne copie pas les objets reliés.
6. Une conversation ne devient pas automatiquement une vérité du graphe : l’agent propose des mutations structurées qui sont commitables et auditables.
7. L’agent d’un nœud reçoit le contexte détaillé de son scope, un résumé du projet et uniquement les voisins récupérés comme pertinents.
8. L’agent global raisonne sur la couche condensée et les entrées commitables, pas sur chaque token de toutes les conversations.
9. Toute action proactive possède une origine, une explication et un niveau de confiance.
10. Tout `ContextPack` référence les versions exactes des connaissances qui l’ont composé.
11. Tout livrable appartient à un contrat et expose son état de couverture.
12. Toute preuve est reliée à l’exigence ou à la décision qu’elle soutient.
13. Une connaissance modifiée peut invalider les `ContextPack`, gates, livrables et preuves qui en dépendent.

## Gestion des mutations du graphe

La conversation doit rester fluide. L’utilisateur ne remplit pas manuellement un formulaire après chaque décision.

Le flux retenu est :

1. l’agent identifie une information atomique ;
2. il produit une proposition structurée ;
3. les notes et questions faibles peuvent être enregistrées avec annulation possible ;
4. les décisions, règles et contraintes nécessitent une confirmation explicite ou une validation de lot en fin de séquence ;
5. le commit crée une nouvelle version et émet un événement ;
6. cet événement déclenche les vérifications proactives pertinentes.

L’interface doit rendre visibles les changements de connaissance sans transformer chaque échange en formulaire administratif.

## Agent spécialisé de nœud

L’agent Produit et l’agent Tech utilisent le même moteur conversationnel abstrait, mais diffèrent par :

- leurs instructions ;
- leurs outils ;
- les types d’entrées qu’ils privilégient ;
- leurs formats de livrables ;
- leur politique de retrieval ;
- leur scope par défaut.

Un agent spécialisé peut demander explicitement du contexte à un autre nœud. Il ne parcourt pas librement tout le graphe à chaque requête.

## Agent global de projet

L’agent global de projet n’est pas l’agent Center ultime. Il joue le rôle de steward du graphe courant.

Ses responsabilités MVP sont :

- maintenir ou proposer le résumé condensé du projet ;
- rechercher les entrées susceptibles d’être affectées par un commit ;
- vérifier la cohérence sémantique entre ces entrées ;
- produire une contradiction expliquée avec ses sources ;
- identifier les exigences sans preuve et les sections de livrable non couvertes ;
- marquer comme obsolètes les projections dépendant d’une connaissance modifiée ;
- revalider les contradictions ouvertes ou acceptées lorsqu’une entrée liée change.

Il est déclenché par événements, notamment :

- `KnowledgeEntryCommitted` ;
- `KnowledgeEntrySuperseded` ;
- `ContextPackCompiled` ;
- `DeliverableCommitted` ;
- `ContradictionAccepted` ;
- `ContradictionResolved`.

Il n’observe pas en permanence chaque mot de chaque session.

## Détection des contradictions

### Pipeline MVP

1. Une entrée est commitée ou modifiée.
2. Un premier filtre sélectionne des candidats par nœud, métadonnées, mots-clés, embeddings et relations existantes.
3. Un détecteur sémantique compare la nouvelle entrée aux candidats.
4. Il produit une explication, une confiance et une sévérité.
5. Une proposition d’arête `contradicts` et un `Insight` sont créés.
6. L’utilisateur traite le signal depuis l’interface.

### Confiance et sévérité

La confiance du modèle et l’impact métier sont deux dimensions distinctes.

Niveaux de sévérité MVP :

- `notice` — incohérence potentielle non bloquante ;
- `warning` — décision à examiner avant de poursuivre ;
- `blocking` — contradiction forte susceptible d’invalider le travail en cours.

Les signaux peu confiants restent dans une boîte d’insights. Seuls les warnings suffisamment confiants interrompent ou notifient l’utilisateur.

### Cycle de vie

- `candidate` — détectée par le système, pas encore qualifiée ;
- `open` — contradiction reconnue et non résolue ;
- `accepted` — incohérence assumée avec justification, auteur et date ;
- `resolved` — une entrée a été corrigée ou remplacée ;
- `dismissed` — faux positif ou relation non pertinente, conservé comme feedback.

Une contradiction acceptée reste visible. Si une entrée liée change, elle repasse en vérification et peut être rouverte.

## Expérience web desktop MVP

### Écrans indispensables

1. **Center** — projets actifs, décisions attendues, handoffs, livrables et insights prioritaires.
2. **Création de projet** — création depuis le template `Software Product Delivery`.
3. **Projet** — vue Produit / Tech, santé du projet, activité et prochaine action recommandée.
4. **Session de nœud** — conversation contextuelle, propositions de connaissances et validation fluide.
5. **Feature Brief** — livrable Produit structuré, sources, état du gate et éléments manquants.
6. **Handoff** — aperçu du `ContextPack`, provenance, limites et passage vers l’agent Tech.
7. **Technical Delivery Plan** — livrable Tech, exigences couvertes et preuves documentaires.
8. **Decision Inbox** — contradictions et trous de couverture priorisés.
9. **Détail d’un insight** — sources reliées, explication, impact et actions de résolution.
10. **Historique** — versions des connaissances, ContextPacks, livrables et décisions humaines.

### Saisie et dictée

Le texte est la saisie principale du premier incrément. La dictée du navigateur peut être ajoutée dès qu’elle n’interrompt pas la construction de la boucle centrale, mais elle n’est pas un critère de sortie du core MVP.

Ne font pas partie du MVP :

- la conversation vocale bidirectionnelle ;
- l’écoute permanente ;
- le déclenchement par mot-clé ;
- le widget système omniprésent.

### Représentation du graphe

Le MVP ne nécessite pas un canvas libre de type Neo4j ou FigJam.

Le graphe est rendu tangible par :

- la navigation Produit / Tech ;
- les cartes d’entrées structurées ;
- les relations affichées dans leurs détails ;
- une contradiction représentée comme un lien explicite entre deux entrées ;
- un compteur d’insights et un état de cohérence par nœud.

Une vue globale en lecture seule pourra être ajoutée après validation de la boucle principale.

## Intégration des outils et adapters

L’abstraction d’intégration existe dès le core MVP, mais elle représente une frontière de contexte, pas un moteur de production. AI Center prépare un `ContextPack`, le transmet à un système choisi, observe son état et rattache ses résultats au graphe. Le système externe conserve sa propre interface, ses données canoniques et son modèle d’autorisation.

### Core MVP

- objet `ExternalReference` indépendant du fournisseur ;
- rattachement manuel d’un ticket, document, design, dépôt, commit, test ou URL ;
- contrat `ToolAdapter` séparant export du contexte, lecture d’état et import de preuves ;
- `ContextPack` versionné et immutable comme entrée de handoff ;
- `DeliverableContract` comme description du résultat attendu ;
- simulateur actuel conservé uniquement comme harnais de contrat, sans en faire une fonction produit ;
- réintégration des références, preuves, limites et écarts dans le graphe.

### Alpha utilisable

- intégration d’un seul système externe, choisie pour fermer le scénario de référence ;
- export explicite du `ContextPack` vers cet outil ;
- lecture ou synchronisation d’un ensemble minimal d’objets et de statuts ;
- rapport final et références d’artefacts réinjectés dans AI Center ;
- relations `tracked_by`, `implemented_by` et `evidenced_by` ;
- comportement dégradé clair lorsque le fournisseur est indisponible ;
- journal des synchronisations et des décisions humaines.

Le premier connecteur peut viser GitHub, un outil de tickets ou un agent de code. Le critère de choix n’est pas la quantité d’actions automatisées, mais la capacité à démontrer le cycle complet **contexte sortant → travail dans l’outil existant → preuve entrante → contrôle de cohérence**.

### Capacités exclues

- IDE, terminal ou éditeur de code complet dans AI Center ;
- runner local ou cloud détenu par AI Center ;
- reproduction complète de Linear, Jira, Notion, Figma, GitHub ou d’une base de données ;
- orchestration autonome et opaque de plusieurs agents de production ;
- import ou réplication exhaustive d’un système d’entreprise ;
- création automatique de pull request ou publication distante sans politique explicite ;
- détection automatique exhaustive du drift entre tous les systèmes.

## Autorisations et confiance

Chaque adapter possède des scopes minimaux et explicites. Le modèle anticipe au minimum :

- lecture dans une source autorisée ;
- création ou mise à jour d’un objet externe autorisé ;
- transmission d’un `ContextPack` à l’agent ou à l’outil choisi ;
- accès réseau ;
- action hors du dépôt ;
- opération destructive ;
- publication distante, push ou création de PR.

Règle proposée pour le premier adapter réel : démarrer en lecture et import de preuves, puis n’autoriser une écriture externe que si elle est nécessaire à la preuve du cas d’usage. Les actions destructives, les sorties de périmètre, l’accès à des secrets et les publications distantes exigent une autorisation explicite dans AI Center ou dans l’outil qui demeure responsable de l’action.

Chaque action importante reste auditée et rattachée à une exécution.

## Périmètre fonctionnel priorisé

### P0 — indispensable à la preuve

- créer et retrouver un projet ;
- appliquer le template système `Software Product Delivery` ;
- ouvrir une session dans un nœud ;
- envoyer un message et reprendre une session persistante ;
- utiliser le profil d’agent correspondant au nœud ;
- créer, confirmer et versionner les entrées minimales ;
- créer et lire les relations du graphe ;
- produire un `FeatureBrief` conforme à son contrat ;
- évaluer le `ProductReadyGate` et expliquer les éléments bloquants ;
- compiler et inspecter un `ContextPack` pour l’agent Tech ;
- effectuer le handoff Produit → Tech sans reformulation ;
- produire un `TechnicalDeliveryPlan` conforme à son contrat ;
- relier le plan aux exigences et afficher leur couverture ;
- matérialiser les preuves documentaires et les limites ;
- déclencher le steward de projet sur commit ;
- détecter, expliquer et matérialiser une contradiction ;
- traiter son cycle de vie ;
- détecter une exigence sans preuve ;
- invalider ou revalider les projections affectées par une modification ;
- rattacher manuellement au moins une référence externe et l’utiliser comme preuve ;
- présenter toute la boucle dans une web app desktop et le client Linux.

### P1 — utile pour une bêta crédible

- session générale du Center puis rattachement à un projet ;
- dictée navigateur ;
- recherche sémantique dans les entrées du projet ;
- génération de vues documentaires Markdown ;
- suggestions de relations `references` ou `depends_on` ;
- écran condensé de santé du projet ;
- contrat générique `ToolAdapter` et journal de synchronisation ;
- un premier connecteur réel vers un système externe ;
- export d’un `ContextPack` et import d’un état ou d’une preuve réelle ;
- rattachement de tickets, documents, designs, fichiers, tests ou commits comme preuves ;
- ouverture de l’objet dans son outil source ;
- fonctionnement dégradé propre lorsque le connecteur est indisponible.

### P2 — après validation du MVP

- création et personnalisation de templates ;
- création libre de nœuds et sous-nœuds ;
- personnalisation des profils d’agents, outils et livrables ;
- agent Center inter-projets ;
- relations et contradictions automatiques entre projets ;
- couche de connaissances partagées au workspace ;
- promotion contrôlée d’une connaissance vers le workspace ;
- vue graphe globale en lecture seule puis éditable ;
- catalogue de connecteurs Slack, Notion, Linear, Jira, Figma, GitHub, GitLab et bases de données ;
- ingestion et mise à jour automatiques ;
- drift connaissance ↔ implémentation ;
- proactivité multi-signal ;
- automatisations planifiées ;
- applications iOS et Android ;
- éventuels modules natifs optionnels ne modifiant pas le cœur contextuel ;
- widget ambiant et conversation vocale complète ;
- collaboration, rôles, audit organisationnel et gouvernance.

## Ce qui est explicitement hors MVP

- expérience grand public non technique ;
- applications iOS et Android ;
- plusieurs utilisateurs dans le même workspace ;
- marketplace de templates ou d’agents ;
- nœuds et agents librement configurables par l’utilisateur ;
- graphe global monolithique chargé par tous les agents ;
- surveillance de tous les tokens par un watcher ;
- autonomie sans politique d’autorisation ;
- import exhaustif de l’écosystème d’entreprise ;
- remplacement des outils de ticketing, documentation, design, code ou données ;
- production et édition détaillées du code dans AI Center ;
- tests Android ou investissement mobile pendant la consolidation du MVP.

## Séquencement de réalisation

### Slice 0 — Prototype UX vertical

Construire d’abord une interface cliquable avec le scénario `Credits v2` déjà peuplé. Elle doit rendre visibles le projet, les nœuds Produit et Tech, le `FeatureBrief`, le handoff, le `ContextPack`, le plan technique, la couverture et la contradiction.

Cette slice utilise des fixtures typées et aucune infrastructure lourde. Son objectif est de vérifier que l’expérience paraît être un système de pilotage — pas un chat accompagné d’une sidebar.

### Slice 1 — Walking skeleton persistant

- web app desktop et API ;
- modèle Project / ContextNode / KnowledgeEntry / Edge ;
- template `Software Product Delivery` ;
- création et versionnement des entrées ;
- événements de domaine ;
- persistance locale ou base de développement ;
- remplacement progressif des fixtures de la Slice 0.

### Slice 2 — Boucle Produit

- création et reprise de sessions ;
- profil d’agent Produit ;
- propositions de mutations du graphe ;
- validation fluide des entrées ;
- génération du `FeatureBrief` ;
- évaluation du `ProductReadyGate`.

### Slice 3 — Context Compiler et handoff Tech

- compilation du `ContextPack` avec provenance et versions ;
- écran de prévisualisation du handoff ;
- profil d’agent Tech ;
- génération du `TechnicalDeliveryPlan` ;
- relations entre exigences, sections et preuves ;
- matrice de couverture.

Cette slice produit le premier moment de valeur complet et doit être testée avant tout investissement dans un connecteur ou une surface supplémentaire.

### Slice 4 — Steward et proactivité

- déclenchement sur commit ;
- sélection des candidats ;
- détection sémantique des contradictions ;
- détection des trous de couverture ;
- détail, sévérité et cycle de vie ;
- invalidation puis réévaluation des ContextPacks et livrables affectés.

### Slice 5 — Robustesse et évaluation

- jeu de cas contradictoires, compatibles et ambigus ;
- évaluation de l’extraction de connaissances et de la compilation de contexte ;
- faux positifs conservés comme feedback ;
- reprise de session et audit minimal ;
- onboarding ;
- instrumentation produit ;
- tests end-to-end du scénario de référence.

### Slice 6 — Premier connecteur externe, après validation du core

- choix d’un système externe et d’un objet précis ;
- interface `ToolAdapter` et mapping de l’identité externe ;
- export versionné du `ContextPack` ;
- lecture d’état et import d’une preuve réelle ;
- scopes d’autorisation et comportement dégradé ;
- preuves `tracked_by`, `implemented_by` ou `evidenced_by` ;
- test end-to-end démontrant que l’objet reste canonique dans l’outil externe.

### Contrat du suivi externe Alpha Context Proof

Le suivi GitHub observe un dépôt, un commit ou une pull request en lecture seule.
L’utilisateur peut déclarer explicitement le ContextPack qu’il a transmis à son
outil. AI Center conserve alors une chaîne pack versionné → tâche → exécution
observée → artefacts → preuves. Il ne déclenche pas ce travail dans l’outil et ne
confond pas un résultat CI avec une validation humaine.

Une preuve de cette chaîne cible explicitement un artefact et un livrable issu
du même ContextPack courant. Un ancien SHA, un pack obsolète ou une référence
inaccessible empêche une nouvelle validation. Les états et artefacts antérieurs
restent consultables ; le rechargement et la reprise d’une commande ne dupliquent
pas la chaîne. Les imports historiques sans pack déclaré restent identifiés
comme tels, sans leur attribuer rétroactivement une transmission.

## Critères de sortie fonctionnels

Le MVP est considéré complet lorsque :

1. un projet peut être créé avec ses nœuds Produit et Tech ;
2. chaque nœud utilise réellement un profil d’agent et un contexte différents ;
3. une session desktop peut produire des entrées atomiques confirmées ;
4. ces entrées persistent et sont retrouvées dans une autre session ;
5. un `FeatureBrief` est produit et contrôlé par le `ProductReadyGate` ;
6. un `ContextPack` Tech est compilé avec les versions et la provenance de ses sources ;
7. l’agent Tech produit un plan sans que l’utilisateur reformule le contexte Produit ;
8. le plan relie explicitement ses sections aux exigences et expose leur couverture ;
9. une contradiction entre Produit et Tech est détectée sans demande explicite ;
10. l’explication cite les deux entrées concernées ;
11. l’utilisateur peut accepter, résoudre ou rejeter le signal ;
12. une modification d’entrée invalide puis réévalue les projections dépendantes ;
13. au moins une absence de preuve est signalée et actionnable ;
14. une référence externe réelle peut être reliée à son exigence avec provenance ;
15. l’application et la session peuvent être fermées puis reprises sans perte d’état.

## Critères de qualité

- aucune connaissance confirmée ne disparaît après une coupure réseau ;
- les mutations du graphe sont versionnées et auditables ;
- l’agent indique son scope et les sources contextuelles utilisées ;
- un `ContextPack` est reproductible à partir des versions qu’il référence ;
- un livrable distingue clairement contenu produit, sources, couverture et incertitudes ;
- une contradiction expose sa confiance, sa sévérité et son explication ;
- les alertes bloquantes restent rares et justifiables ;
- un outil externe ne peut pas recevoir silencieusement plus de contexte ou de permissions que prévu ;
- les exports, synchronisations et retours de preuve sont structurés avant d’être résumés ;
- l’utilisateur peut toujours distinguer connaissance proposée, connaissance confirmée et résultat technique.

## Seuils d’évaluation IA provisoires

Ces seuils servent au dé-risquage interne, pas encore à une promesse commerciale :

- 100 % des contradictions structurelles déterministes du jeu de test détectées ;
- au moins 80 % de précision sur les contradictions sémantiques annotées ;
- au moins 70 % de rappel sur les contradictions sémantiques jugées importantes ;
- 100 % des éléments d’un `ContextPack` accompagnés de leur provenance ;
- 100 % des exigences affichées comme couvertes, partielles ou non couvertes ;
- aucune alerte `blocking` sans paire de sources et explication explicite ;
- moins d’un faux warning à haute priorité pour dix entrées confirmées lors des essais personnels.

Les seuils devront être révisés avec un corpus réel ; une précision faible détruirait plus rapidement la valeur qu’un rappel imparfait.

## Signaux de validation produit

- les entrées structurées sont réutilisées sans reformulation ;
- le changement de nœud réduit le besoin de réexpliquer le rôle ou le contexte ;
- le handoff Produit → Tech est jugé meilleur que copier-coller une spécification dans un nouveau chat ;
- l’utilisateur comprend ce qui compose le `ContextPack` sans devoir inspecter le graphe brut ;
- la matrice de couverture déclenche au moins une correction utile ;
- au moins une contradiction détectée est jugée réellement utile ;
- les faux positifs ne transforment pas l’inbox en bruit ;
- une session documentaire est considérée comme un résultat complet ;
- l’utilisateur préfère la boucle AI Center à la combinaison chat + document + nouveau chat spécialisé.

## Principaux risques et réponses de scope

| Risque | Réponse MVP |
| --- | --- |
| Construire un gestionnaire de chats banal | Graphe, agents scopés et contradiction obligatoires en P0 |
| Construire trop tôt une plateforme de graphe complète | Un projet, deux nœuds utiles et un vocabulaire minimal fermé |
| Fausse proactivité bruyante | Vérification sur commit, candidats filtrés, confiance, inbox et feedback |
| Contexte global trop gros | Résumés condensés et retrieval ciblé |
| Templates futurs impossibles à ajouter | Template et profils modélisés comme données dès la v0 |
| `ContextPack` opaque ou arbitraire | Provenance, versions, aperçu avant handoff et évaluation dédiée |
| Gates bureaucratiques | Contrôles automatiques, explications courtes et override justifié |
| Dérive vers un IDE ou un super-outil | Frontière d’adapter explicite et systèmes externes conservés comme sources de vérité |
| Trop d’autorisations | Contrats et permissions attachés à la tâche, confirmation ciblée pour le risque |
| UX de graphe complexe | Relations rendues par cartes et détails, sans canvas complet |
| Perte de la vision inter-projets | Identifiants globaux et endpoints d’arêtes extensibles, sans moteur inter-projets en v0 |

## Décisions structurantes proposées

1. **Le graphe n’est pas post-MVP** : son atome utile est dans le P0.
2. **La contradiction est la première proactivité** : elle est la fonction différenciante à évaluer avant les autres automatisations.
3. **Le template est fixe mais data-driven** : personnalisation différée, architecture préservée.
4. **Les agents sont scopés par nœud** : même moteur possible, profils et contextes différents.
5. **Le steward agit sur les commits de connaissance** : pas de watcher sur chaque token.
6. **Le Markdown est une projection** : la source de vérité est structurée et requêtable.
7. **Le Context Compiler et les contrats de livrables sont dans le P0** : ils transforment le graphe en système de travail.
8. **Le premier handoff est Produit → Tech** : il constitue la boucle de validation prioritaire.
9. **La consolidation cible le web desktop et Linux** : Android, iOS et les tests mobiles sont explicitement différés.
10. **La production appartient aux outils spécialisés** : AI Center transmet le contexte, observe le résultat et rattache les preuves ; il ne devient ni runner, ni IDE, ni clone des systèmes connectés.
11. **Les relations inter-projets sont préparées dans le modèle, pas automatisées dans la première version**.
12. **L’intégration précède le remplacement** : toute fonction native future doit être optionnelle et justifiée par une valeur supérieure à une référence ou un adapter.

## Arbitrages restant à confirmer

- Les mutations importantes sont-elles confirmées une par une ou validées en lot en fin de séquence ?
- Une contradiction `candidate` apparaît-elle immédiatement, ou seulement après un seuil de confiance ?
- Quel degré d’édition manuelle offrir dans les livrables structurés sans les transformer en traitement de texte ?
- Le `ProductReadyGate` peut-il être contourné avec justification, ou seulement passer avec warning ?
- Quel système externe et quel type d’objet ferment le mieux le premier cycle réel : GitHub/PR, Linear/ticket, Notion/document ou agent de code/tâche ?
- Le premier connecteur doit-il rester en lecture/import, ou une écriture minimale est-elle indispensable à la démonstration ?
- Quel projet réel et quelles données non sensibles serviront de corpus alpha ?

## Documents suivants recommandés

Le prochain travail ne doit pas être un document supplémentaire, mais la Slice 0 visible. Les documents d’architecture seront produits au fil des décisions imposées par ce prototype, dans l’ordre suivant :

1. `05-domain-model.md` — entités, agrégats, états, événements et invariants ;
2. `06-system-architecture.md` — web app, API/control plane, moteur de contexte, workers et protocoles ;
3. `07-security-and-trust-model.md` — autorisations, isolation et audit ;
4. `08-ux-flows.md` — parcours desktop, mutations du graphe, handoff, contradiction et intégrations ;
5. `09-roadmap.md` — slices, dépendances et critères de passage.

Le modèle de domaine doit précéder l’architecture technique : ici, les distinctions entre nœud, entrée, arête, insight, agent et exécution déterminent directement les frontières du système.
