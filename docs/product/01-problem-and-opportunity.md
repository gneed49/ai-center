# AI Center — Problème et opportunité

> Statut : problème consolidé — version 0.2
>
> Date : 24 août 2026
>
> Périmètre : maîtrise du contexte IA entre équipes, agents, projets et outils existants.

## Synthèse

Les entreprises n’ont pas besoin d’un nouvel outil isolé pour produire du code,
des tickets, des documents ou des designs. Elles disposent déjà de GitHub,
Linear, Jira, Notion, Confluence, Figma, de bases de données, de CLI et d’agents
spécialisés.

Le problème apparaît lorsque l’IA devient centrale dans le travail : chaque
personne construit un contexte différent dans ses propres conversations, les
décisions sont recopiées entre les outils, les handoffs perdent leur provenance
et personne ne possède une vue fiable de ce que les agents savent, supposent ou
ont réellement prouvé.

L’opportunité d’AI Center est de devenir la couche de pilotage contextuel qui
redonne à l’entreprise la maîtrise de cette intelligence collective, sans lui
demander de remplacer ses systèmes existants.

## Problème principal

> Comment conserver un contexte d’entreprise partagé, propre, challengeable et
> traçable entre humains, agents, projets et outils, puis fournir à chaque
> spécialiste uniquement ce dont il a besoin sans perdre l’intention commune ?

Ce problème ne se résume pas à retrouver un document ou à enrichir un prompt. La
boucle complète doit permettre de :

- capter les décisions, contraintes, exigences et questions ouvertes ;
- distinguer une proposition d’une connaissance confirmée ;
- relier chaque élément à sa source et à ses versions ;
- compiler le contexte adapté à un rôle, une tâche et un outil ;
- transmettre ce contexte sans copier-coller ni reformulation ;
- rattacher le résultat externe à l’intention qui l’a provoqué ;
- détecter contradictions, dérives et absences de preuve ;
- permettre à l’humain de challenger, résoudre et auditer le système.

## Situation actuelle

Un projet logiciel moderne peut répartir son travail entre :

- conversations avec des assistants généralistes ;
- agents Produit, Design, Tech ou Opérations ;
- Codex, Claude Code, Cursor, OpenCode ou des CLI internes ;
- GitHub ou GitLab ;
- Linear, Jira ou un autre outil de tickets ;
- Notion, Confluence ou Drive ;
- Figma ;
- Supabase et d’autres bases ou plateformes métier.

Chacun de ces systèmes est utile et souvent déjà profondément intégré aux
habitudes de l’équipe. Aucun ne possède toutefois à lui seul la continuité
décisionnelle du projet et la façon dont elle doit être distribuée aux agents.

## Douleurs observées ou à valider

### 1. Le contexte est individuel au lieu d’être organisationnel

Chaque collaborateur explique le projet à son agent dans son propre chat. Les
itérations utiles restent locales, difficiles à partager et rarement
réutilisables par le reste de l’équipe.

### 2. Les handoffs reconstruisent l’histoire

Produit transmet une spécification à Tech, Tech la reformule pour un agent de
code, puis les résultats sont résumés dans un ticket ou une PR. À chaque étape,
des nuances, contraintes et décisions se perdent.

### 3. Les sources de vérité sont multiples mais non reliées

Le ticket, la maquette, le document, le commit et le résultat de test peuvent
tous être corrects localement sans former une chaîne traçable de l’intention à
la preuve.

### 4. Les contradictions apparaissent trop tard

Une décision Produit peut contredire une règle Tech ou un comportement existant
sans être détectée avant l’implémentation, la revue ou la production.

### 5. Le volume n’est pas le bon contexte

Copier davantage de documents dans un prompt augmente le bruit, les coûts et les
risques de fuite sans garantir que l’agent reçoive les éléments applicables à sa
tâche.

### 6. L’entreprise perd la maîtrise de l’usage de l’IA

