# Steward : progression du contexte société

Statut : implémentation autorisée, validation PostgreSQL à effectuer dans la stack gardée.

## Problème vérifié

CC-040/041 : les 128 premières sources favorisaient le projet courant et les
48 meilleures paires étaient reprises à chaque événement. Les paires déjà
évaluées étaient dédupliquées après l'appel IA. Aucun curseur ne permettait
d'examiner les autres projets quand cette première fenêtre était pleine.

## Changement

- Un parcours durable par société, partagé entre ses projets, avec bail borné,
  génération clôturée et reprise après expiration. Aucun verrou DB pendant l'IA.
- Catalogue des identités des versions actuelles, autorisées, triées de façon
  stable (règles société d'abord). Maximum explicite de 10 000 sources : dépassement
  visible et aucune affirmation de couverture. Le contenu est hydraté par lot.
- Réceptions durables par version source : une nouvelle version s’ajoute à la
  frontière sans remettre les anciennes en tête. Chaque ancre sélectionne au plus
  12 voisins partageant des termes métier, avec priorité aux règles société. Les
  paires déjà évaluées sont retirées avant appel. Aucune matrice IA quadratique.
  Les décisions humaines restent intactes.
- Chaque lot respecte les plafonds de sources, paires et 160 Ko de contexte.
  La sélection heuristique ne devient jamais une preuve de couverture sémantique.
- Succès/réception/continuation outbox sont atomiques. Un échec ne saute aucune
  paire ; le bail périmé permet la reprise. Les continuations reprennent l'acteur
  originel, vérifient ses droits et restent soumises aux quotas/arrêt société.
- Maximum de 6 appels steward par heure/société, en plus des quotas IA existants ;
  continuation différée de 60 secondes. Un lot sans paire inédite n’appelle pas
  de fournisseur. Les erreurs conservent l’ancre en attente.
- État lisible : parcours en cours/terminé/bloqué, sources, paires parcourues,
  évaluées, exclusions heuristiques et limites. Terminé signifie parcours des
  candidats, jamais certification de toutes les contradictions.
- Migration 18, inventaire d'effacement, export sans capability de bail et
  extensions runtime alignés. Fingerprint d'effacement à recalculer seulement
  après inspection réelle du schéma migré (avec migration 19 concurrente).

## Preuves

- Tests purs des termes et des bornes ; aucune ancre perdue entre lots.
- Intégration fictive >128 sources et >48 paires, règle société ancienne et
  source d'un autre projet ; pas de second appel pour une paire déjà évaluée.
- Bail concurrent, échec/reprise, révocation acteur, société étrangère et lecteur.
- Revue concurrence : verrou de progression acquis avant les verrous projets ;
  clôture conditionnée au jeton encore valide avec une seule ligne modifiée.
  Un ancien worker repris après expiration ne peut inscrire aucun résultat,
  reçu ni continuation. Son model run passe à l’échec.
- La continuation se déduplique par acteur : celle d’un acteur révoqué ne doit
  pas empêcher un autre membre autorisé de reprendre les versions en attente.
- Deux fichiers distincts du même corpus GitHub restent comparables ; la
  collecte commune ne représente pas l’identité du document.
- Scénarios steward existants conservés pour provenance/code insuffisant et
  décisions humaines. Aucun fournisseur réel ni DB lancés par ce sous-lot.
## Intégration maintenance (coordination)

Les deux nouveaux états de progression entrent dans les exports et inventaires
d’effacement. Un bail de vérification encore actif interdit l’effacement d’une
société comme d’un projet. Les reçus de versions du projet sont effacés avec lui ;
la progression partagée et les autres sociétés restent identiques. La clôture
hors ligne d’un bail abandonné invalide son jeton afin qu’un ancien travailleur ne
puisse plus terminer. La migration de revue finale fixe l’empreinte du schéma
après inspection des nouvelles colonnes et clés étrangères, puis les exercices
sur base jetable prouvent ces frontières.
