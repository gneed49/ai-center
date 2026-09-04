---
project: AI Center
status_schema: 1
last_reviewed: 2026-09-05
stage: candidate-alpha
health: amber
canonical_path: /home/gneed49/Documents/projects/AICenter
audit_base_commit: 8a6e0da08bc2cd47088e0b25af398ec3ba30312c
verified_consolidation_commit: 37921d81f2336fc88d98986d4bd798ba6f516f49
tracking_commit: resolve-with-git-log--1---PROJECT_STATUS.md
---

# État du projet — AI Center

## Résumé

AI Center est une fondation desktop/web de contrôle de contexte et de cohérence.
La reprise du plan Alpha Context Proof se poursuit sur `feat/alpha-context-proof`
dans le checkout de ce projet ChatGPT. Le chemin canonique ci-dessus est conservé ;
son commit de suivi local a été repris sans modifier ce checkout canonique.
Les changements de cette reprise sont locaux ; les gates de release restent ouvertes.

## Ce qui est en place

- Context Compiler, ContextPacks immuables et provenance.
- Workspaces, JWT Supabase, rôle PostgreSQL runtime et RLS forcée.
- Steward, contradictions, résolution et recompilation ciblée.
- Sessions techniques, preuves, outbox/reprise, GitHub read-only.
- React/Vite, Rust/Axum/SQLx, Tauri, OCI et sauvegarde/restauration.

## Preuves

- PR draft n°1 et CI Desktop/OCI vérifiées sur le commit distant `8a6e0da` avant reprise ; les nouveaux commits locaux n'ont pas encore de preuve CI distante.
- Consolidation `37921d8` : stack isolée T01, invalidation ciblée T02, GitHub repository/PR/commit T03 et couverture structurée indépendante T07 intégrés.
- Cycle local complet réussi : 32 pgTAP, neuf tests métier PostgreSQL, magic link et refus de permissions attendus, un parcours Chromium full-stack jusqu'au handoff rechargé.
- Restauration de 39 tables et 96 policies vérifiée ; aucune ressource de la stack jetable après nettoyage.
- 17 tests de garde de stack, liens Markdown et scanner de secrets réussis. Le build Tauri Linux depuis la consolidation passe ; il ne constitue pas un smoke métier natif.
- [Tickets et point de contrôle](specs/alpha-context-proof/tickets.md) ; [registre détaillé](specs/alpha-context-proof/validation.md), dont les preuves antérieures restent datées.

## Risques et portes

- Quota OpenAI et évaluation comparative réels non terminés.
- Corpus réels supplémentaires et GitHub App privée à valider.
- Correction du cache au changement d'identité, certification des parcours, traçabilité externe, protocole d'évaluation et dépendances encore en cours de consolidation.
- Smoke Tauri interactif en attente d'autorisation de l'environnement de validation ; workflow manuel, HTTPS privé, dogfood et alpha équipe à réaliser.
- Android est hors du jalon Alpha Context Proof actuel.

## Prochaine étape

Terminer T04/T05/T08/T09/T10, réunir les changements et effectuer la revue
Standards/Spec avant mise à jour de la PR. Fermer ensuite les gates réelles du
plan avec corpus autorisé, fournisseurs configurés, annotations humaines et
périodes d'usage requises. Ne pas créer `v0.2.0-alpha.1` avant leur réussite.

## Mise à jour par un agent

Lire d'abord `AGENTS.md`, `docs/product/`, les specs actives et le registre de preuve. Vérifier branche, HEAD, PR/CI et gates. Séparer contrôle local, CI et intégration externe. Ne jamais lire ni committer de secret. Mettre à jour ce fichier et son `last_reviewed` dans le même commit que la nouvelle preuve.
