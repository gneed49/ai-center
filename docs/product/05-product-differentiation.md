# AI Center — Stratégie de différenciation

> Statut : proposition stratégique à challenger — version 0.1  
> Date : 15 août 2026  
> Objet : définir ce qui relève de la commodité, de la valeur produit et d’un avantage défendable.

## Résumé exécutif

AI Center ne doit pas être positionné comme une meilleure application mobile pour piloter un agent de code. Cursor, Claude Code, Codex et GitHub permettent déjà de lancer, reprendre ou superviser du travail agentique depuis le web, le mobile, le cloud ou une machine locale.

AI Center ne peut pas non plus se différencier uniquement par la présence d’agents spécialisés, de connecteurs MCP, d’automatisations ou d’un knowledge graph. Atlassian, Notion, Revo et d’autres plateformes combinent déjà plusieurs de ces capacités.

La différenciation recommandée est plus précise :

> **AI Center est la couche de pilotage contextuel qui transforme les décisions d’un projet en workflows exécutables, fournit à chaque agent le bon contexte et vérifie en continu que le travail produit reste cohérent avec l’intention.**

Les agents de code, modèles et outils externes deviennent des exécutants interchangeables. AI Center possède :

- l’organisation du projet ;
- le graphe des décisions et connaissances ;
- les rôles et périmètres des agents ;
- les contrats de livrables ;
- les passages de relais ;
- les contrôles de cohérence ;
- la traçabilité entre intention et preuve.

La mobilité reste importante pour l’expérience personnelle visée, mais elle devient un canal de distribution et d’usage, pas la raison fondamentale d’acheter ou d’adopter le produit.

## Baseline concurrentielle en août 2026

### Agents de code et mobilité

