# Preuves de code, sources exactes et accès privé — validation frontend

Livraison locale du 21 septembre 2026 sous le mandat autonome Company Context V1.
Spécification de référence : `github-code-plan.md`, invalidation ciblée de
`spec.md` et accès privé de `team-access-plan.md`.

## Comportements livrés

- `/projects/:id/code` sélectionne une connexion GitHub existante, un dépôt,
  un SHA de commit complet et un à dix chemins relatifs bornés. Le bouton
  explicite déclenche la lecture ; une réponse incertaine conserve la même
  identité de commande pour une demande inchangée.
- Historique paginé, observation immuable, état de chaque fichier, texte observé
  avec numéros de lignes et copie de référence commit/chemin/lignes/observation.
  Les liens sont construits sur github.com uniquement. L’interface n’attribue
  jamais la preuve de quelques fichiers à tout le dépôt.
- Les sources de signal exposent les métadonnées exactes de provenance et
  indiquent expressément la couverture limitée au fichier observé.
- La fraîcheur du ContextPack provient de `status` servi après contrôle des
  versions sources et scopes actifs. Le numéro historique du graphe n’invalide
  plus à lui seul un pack. L’export vérifie encore son état serveur avant action.
- L’amorçage d’entreprise lit `/api/workspaces/capabilities` ; une identité non
  autorisée est guidée vers une invitation et ne voit pas le formulaire de création.
- La confirmation de publication montre la cible exacte en plus de son libellé.

## Preuves exécutées

- 133 tests unitaires/composants, 36 fichiers, tous passants, suite web complète.
- Lint web et build TypeScript/Vite passants ; diff sans erreur d’espacement.
- Chromium desktop : les trois nouveaux scénarios passent (invitation dont le
  secret reste hors requête de création, pause/reprise, révision/validation et
  publication en attente annulable, preuve de code et citation exacte).
- La page de preuve de code passe axe WCAG 2A/2AA/2.1AA et le contrôle de
  débordement horizontal à 1440 px.
- Les huit scénarios de preuves externes passent, incluant un pack courant après
  modification indépendante du graphe et un pack stale au compteur inchangé.
- Les nouveaux adaptateurs navigateur sont dans `e2e/company-mock-api.ts`, activés
  par `companySuite:true` pour les données nouvelles, avec fixtures synthétiques.

Ces résultats utilisent des doubles d’API et des données `[FICTIF]`. Ils ne
prouvent ni accès GitHub réel, ni e-mail envoyé, ni publication externe, ni
mise en production. La recette serveur/PostgreSQL et la suite navigateur globale
sont consolidées séparément par l’agent principal.
