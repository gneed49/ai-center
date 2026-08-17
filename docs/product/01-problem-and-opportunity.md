# AI Center — Problème et opportunité

> Statut : brouillon de travail — version 0.1  
> Date : 8 août 2026  
> Périmètre : le problème initial de mobilité et de supervision ; la problématique contextuelle de long terme est décrite séparément afin de ne pas diluer la validation du MVP.

## Synthèse

Les agents IA savent désormais effectuer une part croissante du travail de développement : explorer un dépôt, modifier des fichiers, lancer des commandes, utiliser des outils et demander des arbitrages. Pourtant, leur utilisation reste fortement liée au poste de travail et à des interfaces conçues pour une présence continue devant l'écran.

Lorsqu'un développeur s'éloigne de son ordinateur, il perd soit la capacité d'agir, soit la richesse d'interaction, soit la visibilité et le contrôle nécessaires pour déléguer du vrai travail. Les solutions disponibles couvrent chacune une partie du besoin — chat mobile, bureau distant, terminal mobile, agent cloud ou dispatch vers un ordinateur — sans fournir une boucle mobile complète, cohérente et extensible.

L'opportunité d'AI Center est de transformer le téléphone en cockpit de supervision d'un runtime IA situé sur la machine de travail, puis d'utiliser cette première boucle opérationnelle comme fondation d'un système de gestion de projet et de contexte plus profond.

## Situation actuelle

Un développeur utilise couramment plusieurs surfaces :

- un IDE et un terminal sur sa machine ;
- un ou plusieurs agents de code en CLI ou dans une application desktop ;
- une application mobile généraliste pour réfléchir ou discuter ;
- GitHub et d'autres outils de projet pour inspecter les résultats ;
- parfois un bureau distant pour retrouver l'ensemble de l'environnement.

Le passage d'une surface à l'autre fracture le travail. Le contexte doit être reformulé, les sessions ne se poursuivent pas toujours, les actions nécessitent de revenir devant la machine et le téléphone offre rarement la même capacité d'exécution ou de supervision.

## Problème principal

> Comment continuer à faire avancer un projet logiciel depuis un téléphone, en exploitant la puissance et l'environnement réel de sa machine de travail, tout en gardant une compréhension claire et un contrôle explicite des actions de l'agent ?

Le problème ne se résume pas à envoyer un message à distance. Pour qu'une délégation soit utile, la boucle entière doit fonctionner :

- atteindre le bon projet et la bonne session ;
- transmettre une intention suffisamment riche ;
- exécuter avec les bons outils et permissions ;
- diffuser l'état du travail malgré un réseau mobile instable ;
- solliciter l'utilisateur quand une décision est nécessaire ;
- permettre l'arrêt ou la reprise ;
- présenter les changements sous une forme lisible sur mobile ;
- conserver une trace durable et exploitable.

## Douleurs observées ou fortement présumées

### 1. Le travail IA reste attaché au bureau

Un agent peut fonctionner de façon semi-autonome, mais l'utilisateur doit fréquemment rester à proximité pour répondre à une question, autoriser une commande, corriger une direction ou inspecter le résultat. Une autonomie théorique de vingt minutes peut donc exiger une présence humaine discontinue de vingt minutes.

### 2. Les interfaces mobiles sont souvent des versions réduites

Elles permettent de converser, parfois de déclencher une tâche distante, mais n'exposent pas toujours les fonctions nécessaires à un travail sérieux : sélection du projet, suivi structuré, terminal ou événements, demandes d'autorisation, diff, tests, interruption et reprise fidèle.

### 3. Le bureau distant transporte l'écran, pas l'intention

Une solution de bureau distant donne accès à tout, mais impose de manipuler une interface desktop depuis un petit écran. Elle est utile comme solution de secours, peu adaptée à des interactions fréquentes et brèves en mobilité, et ne transforme pas l'exécution de l'agent en objets compréhensibles ou actionnables.

### 4. Le terminal mobile est puissant mais peu supervisable

SSH ou une application terminal peuvent lancer des outils, mais la lecture de flux longs, l'inspection de modifications, la gestion de plusieurs sessions et les validations deviennent vite inconfortables. L'utilisateur doit connaître la mécanique technique plutôt que piloter le résultat attendu.

### 5. Le contexte se fragmente entre les outils

Le chat, le terminal, le dépôt, les décisions et les livrables vivent dans des systèmes différents. Même quand une session est accessible, elle ne s'inscrit pas toujours dans une continuité de projet structurée.

