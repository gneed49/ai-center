# AI Center — travail partagé et contexte d'entreprise

> Périmètre accepté le 21 septembre 2026 ; qualification du logiciel et des
> services suivie séparément dans les tickets Company Context V1.

AI Center est l'espace web commun où les équipes préparent leur travail avec
l'IA, conservent les décisions et transmettent le contexte entre métiers.
Notion, Linear, GitHub et les outils de développement conservent leurs fonctions.
Les artefacts peuvent être conservés dans AI Center ou publiés explicitement
vers une destination configurée ; leur état et leurs sources restent traçables.

Cette évolution remplace les limites mono-utilisateur, Produit/Tech uniquement,
graphe par projet isolé et connecteur unique du [périmètre précédent](04-mvp-scope.md).
Elle applique [l'ADR 0007](../decisions/0007-company-context-graph.md).
Les fichiers historiques de sources restent inchangés.

## Société et équipe

Une société correspond à une frontière de données et de membres. Elle possède
un espace général, des projets et des propriétaires, éditeurs et lecteurs.
Les projets V1 sont partagés avec les membres de la société selon leur rôle.
Un client indépendant doit avoir sa propre société ; une sélection de projet
ne constitue pas une permission privée par équipe.

L'ouverture est privée : un premier propriétaire est autorisé par l'opérateur,
les collègues rejoignent par invitation. Un simple compte authentifié ne donne
pas le droit de créer une société utilisant les ressources IA de l'instance.

## Travail par projet

La personne choisit un profil Généraliste, PM, Commercial, Lead technique ou
Développeur et retrouve des conversations attachées au bon périmètre. Les
profils métier ne sont pas des rôles d'autorisation. Le passage vers la partie
technique conserve le contexte sourcé requis par le relais.

Dans une conversation, l’agent reçoit une fenêtre récente des échanges de cette
session, avec leurs auteurs et leur provenance, afin de poursuivre le
brainstorming. Cette mémoire de travail est séparée des connaissances confirmées.
Les omissions et extraits d’une longue conversation sont explicites ; l’agent
ne prétend pas se souvenir d’échanges qui n’ont pas été transmis. Les messages
arrivés après la capture d’un tour ne changent pas les données de ce tour.

Le PM prépare kickoff, spécification et tickets produit ; le lead prépare plan
et tickets techniques. Les versions des artefacts sont organisées par projet,
consultables et exportables. Une conversation peut être une source de travail,
mais ses propositions ne deviennent des connaissances confirmées qu'après
validation. Les outils externes réalisent et exécutent le code.

## Graphe et contexte

Chaque projet possède un graphe issu des objets et relations persistés. La vue
société fédère ces graphes et les connaissances générales dans la même frontière
d'accès. Les relations portent provenance, versions et liens vers les objets.
Une vue en liste accompagne la visualisation graphique.

Le contexte d'une question est sélectionné dans les sources autorisées avec
un budget explicite. Les règles obligatoires générales sont conservées ; si
elles dépassent le budget, la génération est refusée avec une explication.
Une sélection partielle n'est jamais présentée comme une lecture exhaustive.
Les packs techniques gardent leurs versions et sources ; une modification
indépendante ne les périme pas, une modification de leurs dépendances oui.

## Outils et cohérence

Les destinations V1 exposent des capacités précises : page Notion, issue Linear,
issue GitHub et lecture GitHub ciblée. Elles ne constituent pas une synchronisation
exhaustive des comptes. La publication est une action humaine sur une version
validée ; une réponse perdue impose une vérification avant toute recréation.

Le steward traite en arrière-plan les changements de connaissances, d'artefacts
et d'observations. Il peut signaler une contradiction entre sources, un contexte
manquant ou un écart à examiner. Les citations identifient les versions réellement
observées. Des métadonnées ou un fichier absent ne permettent pas de conclure
à un comportement du code. La personne rejette, accepte ou résout le constat.

## Limites d'ouverture

La V1 vise d'abord une équipe et ses premiers clients invités. Les preuves du
logiciel, des comptes réels, de l'exploitation et du pilote sont quatre étapes
distinctes. Les [tickets](../../specs/company-context-v1/tickets.md) et
[le statut](../../PROJECT_STATUS.md) indiquent l'état effectivement qualifié.
Le [guide opérateur](../operations/company-web-v1.md) décrit l'ouverture privée,
les quotas et l'arrêt, les secrets, la sauvegarde et les preuves encore requises.


## Retrouver les sources au-delà du graphe

La page Connaissances recherche les titres et textes de toute la société ou
un projet, par pages de 25. Les versions courantes sont présentées par défaut ;
les versions historiques et projets archivés restent consultables. Chaque
résultat ouvre sa version exacte. La recherche textuelle n'est pas une promesse
de recherche sémantique exhaustive.

Les nœuds du graphe ouvrent les conversations, connaissances, contextes transmis,
livrables, tâches et fichiers observés. Pour un fichier, le lien conserve
l'observation et le fichier sélectionnés ; une identité absente est signalée
sans substituer un contenu différent. Lire une observation ne relance pas GitHub.
La création d'un projet décrit les cinq agents ; le parcours Produit vers Lead
ou Développement présente validation et transmission en langage métier.


### Brouillons préparés par les agents

La bibliothèque permet de demander explicitement une note de lancement, une
spécification ou des tickets produit aux agents généraliste, produit et commercial.
Le lead technique et le développeur préparent plans et tickets techniques depuis
un contexte transmis courant. Le document conserve ses sections, tickets,
citations et points à clarifier ; l’édition de ces champs met à jour le même
contenu Markdown transmis aux outils. Le résultat reste un brouillon jusqu’à une
validation humaine, et la publication externe nécessite une action distincte.

Un livrable FeatureBrief ou TechnicalPlan existant peut être copié depuis sa
page dans un brouillon publiable. La version choisie, y compris historique,
reste la source exacte : aucune substitution par le livrable le plus récent.

Une demande de génération envoyée reste retrouvable dans le même navigateur et
le même compte/société/projet. Le suivi lit un reçu serveur sans nouvel appel IA.
Une reprise est explicite et conserve son identité ; une demande expirée invite
à consulter les résultats existants avant de préparer une nouvelle demande.


### Tickets distincts dans les outils

Une version validée de tickets produit ou techniques permet de sélectionner les
entrées à publier. Un aperçu montre le contenu et la destination ; chaque entrée
confirmée devient une issue distincte dans Linear ou GitHub, avec son état et
son lien. Notion reçoit toujours une page pour le document complet. Les tickets
existants de cette version sont réutilisés ; une nouvelle version demande une
confirmation supplémentaire, sans modifier automatiquement les anciennes issues.

Un résultat incertain est vérifié individuellement avant toute autre action.
La reprise d’une commande lit son reçu sans envoyer de nouvelles créations.
Le graphe, l’historique et les exports conservent la version et le numéro exacts
de chaque entrée. L’interface permet de lire et modifier les entrées déjà
présentes ; l’ajout et le retrait d’entrées dans cet éditeur restent à compléter.