Sans couche commune, il est difficile de savoir quelles sources ont été
utilisées, quelles décisions ont été prises, quelles permissions ont été
accordées et pourquoi un résultat existe.

### 7. Changer d’outil détruit la continuité

Lorsque la connaissance appartient à un fournisseur ou à un transcript, changer
de modèle, d’agent ou de système oblige à reconstruire une partie du contexte.

## Jobs to be done

### Travail de maîtrise

> Lorsque plusieurs personnes et agents travaillent sur un projet, je veux un
> contexte commun dont les sources, décisions et versions sont explicites, afin
> que l’entreprise reste maîtresse de ce que l’IA utilise.

### Travail de continuité

> Lorsque je passe de Produit à Tech ou d’un agent à un autre, je veux transmettre
> un contexte adapté sans réexpliquer le projet, afin d’éviter les pertes et les
> divergences.

### Travail d’intégration

> Lorsque mon équipe utilise déjà GitHub, Linear, Notion, Figma ou d’autres
> systèmes, je veux les relier au contexte IA sans migrer ni dupliquer leurs
> objets canoniques.

### Travail de contrôle

> Lorsqu’une décision change ou qu’un outil externe produit un résultat, je veux
> voir les impacts, contradictions et preuves manquantes, afin d’intervenir avant
> que le projet ne dérive.

### Travail de confiance

> Lorsque du contexte est transmis à un agent ou à un outil, je veux savoir
> exactement quelles informations et permissions il reçoit, afin d’adopter l’IA
> sans abandonner la gouvernance.

## Alternatives actuelles

| Alternative | Ce qu’elle résout | Limite par rapport au besoin |
| --- | --- | --- |
| Chat ou assistant généraliste | Réflexion et production ponctuelle | Contexte local au transcript et faible continuité organisationnelle |
| Agent de code / IDE agentique | Production de code dans un dépôt | Ne possède pas tout le contexte Produit, Design et entreprise |
| Linear / Jira | Tâches, statuts et coordination | Trace imparfaite des décisions et interactions agentiques |
| Notion / Confluence / Drive | Documentation et recherche | Documents peu opérationnels pour les handoffs et le contrôle continu |
| GitHub / GitLab | Code, PR, CI et collaboration | Arrive tard dans la chaîne intention → preuve |
| Figma | Source de vérité du design | Ne relie pas seul décisions, exigences et implémentation |
| Recherche d’entreprise / RAG | Retrouver des informations multi-sources | Recherche des contenus sans modéliser nécessairement décisions, gates et impacts |
| Suite intégrée nouvelle | Réduit le nombre d’outils | Exige une migration coûteuse et reproduit des fonctions déjà matures |

AI Center ne cherche pas à éliminer ces alternatives. Il les relie en possédant
la continuité contextuelle qu’aucune ne peut fournir seule.

## Opportunité produit immédiate

Prouver une boucle étroite et réelle sur un projet logiciel :

1. cadrer une intention dans le scope Produit ;
2. transformer la conversation en connaissances confirmées ;
3. compiler un `ContextPack` pour la Tech ;
4. effectuer le handoff sans reformulation ;
5. transmettre le contexte à un outil spécialisé existant ;
6. rattacher une preuve réelle produite dans cet outil ;
7. mesurer la couverture et détecter une contradiction ;
8. résoudre ou accepter l’écart avec une trace durable.

La preuve de valeur ne dépend pas d’AI Center produisant du code. Elle dépend de
sa capacité à préserver l’intention et la cohérence d’un bout à l’autre.

## Opportunité stratégique

Si la première boucle fonctionne, AI Center peut devenir la couche de contexte
partagée de l’entreprise :

- plusieurs projets reliés sans graphe monolithique ;
- agents spécialisés alimentés par le même contexte canonique ;
- connecteurs vers les systèmes déjà adoptés ;
- politiques de permissions et de partage ;
- décisions et contradictions transverses ;
- métriques sur la qualité des handoffs et des résultats ;
- fonctions natives optionnelles pour les équipes sans système existant.

