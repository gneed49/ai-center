# Graphe des tickets

| Ticket | Objet | Dépendances | Responsable | État au 7 septembre |
| --- | --- | --- | --- | --- |
| PC-T01 | Catalogue, transport structuré commun et clients API officiels | Contrat | Agent transports | Terminé, intégré |
| PC-T02 | Chiffrement, schéma/RLS, routes, sélection et résolution moteur/Steward | Contrat ; T01 pour compilation finale | Agent backend | Terminé, intégré |
| PC-T03 | Protocole officiel abonnement, isolation, faux processus | Recherche ; T01/T02 pour raccordement | Agent abonnements | Terminé, limites de client documentées |
| PC-T04 | Réglages UI et tests de composants/API | Contrat ; T02/T03 pour raccordement | Agent principal dans worktree UI | Terminé, intégré |
| PC-T05 | Fusion, intégration, documentation et revue | T01 à T04 | Fusion puis revue Standards/Spec | Terminé, constats corrigés et re-revus |

Les critères PC-001 à PC-016 sont la référence d'acceptation. Les tests E2E
sont pris en charge par le propriétaire selon sa consigne du 7 septembre.

La livraison locale est intégrée dans `e8b0541`. Le
[rapport de validation](validation-2026-09-07.md) distingue les tests reproduits,
les deux axes de revue et les essais réels confiés au propriétaire.
