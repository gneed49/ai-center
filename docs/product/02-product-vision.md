# AI Center — Vision produit

> Statut : vision consolidée — version 0.2
>
> Date : 24 août 2026
>
> Nom du produit : `AI Center` est un nom de travail.

## Résumé

AI Center est une **couche de pilotage contextuel pour le travail avec l’IA**.
Il redonne à une personne puis à une organisation la maîtrise du contexte
partagé entre ses équipes, ses agents et les outils qu’elle utilise déjà.

AI Center n’est ni un nouvel IDE agentique, ni un agent de code, ni un
remplacement de Linear, Notion, Figma, GitHub ou des bases de données. Ces
systèmes conservent leur rôle et leur profondeur métier. AI Center se place
au-dessus d’eux pour :

- structurer les décisions, règles, exigences et questions ouvertes ;
- compiler le bon contexte pour chaque agent ou outil ;
- formaliser les passages de relais entre disciplines ;
- relier l’intention aux tâches, livrables, artefacts et preuves externes ;
- détecter les contradictions, les manques et la dérive ;
- rendre ce contexte visible, challengeable, versionné et gouvernable.

> **AI Center ne produit pas le travail à la place des outils spécialisés : il
> leur donne un contexte fiable et réintègre leurs résultats dans une source de
> vérité commune.**

## Problème

L’IA devient centrale dans les entreprises, mais son usage reste fragmenté :

- chaque collaborateur travaille dans son propre chat ou agent ;
- les décisions sont dispersées entre conversations, documents et tickets ;
- Produit, Design, Tech et Opérations transmettent le contexte manuellement ;
- les outils externes conservent des fragments de vérité sans vue commune ;
- un agent ignore souvent ce qu’un autre agent, une autre équipe ou un autre
  projet a déjà décidé ;
- les contradictions apparaissent tard, lorsque le code, le design, le ticket
  ou le comportement observé ne correspondent plus à l’intention.

Ajouter un meilleur agent de code ne résout pas ce problème. Remplacer tous les
outils de l’entreprise est irréaliste : les équipes ont déjà leurs pratiques,
leurs historiques, leurs permissions et leurs systèmes de référence.

## Vision

> Permettre à une organisation de travailler avec les meilleurs agents et outils
> disponibles tout en conservant une maîtrise commune, explicite et vérifiable
> du contexte qui guide leur travail.

AI Center devient le plan de contrôle entre :

- les personnes qui prennent et valident les décisions ;
- les agents spécialisés qui analysent, challengent ou proposent ;
- les outils de production qui exécutent réellement le travail ;
- les systèmes de référence qui conservent tickets, documents, code, designs et
  données.

## Promesse

> Je peux changer d’agent, d’outil, de discipline ou de projet sans reconstruire
> le contexte, sans perdre les décisions prises et sans abandonner le contrôle
> de ce qui devient la vérité partagée.

Pour une équipe :

> Nous pouvons utiliser l’IA dans nos outils actuels sans que chacun crée son
> propre contexte isolé. AI Center nous montre ce qui est décidé, ce qui manque,
> ce qui se contredit et ce que chaque agent doit réellement savoir.

## La boucle de contrôle

AI Center porte une boucle continue :

1. **Relier** les sources existantes : documents, tickets, dépôts, designs,
   données et conversations.
2. **Structurer** les éléments contextuels importants en connaissances typées,
   sourcées et versionnées.
3. **Challenger** les décisions, exigences et hypothèses avec les agents
   spécialisés adaptés.
4. **Confirmer** humainement ce qui peut devenir une vérité du projet.
5. **Compiler** un `ContextPack` minimal pour une tâche, un agent et un contrat
   de résultat précis.
6. **Transmettre** ce contexte à l’outil externe qui sait produire le travail.
7. **Réintégrer** les résultats, liens et preuves sans dupliquer inutilement le
   système de référence.
8. **Contrôler** couverture, contradictions, obsolescence et dérive.
9. **Réviser** le graphe puis invalider ou recompiler les projections impactées.

## Ce qu’AI Center possède

AI Center est la source de vérité de la **signification contextuelle** :

