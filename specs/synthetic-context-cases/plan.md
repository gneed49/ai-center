# Plan — deux projets fictifs

Base de revue : `85b551759328ccb23004b0b5decfa5db985e17ca`.
Source : [spec.md](spec.md), amendement utilisateur du 8 septembre 2026.

## Graphe de tickets

`SYN-T01 → SYN-T02`

### SYN-T01 — Fixtures et tests exécutables

Statut : livré le 8 septembre 2026. Exigences SYN-001 à SYN-007.

Les fixtures versionnées Médiathèque Partagée et Atelier Réparable alimentent
trois contrats du compilateur sans base et un test d'intégration couplé des
deux projets. La collecte `quality` et `integration` inclut ces binaires.
Les trois contrats passent ; le test PostgreSQL exécuté par l'orchestrateur
sur la stack isolée avec le rôle runtime RLS passe en 2,79 s. Le premier montage
de fixture ouvrait directement une session Tech : il a été corrigé pour
respecter le handoff obligatoire. Aucun comportement produit n'a été modifié.

Un implémenteur dans un worktree dédié crée les deux fixtures explicites,
ajoute des assertions comportementales aux interfaces convenues et raccorde
les tests à la collecte existante. Ne pas recopier simplement un ancien
parcours sous deux noms ; vérifier en particulier leur coexistence et
l'invalidation limitée au projet concerné. Privilégier le moteur déterministe
existant, sans lui attribuer de compréhension générale du domaine. Corriger
uniquement les défauts produits réellement révélés par ces scénarios.

### SYN-T02 — Exécution, intégration et revue

Statut : à faire. Dépend de SYN-T01.

Exécuter les unités pertinentes et les intégrations PostgreSQL sur la stack
jetable gardée, sans phase E2E. Faire fusionner le commit par un agent merger,
relire les axes Standards et Spec indépendamment et confier toute correction
à un seul implémenteur. Consigner les résultats réels, mettre à jour le suivi
Alpha pour retirer l'attente des deux projets réels de ce lot, puis nettoyer
la stack et les worktrees. Les changements restent locaux.
