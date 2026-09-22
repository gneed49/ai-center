# Export projet — validation frontend

Lot autorisé dans le mandat autonome Company Context V1, le 21 septembre 2026,
selon `project-data-plan.md`. Le backend définit `GET /api/projects/{id}/export`,
propriétaire seulement, disponible aussi pour les archives.

## Livraison

Le bouton « Exporter ce projet » apparaît sur la page du projet pour le
propriétaire. Il télécharge les octets canoniques reçus avec `cache: no-store`,
sous `ai-center-project-{id}.json`. Il ne reconstruit ni ne réduit les versions
sources citées. Le fichier conserve les exclusions produites par le serveur.
Le conteneur société ne reçoit pas ce bouton.

Une erreur de limite ou de transport n’engendre aucun fichier. L’identité de
connexion et la société sont vérifiées avant de créer le téléchargement ; une
réponse d’un espace abandonné est ignorée. Le lien objet est révoqué après usage.
L’interface ne propose aucun effacement de projet.

## Validation

- Après la réinstallation contrôlée des dépendances corrigées : 149 tests dans
  39 fichiers passent avec Vitest 4.1.11 ; lint et build web passent.
- Les cinq tests de composant couvrent les octets exacts, l’archive absente de la
  liste active, le refus d’interface pour éditeur/lecteur, le changement de société
  pendant la réponse et le refus de produire un fichier partiel.
- Le scénario de téléchargement navigateur passe sur Chromium Desktop,
  Chromium Compact et Firefox Desktop : 3/3, contenu canonique vérifié.
- La recette Firefox globale précédant ce lot est verte : 48/48, dont nouveaux
  parcours, contrôles d’accessibilité et captures sur six écrans principaux.

Cette preuve frontend emploie des fixtures `[FICTIF]` et une API simulée.
La validation du filtrage des données, des sources externes exactes, des limites
et des autorisations serveur appartient à la recette PostgreSQL/API séparée.
Aucune donnée réelle n’a été exportée ou supprimée.
