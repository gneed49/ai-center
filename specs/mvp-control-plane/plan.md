# Plan d’implémentation — MVP du plan de contrôle contextuel

> Statut : completed  
> Spec liée : `spec.md`

## Approche

Monorepo hybride :

- `apps/web` — React 19, TypeScript, Vite, shadcn, Tailwind, React Router,
  TanStack Query ;
- `apps/server` — Rust, Axum, SQLx, moteur de domaine et API HTTP ;
- `apps/desktop/src-tauri` — enveloppe Tauri v2 et commandes natives minimales ;
- `supabase` — schémas déclaratifs, migrations, seed et tests SQL.

Le navigateur et Tauri consomment la même API. Le domaine, la clé OpenAI et
l’accès PostgreSQL restent sur le serveur. L’API OpenAI Responses est encapsulée
derrière un trait ; les tests utilisent une implémentation déterministe.

Alternatives rejetées :

- Supabase directement depuis React : exposition prématurée du domaine et des
  secrets, règles réparties entre client et base ;
- cœur métier dans Tauri : impossible à consommer proprement depuis le web ;
- Electron : ne respecte pas le choix Rust/Tauri ;
- full-local seulement : contredit l’orientation serveur et limite la reprise.

## Découpage

1. **Fondations** — workspaces npm/cargo, outillage, contrats communs, CI locale.
2. **Données** — schéma Supabase privé, indexes, RLS défensive, seed Credits v2.
3. **Domaine/API** — projet, graphe, sessions, mutations, gate, context compiler.
4. **Agents** — profils Produit/Tech, Responses API structurée, test double.
5. **Contrôle** — livrables, preuves, couverture, steward et invalidation.
6. **Frontend** — shell, routes P0, états responsive, interactions réelles.
7. **Desktop** — Tauri Linux, configuration du serveur et build installable.
8. **Validation** — tests unitaires/intégration/E2E, revue visuelle et sécurité.

## Impacts

- Domaine : agrégat Project et projections immutables versionnées.
- API : REST JSON avec erreurs typées et idempotence des confirmations.
- Données : schéma `app` non exposé à PostgREST, UUID et timestamps UTC.
- Frontend : aucune fixture finale ; état serveur via TanStack Query.
- Sécurité : secrets serveur, CORS local explicite, limites de payload et audit.
- Exploitation : variables d’environnement, healthcheck et migrations au boot
  uniquement en développement.

## Validation

- [x] Format, lint et typecheck TypeScript
- [x] Format, clippy et tests Rust
- [x] Tests unitaires du domaine
- [x] Tests d’intégration PostgreSQL
- [x] Tests des politiques et indexes
- [x] Parcours E2E Credits v2
- [x] Vérification responsive desktop/mobile dans le Browser
- [x] Comparaison visuelle concepts/rendus avec registre des écarts
- [x] Build web production
- [x] Build Tauri Linux
- [x] Audit secrets, dépendances et surfaces API
- [x] Matrice AC-01 à AC-20 avec preuve directe

## Déploiement et retour arrière

Le serveur est un binaire stateless devant PostgreSQL. Les migrations sont
versionnées et appliquées avant déploiement. Le rollback applicatif réutilise la
version précédente du serveur ; un rollback de schéma déjà publié se fait par
une migration corrective en avant. Le client web est un bundle statique et
Tauri embarque ce même bundle.
