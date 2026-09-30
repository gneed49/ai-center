# Plan de livraison — connexions IA

Base locale : `8861eca0854b74ab43a2a94013e98e804ab41290` sur
`feat/alpha-context-proof`. Le complément documentaire antérieur reste local.

1. [x] Figer le contrat partagé et vérifier les protocoles officiels.
2. [x] Implémenter en parallèle les transports API, le stockage/les routes et l'interface de réglages, dans des worktrees distincts.
3. [x] Ajouter la connexion par abonnement selon les capacités officielles vérifiées, avec isolation et outils désactivés.
4. [x] Fusionner les tickets, exécuter tests unitaires/contrats/intégration PostgreSQL, lint/build et revue Standards/Spec.
5. [x] Corriger les constats de revue dans un seul worktree, mettre à jour les preuves et fournir les étapes de recette manuelle.

Livraison logicielle intégrée localement dans `e8b0541` après correction unique
`5ea7ce3` et deux re-revues sans constat restant. Les preuves et leurs limites
sont dans le [rapport du 7 septembre](validation-2026-09-07.md) ; les parcours
confiés au propriétaire sont dans la [recette manuelle](manual-verification.md).

Aucun E2E, appel IA facturé, login réel ou contrôle d'interface automatisé.
Ne pas lancer `integration-stack.sh run` sans phases explicites : utiliser
`integration`, `backup-restore`, `auth-smoke` selon les changements.
Aucun push pendant cette implémentation : la portée publiée précédente a été
explicitement limitée par le contrôle d'approbation.
