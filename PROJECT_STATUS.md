---
project: AI Center
status_schema: 1
last_reviewed: 2026-09-05
stage: candidate-alpha
health: amber
canonical_path: /home/gneed49/Documents/projects/AICenter
audit_base_commit: 8a6e0da08bc2cd47088e0b25af398ec3ba30312c
verified_consolidation_commit: 90ca75885da16845abaca513ca468984a5eabe79
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

- Point de contrôle local `90ca758` : T01 à T10 réunis ; la revue Standards/Spec a trouvé huit corrections en cours, non couvertes par les résultats ci-dessous.
- Qualité sur `c8a723b` : fmt, lint, Clippy strict, builds web/serveur, 89 tests Rust, 6 Vitest, 20 tests d'évaluation offline et 17 gardes stack.
- Desktop API/Auth simulées sur `90ca758` : 126/126, soit 42 scénarios sur trois configurations desktop, sans retry.
- PostgreSQL sur `90ca758` : base neuve, upgrade, 32 pgTAP, neuf tests métier, rôle runtime RLS ; auth locale avec réponses attendues 200/200/403/403/401.
- Sauvegarde restaurée : 39 tables et 96 policies comparées ; nettoyage confirmé.
- Client Tauri Linux compilé avec overlay de smoke et empreinte ; quatre tests de garde hors interface. Aucune exécution du parcours natif.
- Parcours navigateur réel jusqu'au handoff rechargé : preuve antérieure sur `37921d8`. L'extension récente plan/couverture/historique reste non reproduite.
- PR draft n°1 au commit distant `8a6e0da`, vérifiée ; aucune CI distante des nouveaux commits locaux.
- [Audit courant](docs/current-state-audit.md), [tickets](specs/alpha-context-proof/tickets.md) et [registre détaillé](specs/alpha-context-proof/validation.md).

## Risques et portes

- Huit constats de revue à corriger, puis validation consolidée à reproduire.
- Protocole d'évaluation en cours de correction ; campagne comparative réelle non réalisée.
- Corpus réels supplémentaires, annotations humaines et GitHub App privée à valider.
- Exploration UI et smoke Tauri bloqués par le contrôle d'autorisation de l'environnement ; aucune relance indirecte. Build et tests hors interface disponibles.
- CI courante, HTTPS privé, dogfood d'une semaine et alpha équipe de deux semaines à réaliser.
- Quota OpenAI historique non recontrôlé ; Android est hors du jalon actuel.

## Prochaine étape

Fusionner les corrections de revue, reproduire les tests concernés et mettre à
jour la PR avec les preuves exactes. Fermer ensuite les gates réelles avec
corpus autorisé, fournisseurs configurés, annotations humaines et périodes
requises. Ne pas créer `v0.2.0-alpha.1` avant leur réussite.

## Mise à jour par un agent

Lire d'abord `AGENTS.md`, `docs/product/`, les specs actives et le registre de preuve. Vérifier branche, HEAD, PR/CI et gates. Séparer contrôle local, CI et intégration externe. Ne jamais lire ni committer de secret. Mettre à jour ce fichier et son `last_reviewed` dans le même commit que la nouvelle preuve.
