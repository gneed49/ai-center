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
