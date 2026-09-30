# Plan — Reprise fiable et export du contexte web

> Statut : livré localement, sans publication  
> Spec liée : [spec.md](spec.md)

## Approche

Conserver le transport API et les composants existants. Partager le décodage
HTTP JSON/texte avec la même frontière d'identité. Ajouter un composant export
isolé au handoff. Maintenir l'identité de la dernière commande dans la session
tant qu'une réponse n'est pas confirmée, même après édition temporaire du texte.

## Découpage

1. Définir les contrats HTTP texte et réessai, puis les tests de régression.
2. Ajouter les actions d'export et le contrôle de fraîcheur au clic.
3. Corriger les messages d'erreur et la réutilisation de commande.
4. Exécuter unités, lint et build ; documenter résultats et limites.

## Impacts

- API : consommation de deux routes de lecture existantes ; aucune migration.
- Frontend : session, handoff, transport et composant export isolé.
- Sécurité : requêtes authentifiées, no-store pour exports, purge par identité,
  aucune persistance de contenu exporté dans le navigateur.
- Exploitation : aucun déploiement, fournisseur réel ou appel payant.

## Validation

- [x] Tests API de lecture texte, erreurs et changement d'identité.
- [x] Tests UI de réponse perdue, double chemin de reprise et nouveau contenu.
- [x] Tests UI export nominal, obsolète, échec et changement d'identité.
- [x] Lint, TypeScript et build web.

## Déploiement et retour arrière

Livraison du bundle avec le serveur existant après validation globale V1.
Retour arrière par le bundle précédent ; aucun schéma ou contenu canonique
n'est modifié par ce lot.