- [Cursor Cloud Agents](https://cursor.com/docs/cloud-agent) se lancent depuis iOS, le web, le desktop, Slack, GitHub, Bitbucket, Linear ou une API ; Android dispose d’une PWA.
- [Cursor pour iOS](https://cursor.com/docs/cloud-agent/mobile) contrôle des agents locaux ou cloud, suit leur travail et permet de revoir ou fusionner leurs pull requests.
- [Claude Code Remote Control](https://code.claude.com/docs/en/remote-control) reprend depuis iOS, Android ou le navigateur une session qui continue de s’exécuter localement avec ses fichiers, outils, MCP et workflows.
- [Codex cloud](https://learn.chatgpt.com/docs/cloud) exécute des tâches parallèles dans des environnements cloud depuis le web, GitHub, Linear ou Slack.
- Le [Codex SDK](https://learn.chatgpt.com/docs/codex-sdk) permet déjà d’intégrer Codex comme spécialiste dans un workflow orchestré plus large.
- [GitHub Copilot cloud agent](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent) peut être lancé depuis GitHub Mobile, les IDE, une API, le CLI, MCP, Slack, Jira, Linear ou Teams et accepte des agents personnalisés.

Conclusion : démarrer un agent, suivre son exécution, travailler à distance, connecter des MCP et récupérer une pull request sont des fonctionnalités attendues du marché.

### Contexte, graphes et agents spécialisés

- [Atlassian Rovo](https://www.atlassian.com/software/rovo) combine recherche, connaissance organisationnelle, agents et assistance proactive.
- Le [Teamwork Graph d’Atlassian](https://www.atlassian.com/blog/company-news/teamwork-graph-team-26) peut être lu et modifié par CLI ou MCP ; des agents externes peuvent donc exploiter et enrichir ce graphe.
- [Notion AI](https://www.notion.com/product/ai) combine recherche multi-source, agents personnalisés, actions, gouvernance et choix de modèles.
- [Revo](https://www.revo.ai/) construit un graphe à partir des communications, détecte des actions et transmet aux LLM des paquets de contexte préparés.

Conclusion : « nous avons un knowledge graph, des agents et de la proactivité » n’est pas encore un positionnement. Il faut préciser quel objet le graphe modélise, quelles décisions il améliore et quel workflow il rend nettement supérieur.

## Ce qui ne constitue pas une différenciation durable

| Capacité | Classement | Pourquoi |
| --- | --- | --- |
| Interface de chat | Commodité | Disponible partout et rapidement reproductible |
| Application mobile | Canal | Très utile, mais déjà couvert par les agents majeurs |
| Exécution locale ou cloud | Infrastructure | Plusieurs fournisseurs couvrent déjà les deux |
| Projet, workspace et historique | Prérequis | Organisation standard des assistants et outils SaaS |
| Connexions MCP | Prérequis | Standard d’interopérabilité, pas avantage propriétaire |
| Agent spécialisé configurable | Prérequis émergent | Cursor, GitHub, Rovo, Notion et autres le proposent |
| Knowledge graph générique | Brique | Puissant, mais présent dans des plateformes établies |
| Automatisations planifiées | Prérequis émergent | Facilement couvertes par agents et outils workflow |
| Multi-modèle | Prérequis stratégique | Réduit le lock-in, mais ne crée pas seul de valeur métier |
| Contradiction détectée ponctuellement | Fonction différenciante | Utile, mais copiable si elle reste isolée |

Le produit ne doit pas supprimer ces capacités. Il doit refuser d’en faire sa promesse principale.

## Catégorie produit recommandée

Deux formulations sont pertinentes :

### Formulation fonctionnelle

**Contextual Project Control Plane** — plan de contrôle contextuel des projets.

Cette formulation insiste sur le fait qu’AI Center se trouve au-dessus des agents et des outils, décide quel contexte leur fournir et gouverne leurs actions.

### Formulation marché

**Agentic Project Operating System** — système d’exploitation agentique des projets.

Cette formulation porte mieux la vision longue : structure du projet, agents, workflows, mémoire, automatisations et supervision dans un même système.

### Positionnement recommandé

> AI Center transforme la connaissance d’un projet en système de pilotage exécutable : il structure les décisions, prépare le contexte pour les agents, orchestre les passages entre disciplines et vérifie que l’exécution reste cohérente avec l’intention.

### Formule courte

> **From intent to evidence, without context drift.**

### Explication imagée

> Cursor, Codex ou Claude fournissent des travailleurs puissants. AI Center leur donne une fonction, un périmètre, une source de vérité, un contrat de travail et un manager.

## La pile de différenciation

La valeur ne repose pas sur une fonctionnalité isolée, mais sur la combinaison cohérente de sept mécanismes.

### 1. Le graphe comme état opérationnel du projet

La plupart des systèmes utilisent la connaissance pour rechercher des réponses ou enrichir un prompt. AI Center doit utiliser le graphe pour représenter l’état décisionnel et opérationnel du projet :

- ce qui est décidé ;
- ce qui reste ouvert ;
- ce qui dépend de quoi ;
- ce qui contredit quoi ;
- ce qui doit être produit ;
- ce qui a réellement été implémenté ;
- quelles preuves valident le résultat.

Le graphe ne sert donc pas seulement à répondre à « que savons-nous ? », mais à répondre à :

- que pouvons-nous faire maintenant ?
- qu’est-ce qui bloque ?
- quel contexte doit recevoir cet agent ?
- quelle décision sera impactée par ce changement ?
- le résultat respecte-t-il toujours l’intention ?

### 2. Les templates comme operating models exécutables

Un template ne doit pas être un ensemble de dossiers ou de pages. Il décrit une manière complète de conduire un type de projet.

Un `ProjectTemplate` pourra contenir :

| Élément | Fonction |
| --- | --- |
| `NodeTopology` | Nœuds, sous-nœuds et relations initiales |
| `AgentProfile` | Rôle, instructions, outils et scope de chaque agent |
| `KnowledgeSchema` | Types d’entrées attendues dans chaque nœud |
| `DeliverableContract` | Forme, contenu et critères d’un livrable |
| `Gate` | Conditions nécessaires avant un passage de relais |
| `Trigger` | Événements lançant une vérification ou une action |
| `Policy` | Permissions et règles de validation |
| `View` | Représentation de l’information adaptée au rôle |

Le premier template `Software Product Delivery` peut imposer Produit + Tech. Plus tard, une agence, une startup ou une équipe pourra adapter ou créer son propre operating model sans programmer un réseau de prompts et de slash commands.

### 3. Le Context Compiler

Avant de déléguer une tâche, AI Center ne transmet pas tout le graphe ni un transcript brut. Il compile un `ContextPack` adapté à la tâche et à l’exécutant.

Un `ContextPack` contient notamment :

- l’objectif ;
- les décisions et règles applicables ;
- les exigences et critères d’acceptation ;
- les contraintes ;
- les dépendances pertinentes ;
- les questions encore ouvertes ;
- les artefacts et sources nécessaires ;
- les permissions ;
- le contrat de résultat attendu ;
- la provenance de chaque élément.

Le même contexte canonique peut être compilé différemment pour Cursor, Codex, Claude Code, un agent produit ou un outil de design.

Ce mécanisme transforme AI Center en couche portable au-dessus des fournisseurs et réduit la perte de contexte lors des handoffs.

### 4. L’orchestration guidée par l’UX

Le produit ne demande pas à l’utilisateur de construire lui-même son workflow agentique.

L’interface exprime naturellement :

- où se trouve l’utilisateur dans le projet ;
- quel agent est actif et pourquoi ;
- quelles connaissances sont consultées ou modifiées ;
- quelles décisions manquent ;
- quel livrable est attendu ;
- quelles actions sont maintenant possibles ;
- quel passage de relais est proposé.

Les commandes, prompts, skills et appels d’outils restent accessibles aux utilisateurs avancés, mais ne constituent pas le langage principal de l’expérience.

### 5. Le moteur de cohérence actif

La contradiction est le premier cas, pas la destination finale.

Le moteur pourra progressivement détecter :

- contradictions ;
- décisions manquantes ;
- dépendances non traitées ;
- connaissance devenue obsolète ;
- changement impactant plusieurs nœuds ;
- livrable incomplet par rapport à son contrat ;
- implémentation non reliée à une exigence ;
- drift entre décision, ticket, code et comportement observé ;
- risque nécessitant une validation humaine.

Chaque insight doit être sourcé, expliqué, priorisé et actionnable. La précision compte davantage que la quantité.

### 6. La traçabilité intention → preuve

AI Center relie :

```text
Intention
→ décision
→ exigence
→ tâche
→ ContextPack
→ exécution
→ artefact
→ validation
→ résultat observé
```

Cette chaîne permet de répondre à des questions auxquelles un chat, un dépôt ou un ticket répondent difficilement seuls :

- pourquoi ce code existe-t-il ?
- quelle décision cette PR implémente-t-elle ?
- quelles exigences ne possèdent aucune preuve ?
- qu’est-ce qui doit être revérifié si cette règle change ?
- quelles décisions ont produit un résultat différent de l’objectif ?

### 7. Des exécutants interchangeables

AI Center ne cherche pas à battre chaque agent spécialisé sur son propre terrain.

Il doit pouvoir déléguer à :

- Cursor ;
- Codex ;
- Claude Code ;
- GitHub Copilot ou un agent GitHub ;
- un runner local ;
- un agent cloud ;
- plus tard, des agents produit, design, recherche ou opérations.

La valeur réside dans le choix du bon exécutant, la préparation de son contexte, le contrôle de son contrat et l’intégration de son résultat dans le graphe.

## Expérience produit directrice

### La home n’est pas un chat vide

Le Center doit afficher :

- projets actifs ;
- décisions en attente ;
- contradictions et risques ;
- agents ou exécutions en cours ;
- livrables récemment produits ;
- actions nécessitant l’utilisateur ;
- prochaines actions proposées.

L’agent global reste accessible, mais la page d’accueil présente l’état du travail avant de présenter une zone de saisie.

### Le projet n’est pas un dossier

Un projet expose :

- ses nœuds ou workstreams ;
- son état de cohérence ;
- ses objectifs et décisions actives ;
- ses livrables ;
- ses dépendances ;
- ses sessions et exécutions ;
- les changements récents du graphe.

### Le chat est un outil local au contexte

Dans un nœud, la conversation sert à :

- explorer ;
- challenger ;
- décider ;
- produire des mutations structurées ;
- préparer un livrable ou un handoff.

Le résultat important apparaît hors du transcript sous forme d’objets durables.

### Les handoffs ne sont pas des copier-coller

Lorsque Produit transmet à Tech, ou Tech à un agent de code :

1. AI Center vérifie les gates applicables ;
2. compile le ContextPack ;
3. indique ce qui est inclus et encore incertain ;
4. propose l’exécutant ;
5. suit le contrat de résultat ;
6. rattache les preuves au graphe.

### La proactivité arrive dans une decision inbox

Les alertes ne doivent pas surgir arbitrairement dans toutes les conversations.

Une inbox centralise :

- contradiction à résoudre ;
- décision demandée ;
- validation d’un livrable ;
- exécution bloquée ;
- impact détecté ;
- suggestion de promotion ou de lien inter-projets.

Chaque item propose une action claire et peut être différé, accepté, rejeté ou résolu.

## Le véritable produit derrière les templates

La vision ne consiste pas seulement à permettre la personnalisation de prompts.

L’utilisateur avancé pourra progressivement modifier :

- les nœuds de son organisation ;
- les agents liés ;
- les types de connaissances ;
- les outils disponibles ;
- les formes de livrables ;
- les gates entre workstreams ;
- les vérifications proactives ;
- les politiques d’autorisation ;
- les règles de compilation du contexte.

Il configure ainsi son propre système opératoire de projet sans écrire tout le workflow agentique à la main.

À terme, un template peut devenir un actif partageable ou commercialisable. Ce n’est pas une marketplace de prompts : c’est un operating model complet et exécutable.

## Wedge recommandé

Le marché horizontal des assistants d’entreprise est déjà fortement occupé. Le premier wedge doit rester étroit :

> **La continuité entre intention produit et implémentation pour les solo builders, fondateurs techniques et petites équipes qui travaillent avec plusieurs agents.**

Le premier template est consacré à la livraison d’une fonctionnalité logicielle :

1. cadrage Produit ;
2. exigences et décisions ;
3. contrôle de cohérence ;
4. passage Tech ;
5. plan et critères d’acceptation ;
6. délégation à un agent de code ;
7. retour des tests et artefacts ;
8. vérification de couverture ;
9. mise à jour du graphe.

Ce wedge exploite l’expérience réelle du premier utilisateur et crée une boucle plus profonde qu’un gestionnaire de tâches généraliste.

## Conséquences pour le MVP

### Ce qui devient central

- template Produit + Tech ;
- agents de nœuds préconfigurés ;
- connaissances atomiques ;
- relations et provenance ;
- mutation du graphe depuis la conversation ;
- contradiction proactive ;
- ContextPack compilé ;
- handoff Produit → Tech → agent de code ;
- contrat de livrable ;
- résultat et preuves réinjectés dans le graphe.

### Ce qui change de statut

| Élément | Ancien rôle implicite | Nouveau rôle |
| --- | --- | --- |
| Android | Élément du wedge | Première surface d’usage |
| Runtime macOS | Cœur d’exécution différenciant | Premier adapter d’exécution choisi pour l’usage personnel |
| Agent de code | Fonction du produit | Exécutant externe interchangeable |
| Chat | Interface principale | Outil contextuel dans un workflow guidé |
| Markdown | Livrable/source de contexte | Projection et format d’échange |
| Graphe | Mémoire structurée | État opérationnel et moteur de décision |
| Template | Arborescence initiale | Operating model exécutable |

### Révision recommandée du P0

Le P0 ne doit pas construire un système complet de contrôle du Mac. Il doit intégrer un seul exécutant derrière une interface minimale.

La priorité devient :

1. graphe et mutations structurées ;
2. agents Produit et Tech ;
3. contradiction et decision inbox ;
4. Context Compiler ;
5. handoff à un exécutant ;
6. rapport et preuves réinjectés.

Le runner local peut être ce premier exécutant parce qu’il répond au besoin personnel, mais l’architecture ne doit pas identifier AI Center à ce runner.

## Moat potentiel

### 1. Le schéma opérationnel du projet

Une ontologie pragmatique reliant décisions, règles, exigences, tâches, exécutions et preuves peut devenir difficile à reproduire si elle est affinée sur des usages réels.

### 2. La bibliothèque d’operating models

Templates, profils d’agents, knowledge schemas, gates, contrats et évaluations forment un actif plus défendable qu’une collection de prompts.

### 3. Le Context Compiler

La capacité à produire un contexte minimal, sourcé et adapté à chaque agent, puis à mesurer son efficacité, peut devenir une technologie centrale.

### 4. Le feedback sur la cohérence

Faux positifs, contradictions acceptées, résolutions, réouvertures et impacts constituent une boucle d’amélioration spécifique aux workflows projet.

### 5. La traçabilité multi-outils

Le lien durable entre intention et preuves provenant de plusieurs exécutants crée une valeur cumulative : changer de modèle ou d’agent ne détruit pas l’historique décisionnel.

### 6. L’habitude organisationnelle

Lorsque les décisions, handoffs et validations passent par AI Center, il devient le système de continuité du projet. Cette position est plus défendable qu’une simple interface de chat.

Les données restent la propriété des utilisateurs. Le moat ne doit pas dépendre de leur enfermement, mais de la qualité du modèle, des workflows et des évaluations.

## Risques stratégiques

### Les plateformes établies peuvent remonter dans la chaîne

Atlassian, Notion, Cursor ou GitHub peuvent ajouter davantage de contexte, de workflows et de gouvernance. AI Center doit progresser plus vite sur un workflow très précis plutôt que les affronter horizontalement.

### Le graphe peut devenir une charge de maintenance

Si l’utilisateur doit classer manuellement chaque entrée ou dessiner les relations, l’adoption échouera. L’IA doit proposer et maintenir le graphe ; l’humain valide les mutations importantes.

### La flexibilité peut détruire le time-to-value

Un constructeur universel de templates et d’agents créerait une page blanche aussi intimidante qu’un chat vide. Le produit doit être opinionated par défaut et personnalisable progressivement.

### La proactivité peut devenir du bruit

Chaque insight doit avoir une source, un impact, une confiance et une action. Un système qui produit plus d’alertes que de décisions utiles recrée le problème qu’il prétend résoudre.

### Le positionnement peut rester trop abstrait

« Contextual control plane » décrit l’architecture mais ne vend pas encore un résultat. Le wedge logiciel doit produire une promesse concrète : moins de reconstruction de contexte, moins de décisions perdues et moins de divergence entre les specs et le code.

## Principes directeurs

1. **Le chat n’est jamais la home du produit.**
2. **L’utilisateur exprime son intention ; AI Center porte l’orchestration.**
3. **Toute connaissance durable est typée, sourcée et versionnée.**
4. **Tout agent possède un scope et un contrat explicites.**
5. **Tout handoff reçoit un ContextPack et des critères de sortie.**
6. **Toute proactivité est expliquée et actionnable.**
7. **Toute exécution retourne des preuves et met à jour le graphe.**
8. **Les valeurs par défaut sont opinionated ; la personnalisation est progressive.**
9. **Les exécutants restent interchangeables.**
10. **Mobile, web et desktop sont des surfaces d’un même système, pas sa proposition de valeur.**
11. **La connaissance n’est pas copiée entre projets : elle est liée ou promue explicitement.**
12. **La précision de la cohérence prime sur le volume des alertes.**

## Carte de positionnement

| Couche | Produits principalement présents | Rôle d’AI Center |
| --- | --- | --- |
| Modèles et agents d’exécution | Cursor, Codex, Claude Code, Copilot | Les utiliser, les router et évaluer leurs résultats |
| Dépôts et delivery | GitHub, GitLab, CI/CD | Collecter les artefacts et preuves |
| Tickets et coordination | Linear, Jira | Synchroniser tâches et état, sans nécessairement remplacer |
| Documents et connaissance | Notion, Confluence, Drive | Importer ou référencer les sources de vérité existantes |
| Graphe organisationnel général | Atlassian Teamwork Graph, plateformes de recherche | Se concentrer d’abord sur le graphe décisionnel du projet logiciel |
| Pilotage contextuel du projet | Espace encore fragmenté | Posséder le workflow intention → contexte → exécution → preuve |

Cette carte décrit des centres de gravité, pas des frontières hermétiques.

## Décision stratégique proposée

> AI Center ne doit pas chercher à devenir l’agent qui sait tout faire. Il doit devenir le système qui permet à plusieurs agents de faire le bon travail, avec le bon contexte, dans le bon ordre, sans perdre la cohérence du projet.

Si cette décision est acceptée, `03-mvp-scope.md` devra être révisé pour :

- retirer la mobilité et le runtime local de la formulation du différenciateur ;
- introduire explicitement `ContextPack`, `DeliverableContract`, `Gate` et `Evidence` ;
- repositionner le runner macOS comme premier adapter ;
- placer le handoff Produit → Tech → exécution au centre du scénario de référence ;
- évaluer la couverture intention → preuve en plus de la détection des contradictions.

## Questions à challenger

1. La catégorie `Agentic Project Operating System` est-elle compréhensible, ou faut-il rester sur une promesse plus concrète ?
2. Le premier wedge doit-il viser le solo builder, la petite équipe produit–tech ou l’agence de développement ?
3. Jusqu’où un template doit-il imposer un workflow avant de devenir trop rigide ?
4. Le graphe doit-il rester invisible la plupart du temps ou devenir une vue centrale du projet ?
5. Le premier exécutant doit-il être un runner local, un agent cloud ou une abstraction offrant les deux ?
6. Quelle preuve de résultat doit clôturer le premier workflow : document validé, tests, preview, commit ou pull request ?
