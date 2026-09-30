# AI Center — Personas et cas d’usage

> Statut : cadrage consolidé — version 0.2
>
> Date : 24 août 2026
>
> Périmètre : pilotage contextuel Produit → Tech et intégration progressive des outils existants.

## Objectif

Ce document identifie les utilisateurs pour lesquels AI Center doit devenir
excellent en premier. Il ne décrit pas une cible marketing définitive : il sert
à arbitrer le MVP, la place des connecteurs et les capacités qu’AI Center ne
doit pas chercher à reproduire.

## Décisions de cadrage

- AI Center est un plan de contrôle contextuel, pas un agent de code.
- La source de vérité centrale porte la signification : intentions, décisions,
  règles, exigences, relations, handoffs et preuves.
- Les outils existants conservent leurs objets métier : tickets, documents,
  designs, dépôts, données et exécutions.
- Les agents spécialisés reçoivent un ContextPack et un contrat adaptés à leur
  rôle ; leurs résultats reviennent sous forme de références, preuves et
  propositions contextuelles.
- La validation humaine distingue une proposition de la vérité confirmée.
- Le premier workflow prouvé est Produit → Tech.
- L’adoption doit fonctionner sans migration globale des outils.
- Android et la mobilité sont entièrement différés.

## Persona primaire alpha — le lead Produit–Tech AI-native

### Profil

Fondateur technique, product engineer, staff engineer ou lead Produit–Tech
menant un ou plusieurs projets. Il utilise déjà plusieurs agents et outils :

- un assistant généraliste pour explorer ;
- Codex, Claude Code, Cursor, OpenCode ou une autre CLI pour le code ;
- GitHub ou GitLab pour les dépôts ;
- Linear ou Jira pour les tickets ;
- Notion, Confluence ou Drive pour la documentation ;
- Figma pour le design ;
- une ou plusieurs plateformes de données.

Il ne veut pas remplacer cette stack. Il veut arrêter de reconstruire le
contexte entre les agents, les disciplines et les systèmes.

### Comportements déterminants

- alterne Produit, Tech, Design et exécution ;
- utilise plusieurs agents selon la tâche ;
- copie encore des décisions entre chats, documents et tickets ;
- conserve des informations contradictoires sans le savoir ;
- réexplique fréquemment le projet lors d’un changement d’outil ;
- évalue les résultats dans leurs systèmes de référence ;
- souhaite introduire AI Center progressivement sur un workflow limité.

### Motivations

- garder la maîtrise du contexte partagé ;
- éviter les répétitions et pertes lors des handoffs ;
- rendre les agents réellement spécialisés sans multiplier les silos ;
- savoir quelles sources ont guidé une décision ou un résultat ;
- détecter plus tôt contradictions, oublis et dérive ;
- changer d’agent ou de fournisseur sans repartir de zéro ;
- prouver que le travail produit respecte encore l’intention.

### Frustrations

- un ticket ne contient pas toute la décision Produit ;
- un document ne sait pas si le code l’a réellement respecté ;
- un agent Tech ne voit pas toujours les contraintes découvertes par Produit ;
- les conversations importantes ne deviennent pas automatiquement du contexte
  durable ;
- le volume de contexte rend les prompts longs, coûteux et peu fiables ;
- personne ne sait clairement quelle version d’une décision est active ;
- l’usage de l’IA devient central mais reste difficile à gouverner collectivement.

### Critères de confiance

- chaque connaissance structurante expose source, auteur, version et statut ;
- l’utilisateur voit pourquoi une information appartient au ContextPack ;
- aucune proposition ne devient vérité sans validation explicite ;
- un conflit cite précisément ses deux sources ;
- un résultat externe reste consultable dans son outil d’origine ;
- la synchronisation ne modifie pas silencieusement un système de référence ;
- les permissions et le périmètre de chaque agent sont compréhensibles.

## Persona secondaire — l’équipe Produit–Design–Tech

Une équipe de quelques personnes possède déjà des pratiques et outils établis.
Plusieurs membres utilisent des agents différents. Sa douleur principale est la
fragmentation du contexte collectif :

- Produit travaille dans Notion et Linear ;
- Design travaille dans Figma ;
- Tech travaille dans GitHub et ses agents de code ;
- les décisions passent d’un système à l’autre par copier-coller ;
- les agents ne partagent ni mémoire ni règles de gouvernance.

Cette équipe valorise :

- un contexte commun sans changement d’outils ;
- des handoffs explicites ;
- des permissions par workspace et projet ;
- une Decision Inbox partagée ;
- une traçabilité intention → ticket → design → code → preuve ;
- un audit des décisions humaines et propositions agentiques.

