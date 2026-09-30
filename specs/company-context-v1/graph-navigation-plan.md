# Graphe : compléter les objets navigables — CC-031 / CC-033

Constat de la revue du 22 septembre : le graphe projette les espaces, agents,
connaissances, livrables, sources externes et alertes, mais n’expose pas encore
les conversations et tâches comme des nœuds sélectionnables. Certaines sources
internes n’ont pas de page de consultation exacte. Les tests simulés utilisaient
une URL Notion et ne prouvaient donc pas la navigation vers un document interne.

Première correction réalisée : numéro de version explicite, état historique
des anciennes versions de documents et lien interne vers la version exacte du
document, du livrable ou de l’alerte. La recette réelle crée quatre versions et
ouvre celle choisie depuis le graphe. Les liens externes HTTPS et les
chemins applicatifs autorisés restent deux contrats distincts.

Le complément ci-dessous est autorisé par le mandat autonome et est implémenté et vérifié localement le 22 septembre. Il ne crée pas un gestionnaire de tâches concurrent de Linear.

1. Projeter `sessions` et les tâches déjà canoniques, avec leur projet, état et
   provenance. Les relations conversation → agent, conversation → pack,
   artefact → conversation d’origine et tâche → pack doivent provenir des
   associations persistées. Ne pas inventer une relation par proximité visuelle.
2. Permettre la consultation exacte d’une version de connaissance et d’un pack
   depuis le graphe, y compris une version ancienne citée. Une vue en lecture
   seule expose titre, contenu, statut, scope, version et provenance ; aucune
   substitution silencieuse par la dernière version.
3. Les tâches renvoient vers les sources et sorties existantes. Ne pas ajouter
   d’exécution de code ou de synchronisation distante automatique.
4. Si la création de relations manuelles inclut les conversations, étendre la
   validation SQL des extrémités et ses grants dans une migration additive.
   Maintenir le refus inter-sociétés et l’inventaire d’effacement. Une extrémité
   projet/UUID forgée reste refusée.
5. Projeter ces types dans les filtres, les listes accessibles et les cartes.
   Les mêmes libellés distinguent les versions dans les sélecteurs de relations.
   Les limites et omissions restent affichées ; les graphes volumineux ne
   promettent pas une vue exhaustive.

Preuves avant fermeture : API/DB sur deux sociétés, sources historiques,
conversations de deux projets et tâche avec pack ; navigateur depuis le graphe
jusqu’à chaque objet, retour au projet, clavier et affichage compact. La réussite
des seuls tests des livrables ne clôt pas cette exigence plus large.


Recette du 22 septembre : unité web 154/154, Clippy, migration additive 13,
intégration société 20/20 dont sources historiques, tâche/pack, viewer et deux
sociétés ; deux parcours navigateur réels passent, dont source connaissance,
pack et conversation depuis le graphe. Restent dans CC-031/033 : navigation
précise des fichiers observés et complément navigateur compact/clavier/tâche.

Complément CC-031 : chaque nœud de fichier GitHub doit ouvrir l'observation
immuable et le fichier exact, avec leur UUID. Un fichier absent de l'observation
affiche une erreur explicite, jamais le premier fichier à sa place. Le lien
reste interne, limité aux routes autorisées ; aucun téléchargement ou appel
GitHub n'est déclenché à la lecture. Preuves : sélection du second fichier,
refus de substitution, lien issu du vrai graphe en PostgreSQL pour un lecteur.
