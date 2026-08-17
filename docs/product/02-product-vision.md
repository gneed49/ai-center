# AI Center — Vision produit

> Statut : brouillon de travail — version 0.1  
> Date : 8 août 2026  
> Nom du produit : `AI Center` est un nom de travail.

## Résumé

AI Center est un centre de commandement mobile permettant de confier, suivre, interrompre et valider du travail réalisé par des agents IA sur une machine distante sécurisée.

La première version vise un usage personnel de développement logiciel : depuis une application Android, l'utilisateur pilote un runtime installé sur son Mac, capable d'utiliser des agents de code, le terminal, les fichiers et, à terme, le navigateur. Le produit doit offrir une continuité de travail suffisamment fiable et lisible pour permettre de quitter physiquement son poste sans abandonner son environnement de développement.

À plus long terme, AI Center a vocation à devenir un système d'exploitation de projet centré sur l'IA : chaque projet rassemble ses sources, décisions, livrables et contextes dans une structure explicite, exploitable par des agents spécialisés et, ultérieurement, par un graphe de connaissances.

## Vision

Permettre à une personne puis à une organisation de piloter un travail IA réel, contextualisé et vérifiable depuis n'importe quel appareil, sans perdre le contrôle de ses machines, de ses données ni de ses décisions.

## Promesse initiale

> Je peux quitter mon bureau et continuer depuis mon téléphone à confier, superviser et valider des tâches de développement exécutées sur mon Mac.

Cette promesse implique davantage qu'une interface de chat. L'utilisateur doit pouvoir comprendre ce qui se passe, intervenir au bon moment et retrouver une session dans un état cohérent après une interruption ou un changement d'appareil.

## Conviction fondatrice

Le chat est une porte d'entrée, pas le produit.

La valeur d'AI Center vient de la boucle complète :

1. exprimer une intention ;
2. la rattacher au bon projet et au bon contexte ;
3. déléguer le travail à un agent ou un outil adapté ;
4. observer l'exécution en temps réel ;
5. approuver les actions qui nécessitent une décision humaine ;
6. inspecter les changements et le résultat ;
7. reprendre ou poursuivre le travail sans rupture de contexte.

## Les trois horizons du produit

### Horizon 1 — Remote Agent Cockpit

Un utilisateur, une application Android, un Mac et une sélection de projets locaux autorisés.

L'objectif est de rendre le développement assisté par IA réellement mobile : démarrer ou reprendre une session, déléguer une tâche, suivre l'activité de l'agent, répondre à ses demandes, contrôler les actions sensibles et consulter le résultat.

Cet horizon constitue le terrain de validation du produit. Il doit être utile quotidiennement avant toute extension vers une plateforme d'entreprise.

### Horizon 2 — AI Workspace

Le produit structure le travail au-delà d'une succession de conversations : projets, objectifs, tâches, sessions, artefacts, décisions, sources et livrables deviennent des objets de premier rang.

Plusieurs agents et outils peuvent intervenir sur un même projet. AI Center conserve la continuité, expose l'état du travail et facilite le passage d'une tâche à une autre ou d'un appareil à un autre.

### Horizon 3 — AI Project Operating System

Chaque projet dispose d'une architecture contextuelle adaptable, par exemple des espaces Produit, Technique, Utilisateurs ou Opérations. Ces espaces peuvent être issus de modèles, constituer des nœuds d'un graphe de connaissances et être administrés par des agents spécialisés.

Le système relie les sources, décisions, conversations, actions et livrables. Il devient une couche de coordination entre les personnes, les agents, les machines et les outils de l'entreprise, avec des politiques d'accès, un historique vérifiable et une gouvernance explicite.

Les horizons décrivent une trajectoire, pas trois produits à développer simultanément.

## Utilisateur initial

Le premier utilisateur est un développeur expérimenté qui utilise déjà des agents de code et possède une machine de travail capable d'exécuter les outils localement. Il veut avancer loin de son bureau sans transférer tout son environnement de développement sur son téléphone ou dans une infrastructure cloud supplémentaire.

Les personas détaillés et leur ordre de priorité seront définis dans `02-personas-and-use-cases.md`.

## Principes produit

### 1. Mobile-first, sans appauvrissement critique

L'interface mobile ne cherche pas à reproduire un IDE sur un petit écran. Elle optimise les décisions qui ont de la valeur en mobilité : donner une intention, répondre, suivre, approuver, interrompre et vérifier.

### 2. Local-first pour l'exécution

Le runtime s'exécute sur une machine contrôlée par l'utilisateur. Il exploite l'environnement, les dépôts et les outils déjà présents, dans des périmètres explicitement autorisés.