Le produit gagne alors de la valeur à mesure que les outils et agents se
multiplient : leur diversité ne fragmente plus le contexte, car AI Center assure
la continuité entre eux.

## Avantage potentiel

L’avantage durable peut émerger de la combinaison suivante :

- une ontologie opérationnelle reliant intentions, décisions, exigences,
  tâches, artefacts et preuves ;
- un Context Compiler qui sélectionne un contexte minimal, sourcé et adapté ;
- une boucle de feedback sur les contradictions, faux positifs et résolutions ;
- une traçabilité multi-outils indépendante des fournisseurs ;
- des operating models réutilisables par type de projet ;
- une expérience où l’humain contrôle les mutations importantes sans maintenir
  manuellement le graphe.

Les données et systèmes canoniques restent la propriété des utilisateurs. Le
moat vient de la qualité du pilotage, pas de l’enfermement.

## Risques et contre-hypothèses

### Le problème peut rester trop abstrait

« Gérer le contexte » ne suffit pas comme promesse. Le MVP doit démontrer des
résultats concrets : moins de reformulation, meilleur handoff, contradiction
utile et preuve retrouvable.

### La structuration peut devenir une charge

Si l’utilisateur doit classer chaque phrase ou dessiner le graphe, l’adoption
échouera. L’IA doit proposer ; l’humain confirme les mutations importantes.

### Les connecteurs peuvent devenir le produit

Multiplier les intégrations avant de prouver le cœur créerait une plateforme
large et fragile. Un seul connecteur étroit suffit pour l’alpha.

### Les systèmes établis peuvent étendre leur propre contexte

Atlassian, Notion, GitHub ou les fournisseurs d’agents peuvent enrichir leurs
graphes et workflows. AI Center doit exceller dans la continuité transverse et
rester portable entre fournisseurs.

### La proactivité peut produire du bruit

Une alerte sans sources, impact ni action détruit la confiance. La précision et
la capacité de résolution priment sur le nombre d’insights.

### La copie des données peut créer un risque de sécurité

Le produit doit minimiser le contexte transmis, préférer les références et
exposer clairement fraîcheur, provenance, permissions et rétention.

## Hypothèses de valeur à tester en priorité

1. Un `ContextPack` réduit réellement la reformulation lors d’un handoff.
2. Le contexte sélectionné est jugé plus pertinent qu’un transcript ou un dump
   documentaire.
3. Une contradiction générique et sourcée provoque une correction utile.
4. Une preuve externe peut être rattachée sans dupliquer son objet canonique.
5. L’utilisateur comprend ce que l’agent a reçu et pourquoi.
6. La boucle reste utile sur plusieurs projets réels et des données non seedées.
7. Les équipes préfèrent connecter leur stack plutôt que migrer vers une suite
   intégrée.

## Signaux de validation proposés

- baisse mesurée du nombre de reformulations Produit → Tech ;
- temps plus court pour reprendre une session ou un projet ;
- ContextPack jugé pertinent et sans données superflues ;
- contradictions utiles nettement plus nombreuses que les faux warnings ;
- lien retrouvable entre décision, livrable externe et preuve ;
- capacité à expliquer quel outil détient l’objet canonique ;
- adoption possible sans remplacer GitHub, Linear, Notion ou Figma ;
- réutilisation du même contexte par plusieurs agents ou rôles.

## Frontières de la consolidation

Ne font pas partie de la consolidation actuelle :

- un IDE ou un terminal dans AI Center ;
- un runner de code détenu par AI Center ;
- le remplacement complet d’un outil de tickets, documents, design ou code ;
- un catalogue large de connecteurs ;
- la collaboration entreprise complète ;
- Android, iOS et tout test mobile.

La prochaine étape est de solidifier la boucle contextuelle actuelle, la tester
sur de vraies données, puis de la fermer avec un premier connecteur externe
minimal.