### 6. La confiance chute lorsque l'exécution est opaque

Une promesse d'autonomie ne suffit pas. Sans visibilité sur le plan, les actions, les changements et les points de décision, l'utilisateur hésite soit à déléguer, soit à accorder les permissions nécessaires.

## Travaux à accomplir — formulation provisoire

Les travaux à accomplir seront précisés et hiérarchisés avec les personas. À ce stade, les hypothèses principales sont :

### Travail fonctionnel

> Lorsque je ne suis pas devant mon ordinateur, je veux confier ou poursuivre une tâche de développement sur mon vrai environnement, afin que le projet avance sans attendre mon retour au bureau.

### Travail de contrôle

> Lorsque l'agent travaille à distance, je veux voir son état, comprendre ses actions et pouvoir intervenir, afin de déléguer sans abandonner mon autorité.

### Travail de continuité

> Lorsque ma connexion ou mon attention est interrompue, je veux retrouver une session cohérente et son historique, afin de reprendre sans reconstruire le contexte.

### Travail de confiance

> Lorsque l'agent demande une action sensible, je veux recevoir une demande intelligible et proportionnée au risque, afin d'autoriser rapidement ce qui est sûr et de bloquer le reste.

## Alternatives actuelles

| Alternative | Ce qu'elle résout | Limite supposée par rapport au besoin |
| --- | --- | --- |
| Application mobile généraliste d'IA | Conversation, réflexion, parfois tâches distantes | Accès incomplet au runtime, au projet ou aux fonctions de supervision |
| Fonction de dispatch vers un ordinateur | Déclenchement d'un agent local depuis le mobile | Périmètre et profondeur d'interaction variables ; dépendance à un fournisseur |
| Bureau distant | Accès complet à la machine | UX desktop sur petit écran, faible structuration du travail agentique |
| SSH / terminal mobile | Contrôle technique direct | Lecture, validation, diff et multi-session peu adaptés au mobile |
| Agent de code cloud | Exécution indépendante de la machine locale | Environnement, secrets, données, coûts ou fidélité au poste local différents |
| GitHub depuis mobile | Revue des commits, PR et résultats | Intervient surtout après l'exécution et ne couvre pas toute la boucle interactive |
| Automatisations par messagerie | Déclenchement simple depuis n'importe où | Faible observabilité, commandes limitées et modèle d'autorisation rudimentaire |

Ces limites sont des hypothèses à vérifier par une étude concurrentielle ciblée et par des tests réels. Elles ne doivent pas être traitées comme des vérités acquises.

## Pourquoi maintenant

Plusieurs évolutions convergent :

- les agents de code deviennent capables de travailler plus longtemps et d'utiliser davantage d'outils ;
- les CLI et SDK agentiques rendent leur intégration plus accessible ;
- les développeurs acceptent progressivement un mode de travail fondé sur la délégation et la revue ;
- les réseaux privés maillés simplifient l'accès sécurisé à une machine personnelle ;
- le téléphone est déjà la surface naturelle des validations rapides, notifications et interactions vocales ;
- plus les agents gagnent en autonomie, plus le besoin se déplace de la saisie de code vers la supervision, l'arbitrage et la gestion du contexte.

Cette dernière évolution est centrale : une interface mobile devient crédible non pas parce qu'elle sait afficher un IDE miniature, mais parce qu'une part croissante du travail humain consiste à donner une direction et à contrôler un résultat.

## Opportunité produit immédiate

Construire une expérience mobile spécialisée autour d'une boucle courte et fiable :

1. choisir un projet ;
2. confier une tâche ;
3. suivre son exécution sur le Mac ;
4. répondre aux demandes de l'agent ;
5. examiner et valider le résultat.

Le runtime macOS conserve l'accès aux dépôts, outils et configurations existants. L'application Android présente non pas l'écran du Mac, mais une vue sémantique du travail : sessions, événements, actions, autorisations, changements et résultats.

## Opportunité stratégique

Le cockpit génère naturellement une matière structurée : intentions, plans, commandes, décisions, fichiers modifiés, résultats de tests, erreurs, validations et livrables. Ces événements peuvent devenir la base factuelle du futur système contextuel.

La trajectoire peut donc rester cohérente :

- le MVP résout la mobilité et le contrôle ;
- les itérations suivantes structurent les sessions en projets, tâches et artefacts ;
- le système contextuel relie ensuite ces éléments à des sources et à des nœuds spécialisés ;
- des agents spécialisés peuvent enfin gérer ou exploiter les différentes zones du projet.

