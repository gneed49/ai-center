# AI Center — Modèle d'architecture (schématique)
_Posé par Geoffrey, affiné le 1 août 2026. Voir CONTEXT.md, VISION.md, MVP.md._

## Métaphore : un graphe de connaissance par projet
Chaque projet = un **graphe**. Chaque nœud = une partie du projet (Produit, UX/Design, Dev/Tech, Sales...). Les nœuds ont des enfants ; chaque nœud produit des **livrables structurés** qui servent de contexte à ses enfants. Les **arêtes** portent les liens et dépendances entre parties.

## Ce qu'on entend par « agent » (important)
Pas des agents autonomes. Un « agent spécialisé » = **le même LLM classique**, à qui on donne un **rôle via un prompt / fichier .md** + des **outils / skills spécifiques**. L'agent dev = LLM + instructions dev. L'agent produit = LLM + instructions produit. C'est une question d'instruction et d'outillage, rien de plus.

## UI : une seule fenêtre, plusieurs cerveaux
Visuellement, **la même fenêtre de chat partout**. Sur le nœud Produit tu parles à l'agent produit ; sur le nœud Tech, à l'agent tech. L'UI ne change pas ; seules les instructions derrière changent selon le nœud courant.

## Pourquoi scoper plusieurs agents plutôt qu'un seul : le contexte
Un agent unique portant toutes les casquettes = **fenêtre de contexte énorme** → compression, perte d'info, saturation rapide. Donc chaque agent est volontairement **« aveugle » à ce qui l'entoure** : il n'a que le contexte de **son nœud et de ses enfants** (+ ce qu'on lui donne explicitement ; il peut aller chercher un autre nœud si on le lui demande).
Analogie entreprise : chaque manager (produit, tech, UX) connaît en profondeur SON périmètre, pas celui des autres. C'est un choix délibéré de **partitionnement du contexte**, pas de la complexité gratuite.

## L'agent global (le « CTO ») — le cœur de la proactivité
Au-dessus, un **agent global** avec une vision **complète mais non détaillée** (comme un CTO : il connaît tout globalement, pas chaque détail). Son rôle : **checker, vérifier, suggérer**. Il capte ce qui se décide et alerte sur les liens : « attention, décision côté produit qui contredit ça ».
Refinement recommandé : plutôt que de faire lire chaque token de chaque conversation à un watcher (coûteux, fragile), chaque agent spécialisé **commite des entrées structurées** (décisions, questions ouvertes) dans le graphe au fil de l'eau ; l'agent global **raisonne sur cette couche condensée** et surveille les contradictions sur les arêtes. Contexte scopé pour les agents ET tâche tractable pour le global.

## Le « routeur » du milieu = deux choses distinctes
- **Routage UI** : quel agent tu adresses selon le nœud courant. Trivial.
- **Routage de connaissance** : retrouver le bon contexte dans le graphe pour une requête (récupération / retrieval sur les nœuds). C'est le vrai sujet. Le « nœud du milieu » est surtout un **index / système de récupération**, pas forcément un agent.

## Séquencement MVP (rappel)
Le graphe + une couche « décisions structurées » + un agent global qui détecte une contradiction entre 2 nœuds = l'atome à prouver en premier. La galerie complète d'agents spécialisés vient ensuite.
