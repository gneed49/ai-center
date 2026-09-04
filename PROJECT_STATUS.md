---
project: AI Center
status_schema: 1
last_reviewed: 2026-09-03
stage: candidate-alpha
health: amber
canonical_path: /home/gneed49/Documents/projects/AICenter
audit_base_commit: 8a6e0da08bc2cd47088e0b25af398ec3ba30312c
tracking_commit: resolve-with-git-log--1---PROJECT_STATUS.md
---

# État du projet — AI Center

## Résumé

AI Center est une fondation desktop/web de contrôle de contexte et de cohérence. La branche de référence est `feat/alpha-context-proof`, 14 commits devant l'ancien MVP `feat/unified-dev-flow`. La fondation est solide, mais les portes externes empêchent encore une release alpha.

## Ce qui est en place

- Context Compiler, ContextPacks immuables et provenance.
- Workspaces, JWT Supabase, rôle PostgreSQL runtime et RLS forcée.
- Steward, contradictions, résolution et recompilation ciblée.
- Sessions techniques, preuves, outbox/reprise, GitHub read-only.
- React/Vite, Rust/Axum/SQLx, Tauri, OCI et sauvegarde/restauration.

## Preuves

- PR draft n°1 ouverte, fusionnable et propre au 3 septembre 2026.
- CI Desktop et OCI vertes au commit d'audit.
- Registre : 75 tests Rust, 32 pgTAP, 6 PostgreSQL, 4 Vitest, 57 Playwright simulés, 1 full-stack Chromium et 9 tests offline.
- Équivalence de restauration enregistrée pour 39 tables et 96 policies.

## Risques et portes

- Quota OpenAI et évaluation comparative réels non terminés.
- Corpus réels supplémentaires et GitHub App privée à valider.
- Smoke Tauri interactif, workflow manuel, HTTPS privé, dogfood et alpha équipe à réaliser.
- Android est hors du jalon Alpha Context Proof actuel.

## Prochaine étape

Fermer les gates du registre de preuve dans l'ordre : fournisseur réel, corpus, GitHub App, Tauri manuel, hébergement privé, dogfood, équipe alpha. Ne pas créer `v0.2.0-alpha.1` avant cela.

## Mise à jour par un agent

Lire d'abord `AGENTS.md`, `docs/product/`, les specs actives et le registre de preuve. Vérifier branche, HEAD, PR/CI et gates. Séparer contrôle local, CI et intégration externe. Ne jamais lire ni committer de secret. Mettre à jour ce fichier et son `last_reviewed` dans le même commit que la nouvelle preuve.
