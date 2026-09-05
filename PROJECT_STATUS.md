---
project: AI Center
status_schema: 1
last_reviewed: 2026-09-05
stage: candidate-alpha
health: amber
publication_status: local_only_pending_owner_approval
canonical_path: /home/gneed49/Documents/projects/AICenter
audit_base_commit: 8a6e0da08bc2cd47088e0b25af398ec3ba30312c
verified_consolidation_commit: a9b3140dab9c0d0c78a2da4633d874cd1c79e86f
tracking_commit: resolve-with-git-log--1---PROJECT_STATUS.md
---

# État du projet — AI Center

## Résumé

AI Center est une fondation desktop/web de contrôle de contexte et de cohérence.
La reprise du plan Alpha Context Proof se poursuit sur `feat/alpha-context-proof`
dans le checkout de ce projet ChatGPT. Le chemin canonique ci-dessus est conservé ;
son commit de suivi local a été repris sans modifier ce checkout canonique.
La branche de consolidation est suivie par la PR n°1 ; les gates de release restent ouvertes. Le statut CI du HEAD se consulte sur la PR, séparément des preuves locales ci-dessous.

## Ce qui est en place

- Context Compiler, ContextPacks immuables et provenance.
- Workspaces, JWT Supabase, rôle PostgreSQL runtime et RLS forcée.
- Steward, contradictions, résolution et recompilation ciblée.
- Sessions techniques, preuves, outbox/reprise, GitHub read-only.
- React/Vite, Rust/Axum/SQLx, Tauri, OCI et sauvegarde/restauration.

## Preuves

- Consolidation fonctionnelle `a9b3140` : T01 à T10 réunis ; constats Standards/Spec et régression de calibration corrigés, revus et couverts par leurs tests.
- `7a1a64c` : 89 tests Rust, compilation de tous les tests et Clippy strict ; `fc30ab6` : 27 tests d'évaluation offline et schémas valides.
- PostgreSQL sur `7a1a64c`, sources identiques dans la fusion : base neuve, upgrade, 32 pgTAP, 11 tests métier, rôle runtime RLS. Concurrence du brief, fournisseur >31 secondes, perte de bail et cycles GitHub testés.
- Sur `84634b7` : sauvegarde/restauration de 39 tables et 96 policies ; auth locale avec réponses attendues 200/200/403/403/401 ; nettoyage confirmé.
- Web inchangé par les correctifs : six Vitest, lint/build ; 126 tests API/Auth simulées sur trois configurations desktop, sans retry.
- Tauri Linux : build avec overlay de smoke et empreinte ; quatre tests de garde hors interface. Aucun parcours natif métier exécuté.
- Ancien parcours navigateur réel jusqu'au handoff rechargé sur `37921d8`. Extension récente plan/couverture/historique non reproduite localement.
- [Rapport daté de revue et validation](specs/alpha-context-proof/review-2026-09-05.md), [audit](docs/current-state-audit.md), [tickets](specs/alpha-context-proof/tickets.md), [PR n°1](https://github.com/gneed49/ai-center/pull/1) et [issue n°2](https://github.com/gneed49/ai-center/issues/2).

## Risques et portes

- Publication bloquée : la revue automatique a refusé le push vers le dépôt public, faute d'autorisation explicite de publier ce code et cette documentation. Aucun nouveau commit de reprise n'a été poussé ; la PR distante reste à `8a6e0da`. Attendre l'accord du propriétaire sur le diff concret, sans autre canal de publication.

- Schéma des observations et serveur à mettre à niveau ensemble : l'ancienne version du serveur utilise la contrainte retirée. Aucun déploiement effectué.
- Campagne comparative réelle non réalisée ; deux projets supplémentaires, corpus autorisé, annotations humaines et GitHub App privée à valider.
- Exploration UI et smoke Tauri bloqués par le contrôle d'autorisation de l'environnement ; aucune relance indirecte. Build et tests hors interface disponibles.
- HTTPS privé, dogfood d'une semaine et alpha équipe de deux semaines à réaliser.
- Quota OpenAI historique non recontrôlé ; Android est hors du jalon actuel.

## Prochaine étape

Faire relire la PR avec ses preuves et son statut CI courant. Fermer ensuite
les gates réelles avec corpus autorisé, fournisseurs configurés, annotations
humaines et périodes requises. Ne pas créer `v0.2.0-alpha.1` avant leur réussite.

## Mise à jour par un agent

Lire d'abord `AGENTS.md`, `docs/product/`, les specs actives et le registre de preuve. Vérifier branche, HEAD, PR/CI et gates. Séparer contrôle local, CI et intégration externe. Ne jamais lire ni committer de secret. Mettre à jour ce fichier et son `last_reviewed` dans le même commit que la nouvelle preuve.
