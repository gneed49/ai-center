# Graphe des tickets

| Ticket | Objet | Dépendances | Responsable |
| --- | --- | --- | --- |
| PC-T01 | Catalogue, transport structuré commun et clients API officiels | Contrat | Agent transports |
| PC-T02 | Chiffrement, schéma/RLS, routes, sélection et résolution moteur/Steward | Contrat ; T01 pour compilation finale | Agent backend |
| PC-T03 | Protocole officiel abonnement, isolation, faux processus | Recherche ; T01/T02 pour raccordement | Agent abonnements |
| PC-T04 | Réglages UI et tests de composants/API | Contrat ; T02/T03 pour raccordement | Agent principal dans worktree UI |
| PC-T05 | Fusion, intégration, documentation et revue | T01 à T04 | Fusion puis revue Standards/Spec |

Les critères PC-001 à PC-016 sont la référence d'acceptation. Les tests E2E
sont pris en charge par le propriétaire selon sa consigne du 7 septembre.