- objectifs et intentions ;
- décisions et règles confirmées ;
- exigences, contraintes et critères d’acceptation ;
- questions ouvertes et hypothèses ;
- relations entre connaissances ;
- profils, scopes et contrats des agents ;
- ContextPacks et handoffs ;
- gates et politiques de validation ;
- insights de cohérence ;
- provenance, versions et audit ;
- liens entre intention et preuves externes.

Il peut conserver un snapshot ou un extrait lorsqu’il est nécessaire à la
traçabilité, mais ne doit pas copier un système entier sans raison.

## Ce que les outils existants continuent de posséder

| Système | Source de vérité conservée | Rôle d’AI Center |
| --- | --- | --- |
| GitHub, GitLab | Dépôts, commits, branches, pull requests | Fournir le contexte, relier exigences et preuves de code |
| Codex, Claude Code, Cursor, OpenCode | Production et modification du code | Préparer le contrat, transmettre le ContextPack, réintégrer le résultat |
| Linear, Jira | Cycle de vie opérationnel des tickets | Relier le ticket aux décisions, dépendances et impacts |
| Notion, Confluence, Google Drive | Documents et contenus collaboratifs | Extraire ou référencer les connaissances utiles avec provenance |
| Figma | Designs, composants et prototypes | Relier intention, contraintes, décisions et preuves de design |
| Supabase et autres bases | Schémas, données et opérations | Exposer les contraintes et changements pertinents au contexte |
| Outils métier | Exécution spécialisée et données opérationnelles | Donner aux agents le contexte commun et suivre les preuves |

## Doctrine d’intégration

### Se greffer avant de remplacer

L’adoption ne doit pas exiger une migration préalable. Une équipe peut connecter
un seul projet, une seule source ou un seul workflow et obtenir de la valeur
avant d’élargir le périmètre.

### Référencer avant de dupliquer

AI Center conserve les identifiants, versions, extraits et hashes nécessaires à
la provenance. Le contenu détaillé reste dans son système de référence lorsque
ce dernier est accessible et fiable.

### Contrats plutôt qu’intégrations rigides

Un agent ou un outil externe reçoit :

- un ContextPack ;
- un objectif ;
- des permissions ;
- un contrat de livrable ;
- les preuves attendues.

Il retourne :

- un statut ;
- des artefacts ou leurs références ;
- des preuves ;
- des limites ;
- des propositions de mise à jour contextuelle.

Le cœur reste portable entre fournisseurs.

### Fonctions internes optionnelles

AI Center pourra proposer plus tard des tâches, documents ou automatisations
internes aux nouveaux utilisateurs. Ces fonctions devront rester optionnelles et
ne jamais devenir une condition pour connecter une organisation existante.

## Agents spécialisés

Un agent Produit, Tech, Design ou Opérations n’est pas défini par une nouvelle
interface de chat. Il est défini par :

- son périmètre contextuel ;
- les connaissances qu’il peut lire ou proposer ;
- le contrat de résultat qu’il doit respecter ;
- les outils externes auxquels il peut accéder ;
- les actions nécessitant une validation humaine ;
- la manière dont ses résultats reviennent dans le graphe.

AI Center orchestre ces agents sans chercher à reproduire toutes leurs capacités
spécialisées.

## Utilisateur initial

Le premier utilisateur reste un solo builder, fondateur technique ou lead
Produit–Tech déjà équipé d’agents et d’outils professionnels. Il constitue un
terrain d’apprentissage rapide parce qu’il concentre plusieurs rôles et ressent
directement la perte de contexte.

La cible de valeur longue est l’équipe ou l’entreprise qui :

- utilise plusieurs agents et modèles ;
- possède déjà des outils de tickets, documentation, design, code et données ;
- veut partager un contexte cohérent entre disciplines ;
- refuse le verrouillage dans un seul fournisseur ;
- a besoin de comprendre et gouverner l’usage de l’IA.

## Horizons du produit

### Horizon 1 — Context Control Plane

Prouver la boucle Produit → Tech :

- connaissances structurées et confirmées ;
- ContextPack sélectif et sourcé ;
- handoff sans reformulation ;
- livrables contractuels ;
- contradictions et trous de preuve ;
- liens manuels ou via un premier adapter vers les systèmes externes.