`Local-first` ne signifie pas nécessairement `local-only` : une couche de relais ou de synchronisation pourra être introduite si elle améliore la disponibilité sans affaiblir le modèle de confiance.

### 3. L'humain conserve l'autorité

L'agent peut proposer et exécuter dans les limites définies. Les actions sensibles ou irréversibles doivent être visibles, explicables et soumises à une politique d'autorisation adaptée.

### 4. L'exécution est observable

L'utilisateur doit distinguer l'intention, le plan, l'action en cours, les commandes exécutées, les fichiers modifiés, les blocages et le résultat. Une animation « l'IA réfléchit » ne constitue pas une supervision.

### 5. La continuité prime sur la conversation

Une session doit survivre à la fermeture de l'application, aux pertes temporaires de réseau et aux changements d'appareil. Le système possède un état durable ; le flux temps réel n'en est qu'une projection.

### 6. Le projet devient l'unité de contexte

Les conversations et exécutions appartiennent à un projet. Progressivement, ce projet devient une structure vivante de sources, décisions, actions, connaissances et livrables, et non un simple dossier ou une longue fenêtre de chat.

### 7. L'intégration avant la réinvention

AI Center doit pouvoir orchestrer des agents de code et des outils existants. Sa valeur initiale réside dans le contrôle, la continuité et l'expérience mobile, pas dans la création prématurée d'un nouveau modèle ou d'un nouvel IDE.

## Expérience cible initiale

Depuis son téléphone, l'utilisateur :

- se connecte de manière sécurisée à son Mac ;
- choisit un projet local autorisé ;
- crée une session ou reprend une session existante ;
- décrit une tâche par texte ou par voix ;
- voit l'agent analyser, planifier et exécuter ;
- reçoit une demande claire lorsqu'une décision ou une autorisation est nécessaire ;
- peut interrompre l'exécution ;
- consulte les fichiers modifiés, le diff, les tests et une synthèse ;
- poursuit la conversation ou lance la prochaine action.

## Frontières actuelles

Ne font pas partie de la première preuve produit :

- la gestion d'une organisation multi-équipe ;
- un graphe de connaissances complet ;
- une marketplace d'agents ;
- un remplacement généraliste de ChatGPT, Claude ou d'un IDE ;
- l'autonomie sans limite ni validation ;
- l'exécution arbitraire sur l'ensemble du système ;
- la couverture simultanée d'Android, iOS, web et desktop ;
- la conception d'un moteur d'agent propriétaire si un agent existant permet de tester la promesse.

Ces exclusions protègent le premier apprentissage ; elles ne renient pas la vision longue.

## Résultat attendu

AI Center réussit sa première étape lorsqu'il devient naturel et fiable de lancer ou de poursuivre depuis un téléphone une vraie tâche de développement sur le Mac, sans devoir ouvrir une session de bureau à distance et sans perdre la compréhension ni le contrôle du travail effectué.

À terme, le produit réussit lorsqu'une équipe peut retrouver dans un même système le contexte de ses projets, les décisions prises, les actions proposées, les travaux exécutés et les livrables produits — avec une collaboration fluide entre humains et agents.

## Hypothèses structurantes à valider

- Le besoin de mobilité concerne assez de tâches de supervision et de décision pour justifier une application dédiée.
- Un cockpit mobile est plus efficace qu'un bureau distant ou qu'un simple accès terminal pour ces tâches.
- Les utilisateurs acceptent qu'une machine personnelle ou d'entreprise joue le rôle de runtime disponible à distance.
- L'accès via un réseau privé tel que Tailscale est une base acceptable pour la première version.
- Les agents de code existants peuvent être intégrés derrière une abstraction suffisamment stable.
- La voix améliore réellement l'expression des tâches en mobilité, mais n'a pas nécessairement besoin d'être complète dans la première tranche du MVP.
- Le travail effectué dans le cockpit produit naturellement les événements et artefacts qui alimenteront plus tard le système contextuel.

## Questions encore ouvertes

- Quel scénario d'usage doit constituer le « moment magique » de la première démonstration ?
- Quelle profondeur d'interaction mobile est nécessaire avant que le produit soit préférable à une solution existante ?
- Quel agent de code doit être intégré en premier ?
- Quelle partie du contrôle doit être assurée directement par le runtime et quelle partie nécessite un service intermédiaire ?
- Quel modèle d'autorisation offre un bon équilibre entre fluidité et sécurité ?
- La voix appartient-elle au premier MVP utilisable ou à l'itération suivante ?
- La cible initiale doit-elle rester strictement personnelle ou anticiper dès l'architecture un petit nombre de collaborateurs ?

