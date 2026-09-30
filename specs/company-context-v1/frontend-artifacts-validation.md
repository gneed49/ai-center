# Livrables web — preuves locales du 21 septembre 2026

Lot CC-T05 réalisé dans le mandat autonome du 21 septembre, selon la
spécification active `specs/company-context-v1/spec.md` et sa planification.

## Parcours livrés

- Bibliothèque société `/artifacts` et projet `/projects/:id/artifacts`,
  recherche, filtres de type/état et pagination de 25 documents.
- Création manuelle de cinq types : kickoff, spécification, tickets produit,
  plan technique et tickets techniques. Depuis une conversation, l'action
  « Préparer un livrable » rattache son origine ; reprendre le dernier texte
  de l'agent nécessite une action explicite.
- Édition créant une nouvelle version, validation par identité de version,
  historique paginé, ouverture directe d'une version et comparaison de deux
  versions exactes. Le contenu et les sources restent visibles pour chaque
  version. Les données structurées existantes sont préservées.
- Sources sélectionnées depuis les versions de connaissances, livrables
  existants, pack transmis et conversation d'origine. Les entrées envoyées
  au serveur contiennent seulement `kind` et `public_id`, jamais les champs
  descriptifs des snapshots de lecture.
- Export authentifié canonique en Markdown/JSON ou copie Markdown pour la
  version consultée, y compris une version historique. Identité/version et
  provenance restent celles du serveur.
- Destinations société/type et overrides projet, origine du réglage visible,
  contrôle de révision, retour au réglage hérité. Ces réglages seuls n'envoient
  rien et n'affichent aucun succès de publication.
- Navigation et action conversation reliées aux nouvelles pages. Les anciens
  livrables d'exécution restent accessibles dans « Plans et preuves ».

## Fiabilité et droits

La même tentative de création, édition, validation ou destination conserve
sa commande lors d'un échec ambigu. Une révision en cours conserve sa version
de départ malgré une actualisation concurrente ; le serveur arbitre le
conflit au lieu de perdre silencieusement le contenu. Les lecteurs n'ont pas
d'action d'édition ; seul le propriétaire peut modifier les destinations
société. Un projet inaccessible ne retombe jamais sur les données société.

Les pages d'artefacts sont chargées à la demande. Aucun nouveau secret,
connecteur réel, déploiement ou publication externe n'a été utilisé.

## Validation

- Suite web collective : **90 tests réussis dans 21 fichiers** à ce point.
- 13 nouveaux tests portent sur API artefacts, sources de conversation,
  identités de commande, concurrence/version initiale, historique/export
  exacts, permissions et héritage/révision des destinations.
- Un test supplémentaire couvre le relais Développement : `node_key=dev`,
  pack ciblé via `target_node_key=dev` ; `tech` reste le défaut.
- Lint web sans avertissement ; compilation TypeScript et Vite réussies.
  Bundle principal mesuré à environ 276,83 kB minifié / 85,07 kB gzip.
- Contrôle de différences sans erreur.
- Le débordement horizontal de conversation lié aux nouveaux espacements
  du shell a été corrigé. Test navigateur ciblé
  `accessibility-desktop.spec.ts` : **1 réussi**, à 1280/1440/1920 px avec
  audit axe et vérification des tabulations (fixtures synthétiques).

## Limites exactes

Les tests UI du lot artefacts sont locaux sous jsdom, avec fixtures
`[FICTIF]`. La recette navigateur intégrée du nouveau parcours est prise en
charge par le lot global ; elle n'est pas revendiquée comme acquise ici.
La comparaison montre les deux textes exacts et surligne les lignes
uniquement présentes d'un côté ; elle ne prétend pas calculer un diff
sémantique ni identifier parfaitement les déplacements ou répétitions.
Le texte Markdown est affiché lisiblement sans interprétation HTML.
La provenance conversation est un instantané d'origine (identité, scope,
date et bornes de messages) ; elle ne fait pas du transcript une connaissance
confirmée. La publication dans un outil externe appartient aux lots CC-T06/07.