### Horizon 2 — Connected AI Workspace

Connecter les outils les plus structurants :

- GitHub ou GitLab ;
- Linear ou Jira ;
- Notion ou Confluence ;
- Figma ;
- agents et CLI de production.

AI Center synchronise les changements pertinents, enrichit le graphe et détecte
la dérive sans devenir le système de production de ces outils.

### Horizon 3 — Enterprise Context Operating System

Étendre le plan de contrôle à plusieurs équipes et projets :

- gouvernance et permissions ;
- contexte inter-projets ;
- politiques de partage ;
- audit d’entreprise ;
- operating models personnalisables ;
- choix et routage des agents ;
- observabilité de la qualité et de l’impact de l’IA.

Des fonctions natives optionnelles peuvent réduire le nombre d’outils pour les
équipes qui le souhaitent, sans remettre en cause la doctrine d’intégration.

## Principes produit

1. **Le contexte est le produit.** Une génération de texte ou de code ne vaut que
   par le contexte, le contrat et les preuves qui l’entourent.
2. **L’intégration précède la réinvention.** AI Center complète les outils
   existants avant d’envisager une capacité native.
3. **L’humain confirme la vérité.** Un agent propose, challenge et explique ; il
   ne transforme pas silencieusement une inférence en décision.
4. **Le contexte doit être challengeable.** Toute connaissance structurante doit
   exposer sa source, sa version, son statut et ses relations.
5. **La sélection vaut mieux que l’accumulation.** Un ContextPack contient le
   minimum pertinent, pas le dump complet du projet.
6. **Les exécutants sont interchangeables.** Le graphe et les contrats ne
   dépendent pas d’un modèle, d’une CLI ou d’un outil unique.
7. **Les preuves reviennent au contexte.** Un ticket fermé, une PR, un design ou
   un test doit pouvoir confirmer ou invalider ce qui était attendu.
8. **L’adoption est progressive.** Connecter une première source doit déjà créer
   de la valeur.
9. **La gouvernance n’est pas une surveillance opaque.** Le système rend le
   contexte et les décisions visibles aux personnes autorisées.

## Frontières du MVP consolidé

Ne font pas partie du cœur à prouver :

- un IDE ou agent de code propriétaire ;
- un terminal généraliste ou un runner de code interne ;
- un remplacement complet de Linear, Notion, Figma ou GitHub ;
- un gestionnaire de tâches généraliste ;
- un éditeur documentaire complet ;
- une application Android ou un workflow mobile ;
- l’autonomie sans validation ;
- une marketplace de connecteurs ;
- le multi-tenant d’entreprise complet.

Une intégration externe étroite peut être ajoutée pour démontrer la boucle
intention → contexte → outil → preuve. Elle reste un adapter, pas une nouvelle
identité produit.

## Résultat attendu

AI Center réussit son MVP lorsque, sur plusieurs projets réels :

- un utilisateur n’a pas besoin de reformuler le contexte lors d’un handoff ;
- un agent ou outil externe reçoit uniquement les connaissances pertinentes ;
- les sorties reviennent avec leur provenance et leurs preuves ;
- une contradiction réelle ou un manque important est détecté et actionnable ;
- une révision invalide puis recompile correctement le contexte dépendant ;
- l’équipe comprend mieux ce que ses agents savent, pourquoi ils agissent et si
  leur travail respecte toujours l’intention.

## Hypothèses structurantes à valider

- La perte de contexte entre personnes, agents et outils est une douleur assez
  importante pour justifier une couche dédiée.
- Un graphe opérationnel apporte plus de valeur qu’une recherche documentaire
  ou un prompt partagé.
- Les équipes acceptent de confirmer les connaissances structurantes si la
  friction reste faible.
- Un ContextPack sélectif améliore la qualité, la continuité ou le coût par
  rapport à un dump du projet.
- Les insights de cohérence sont suffisamment précis pour créer de la confiance.
- La valeur apparaît avant que tous les outils d’une organisation soient
  connectés.
- Les contrats et preuves rendent les exécutants réellement interchangeables.