La collaboration complète n’est pas requise pour la première preuve, mais le
modèle doit la préparer.

## Persona futur — l’organisation multi-projets

Une organisation veut comprendre et gouverner l’usage de l’IA à travers
plusieurs équipes, domaines et fournisseurs. Elle a besoin :

- de politiques de partage ;
- d’un contexte inter-projets contrôlé ;
- de modèles opératoires ;
- de SSO et gouvernance ;
- d’une observabilité de la qualité et des coûts ;
- d’un audit transverse ;
- de signaux de dérive ou d’impact entre projets.

Ce persona porte l’ambition longue, pas le périmètre de l’alpha.

## Anti-personas

### Le développeur cherchant un nouvel IDE agentique

Il souhaite écrire, exécuter et déboguer du code dans une interface comparable à
Codex ou Cursor. AI Center doit lui fournir le bon contexte et le renvoyer vers
son outil de code, pas reproduire cet outil.

### L’équipe cherchant à remplacer immédiatement toute sa stack

AI Center n’est pas un programme de migration vers un ticketing, un wiki, un
outil de design et un dépôt propriétaires.

### L’utilisateur cherchant un simple moteur de recherche documentaire

La recherche est une brique. La valeur vient de la structuration, des contrats,
des handoffs, de la cohérence et de la traçabilité vers les preuves.

### L’organisation exigeant immédiatement une plateforme complète

SSO avancé, centaines de connecteurs, gouvernance multi-entités et automatisation
totale dépassent la première preuve de valeur.

## Jobs to be done prioritaires

### JTBD-1 — Reprendre la main sur le contexte

> Lorsque plusieurs personnes et agents travaillent sur un projet, je veux une
> vue commune de ce qui est décidé, ouvert ou contradictoire afin d’éviter que
> chaque acteur avance avec sa propre vérité.

### JTBD-2 — Transformer les échanges en connaissance durable

> Lorsque des décisions ou exigences émergent d’une conversation, je veux les
> structurer, les sourcer et les confirmer sans maintenir manuellement un
> document parallèle.

### JTBD-3 — Passer d’une discipline à l’autre

> Lorsque Produit transmet une évolution à Tech, je veux que l’agent Tech reçoive
> exactement le contexte nécessaire sans que je reformule l’historique.

### JTBD-4 — Utiliser le meilleur outil sans perdre le contexte

> Lorsque le travail doit continuer dans Linear, Figma, Notion, GitHub ou un
> agent de code, je veux transmettre un contrat et récupérer les preuves sans
> déplacer artificiellement tout le travail dans AI Center.

### JTBD-5 — Challenger la cohérence

> Lorsqu’une nouvelle décision ou un résultat externe contredit le projet, je
> veux comprendre le conflit, ses sources et son impact avant qu’il ne se propage.

### JTBD-6 — Prouver la couverture

> Lorsqu’un livrable paraît terminé, je veux savoir quelles exigences sont
> couvertes, partielles ou sans preuve.

### JTBD-7 — Réviser sans dérive

> Lorsqu’une règle change, je veux que les ContextPacks, handoffs, tickets et
> livrables dépendants deviennent explicitement obsolètes ou soient réévalués.

### JTBD-8 — Adopter progressivement

> Lorsque j’introduis AI Center dans une équipe équipée, je veux commencer avec
> un projet et quelques sources sans imposer une migration globale.

## Cas d’usage

| Priorité | ID | Cas d’usage | Valeur |
| --- | --- | --- | --- |
| P0 | UC-01 | Créer un projet avec scopes Produit et Tech | Frontière contextuelle explicite |
| P0 | UC-02 | Challenger une intention et proposer des connaissances | Contexte structuré |
| P0 | UC-03 | Confirmer, rejeter ou réviser une proposition | Autorité humaine |
| P0 | UC-04 | Évaluer un gate et produire un Feature Brief | Passage contrôlé |
| P0 | UC-05 | Compiler un ContextPack Tech sélectif | Continuité sans dump |
| P0 | UC-06 | Réaliser le handoff sans reformulation | Fluidité inter-disciplines |
| P0 | UC-07 | Produire un plan Tech contextualisé et traçable | Contrat de travail exploitable |
| P0 | UC-08 | Relier un artefact ou résultat externe | Intention → preuve |
| P0 | UC-09 | Détecter et résoudre une contradiction réelle | Cohérence active |
| P0 | UC-10 | Invalider et recompiler les projections impactées | Prévention du drift |
| P1 | UC-11 | Connecter un dépôt GitHub ou GitLab | Preuves de code |
| P1 | UC-12 | Connecter Linear ou Jira | Tickets contextualisés |
| P1 | UC-13 | Référencer Notion ou Confluence | Provenance documentaire |
| P1 | UC-14 | Référencer Figma | Contexte et preuves de design |
| P1 | UC-15 | Transmettre un ContextPack à une CLI ou un agent externe | Exécutant interchangeable |
| P2 | UC-16 | Partager un workspace entre plusieurs rôles | Contexte d’équipe |
| P2 | UC-17 | Détecter des impacts inter-projets | Contexte d’organisation |
| P2 | UC-18 | Créer des operating models personnalisés | Standardisation |