Le cockpit n'est alors pas un détour : il est le premier point de captation du travail réel.

## Avantage potentiel

L'avantage durable ne viendra probablement ni du chat ni de l'accès distant seuls. Il pourrait émerger de la combinaison suivante :

- une expérience mobile conçue pour la supervision agentique ;
- un runtime local extensible et indépendant d'un unique agent ;
- un protocole d'événements et d'autorisations commun aux outils ;
- une continuité robuste entre appareils et sessions ;
- un modèle de projet qui transforme progressivement l'historique d'exécution en contexte exploitable ;
- une architecture de confiance adaptée aux usages personnels puis aux organisations.

## Risques et contre-hypothèses

### Le besoin peut être fréquent mais peu important

Les utilisateurs peuvent apprécier de vérifier une tâche depuis leur téléphone sans vouloir y initier un travail complexe. Le produit devrait alors privilégier notifications, réponses rapides et revue plutôt qu'un chat exhaustif.

### Les solutions existantes peuvent combler rapidement l'écart

Les principaux fournisseurs d'agents peuvent enrichir leurs applications mobiles et fonctions de dispatch. AI Center doit donc tester une valeur différenciante plus profonde que la seule disponibilité sur Android : interopérabilité, runtime contrôlé, observabilité et structuration de projet.

### Le runtime domestique peut être une contrainte excessive

Une machine éteinte, endormie, hors ligne ou mal configurée dégrade la promesse. La disponibilité, la reconnexion et la récupération doivent être considérées comme des fonctions produit, pas seulement comme des détails d'infrastructure.

### La sécurité peut détruire la fluidité

Trop peu de contrôles rend le produit dangereux ; trop de confirmations le rend inutilisable. Les autorisations devront être contextuelles, regroupables et compréhensibles, avec des valeurs par défaut prudentes.

### Le flux brut est illisible sur mobile

Transposer la sortie d'un terminal ne suffit pas. Le système devra produire des événements structurés et des résumés, tout en permettant d'accéder aux détails lorsque nécessaire.

### Le périmètre long terme peut distraire le MVP

Le graphe de connaissances et l'organisation multi-agent sont prometteurs, mais ils peuvent conduire à construire une infrastructure abstraite avant d'avoir validé un usage quotidien. Ils doivent rester une contrainte d'extensibilité, pas une dépendance du premier produit.

## Hypothèses de valeur à tester en priorité

1. L'utilisateur rencontre chaque semaine plusieurs situations où une tâche pourrait avancer s'il pouvait piloter son agent depuis son téléphone.
2. Les interactions nécessaires sont majoritairement des intentions, réponses, validations et revues — donc compatibles avec une UX mobile spécialisée.
3. Une session persistante et observable apporte nettement plus de valeur qu'un simple déclenchement à distance.
4. Le contrôle des projets locaux et des permissions constitue un bénéfice perçu, et pas seulement une complexité technique.
5. Une première intégration avec un seul agent de code suffit à démontrer la promesse.
6. Le texte permet de valider le cœur du produit avant une intégration vocale complète.

## Signaux de validation proposés

Avant de définir des métriques définitives, les signaux suivants peuvent guider les premiers essais :

- l'utilisateur choisit spontanément AI Center plutôt qu'un bureau distant pour une tâche agentique ;
- au moins une tâche réelle est initiée ou poursuivie loin du Mac plusieurs fois par semaine ;
- une session interrompue est reprise sans reformulation importante ;
- l'utilisateur comprend l'état du travail et sait quand intervenir ;
- les demandes d'autorisation sont traitées rapidement sans devenir irritantes ;
- le résultat peut être évalué sur mobile dans une proportion significative des cas ;
- le système permet effectivement de gagner du temps ou de supprimer une attente jusqu'au retour au bureau.

## Ce que la prochaine étape doit décider

Le travail sur les personas et cas d'usage devra éviter une cible générique de « développeur mobile ». Il devra préciser :

- qui ressent le problème le plus intensément ;
- dans quelles situations concrètes il quitte son poste ;
- quelles tâches sont réellement délégables depuis un téléphone ;
- quel niveau de risque et de complexité il accepte ;
- quelle alternative il utilise aujourd'hui ;
- quel cas d'usage est assez fréquent, douloureux et démontrable pour devenir le cœur du MVP ;
- quels cas doivent être volontairement exclus, même s'ils sont séduisants.