## Parcours principal — Produit → outil externe → preuve

1. L’utilisateur crée ou ouvre un projet.
2. Il relie les sources utiles ou référence leurs objets existants.
3. Dans le scope Produit, il expose une intention.
4. L’agent Produit challenge et propose des unités de connaissance.
5. L’utilisateur confirme, rejette ou corrige chaque proposition importante.
6. Le ProductReadyGate expose les manques.
7. Le Feature Brief est produit depuis les versions confirmées.
8. AI Center compile un ContextPack sélectif pour le scope Tech.
9. Le handoff ouvre une session Tech sans reformulation.
10. L’agent Tech produit un plan et un contrat exploitables par l’outil choisi.
11. L’utilisateur transmet ce contexte à un agent de code, un outil de tickets,
    de design ou de documentation.
12. L’outil externe produit le travail dans son système de référence.
13. AI Center réintègre les références, statuts, preuves et éventuelles
    propositions de connaissance.
14. Le moteur de cohérence recalcule la couverture et les contradictions.
15. L’utilisateur traite les insights puis valide ou révise le contexte.

Le parcours est réussi même si AI Center n’a produit aucun code, ticket ou
design lui-même. Sa valeur réside dans la continuité, la qualité du contrat et
le contrôle du résultat.

## Moment de valeur

> « Produit, Tech et nos agents ont travaillé dans leurs outils habituels. Nous
> n’avons pas répété le contexte, AI Center a relié les décisions aux résultats
> et nous a signalé une incohérence avant qu’elle ne devienne un défaut. »

## Modèle contextuel requis

| Objet | Rôle |
| --- | --- |
| `Workspace` | Frontière de propriété, permissions et connaissance |
| `Project` | Initiative ayant objectif, état et livrables |
| `ContextNode` | Scope spécialisé tel que Produit, Design ou Tech |
| `KnowledgeEntry` | Décision, règle, exigence, contrainte ou question versionnée |
| `ExternalReference` | Identité stable d’un ticket, document, design, dépôt ou autre objet externe |
| `AgentProfile` | Rôle, contexte, outils, permissions et contrat d’un agent |
| `Session` | Continuité d’une interaction dans un scope |
| `ContextPack` | Projection minimale et immutable pour une tâche et un exécutant |
| `Handoff` | Passage explicite entre scopes ou systèmes |
| `DeliverableContract` | Résultat attendu et règles de couverture |
| `Deliverable` | Livrable structuré ou référence vers un résultat externe |
| `Task` | Travail à transmettre ou à suivre, sans imposer un gestionnaire interne |
| `Execution` | Référence vers une tentative réalisée par un agent ou outil externe |
| `Artifact` | Résultat ou référence : document, ticket, design, commit, PR, test |
| `Evidence` | Élément qui prouve ou infirme une exigence |
| `Insight` | Contradiction, manque, obsolescence ou risque actionnable |
| `Edge` | Relation entre contexte, système externe et preuve |

## Règle Workspace / Projet

- **Workspace** : frontière stable de connaissance, de propriété et de
  permissions.
- **Project** : initiative bornée avec objectif, état et livrables.
- **ContextNode** : périmètre spécialisé à l’intérieur du projet.

Deux initiatives appartiennent au même workspace si elles partagent durablement
le même domaine, les mêmes personnes autorisées et une source de vérité commune.

## Critères de réussite des usages P0

- aucune reformulation manuelle pendant le handoff de référence ;
- chaque élément du ContextPack expose sa raison d’inclusion ;
- chaque vérité structurante possède une provenance et une version ;
- chaque résultat externe est relié sans être copié intégralement ;
- une contradiction réelle est détectée avec des sources précises ;
- une résolution met à jour la connaissance puis réévalue les projections ;
- l’utilisateur peut expliquer ce qu’un agent savait et pourquoi ;
- la valeur est démontrée sur plusieurs projets et pas uniquement sur un
  scénario seedé.
