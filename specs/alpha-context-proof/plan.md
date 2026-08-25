# Plan d'implémentation — Alpha Context Proof

> Ce plan est séquentiel. Chaque gate ferme la phase avant l'ouverture de la
> suivante. Les évaluations OpenAI réelles ne s'exécutent jamais en CI.

## Phase 0 — Contrat et baseline

- Valider `spec.md`, les dix parcours et les trois diagrammes d'architecture.
- Classer les preuves existantes avec les quatre statuts normatifs.
- Séparer la baseline documentaire de l'implémentation fonctionnelle.

**Gate G0** : périmètre, ACP, parcours et preuves approuvés.

## Phase 1 — Reproductibilité desktop

- Aligner les environnements sans secret sur `deterministic`.
- Isoler PostgreSQL/Supabase pour les tests d'intégration.
- Configurer Vitest et Playwright dans des répertoires de tests distincts.
- Activer la CI web desktop, Rust, PostgreSQL et Tauri Linux.
- Conserver Android/iOS/mobile entièrement hors des commandes actives.

**Gate G1** : clone neuf, base neuve, tests et builds desktop verts.

## Phase 2 — Identité et isolation

- Introduire magic link, JWT vérifié côté Axum et `RequestContext`.
- Ajouter les memberships owner/editor/viewer et les policies RLS.
- Utiliser un rôle runtime `NOBYPASSRLS` et des transactions scopées.
- Refuser un bind non-loopback sans configuration sûre.

**Gate G2** : zéro accès inter-workspace/inter-projet dans la matrice de rôles.

## Phase 3 — Commandes fiables

- Ajouter l'idempotence persistée et le contrôle optimiste.
- Persister les `model_runs` et corréler les request IDs.
- Transformer les événements en outbox avec lease et reprise.
- Sortir tous les appels réseau des transactions métier.

**Gate G3** : timeout, crash, double clic et replay sans perte ni duplication.

## Phase 4 — Context Compiler réel

- Séparer les opérations IA et leurs schémas structurés.
- Compiler les obligations par règles puis les candidats par sélection IA.
- Enregistrer décisions, raisons, budget et versions sources.
- Rendre packs, exports et handoffs strictement versionnés et project-scoped.
- Générer plan et couverture depuis le pack, sans contenu `Credits v2` codé.

**Gate G4** : trois projets distincts produisent des packs sélectifs et sourcés.

## Phase 5 — Steward et obsolescence

- Classifier les paires candidates de façon asynchrone.
- Persister contradiction, compatibilité et ambiguïté.
- Distinguer accept, dismiss et resolve.
- Propager une invalidation ciblée puis obliger la recompilation.

**Gate G5** : contradiction → mutation → stale → recompilation → couverture.

## Phase 6 — Parcours desktop complets

- Fermer les dix parcours avec états pending/success/error/retry.
- Restaurer handoffs et saisies après reload/offline.
- Exposer versions, provenance, staleness et raisons de sélection.
- Corriger accessibilité, clavier, focus et attribution projet.

**Gate G6** : UF-01 à UF-10 passent sur les deux viewports desktop.

## Phase 7 — GitHub read-only

- Ajouter connexions, références externes et observations append-only.
- Exporter JSON/Markdown puis importer PR, commit et checks allowlistés.
- Ajouter validation humaine des preuves et refresh ETag/idempotent.
- Tester permissions read-only, SSRF, rate limits et perte d'accès.

**Gate G7** : ContextPack → outil externe → GitHub → preuve → couverture.

## Phase 8 — Certification déterministe

- Exécuter unités, DB, contrats fournisseurs, Playwright et Tauri Linux.
- Couvrir succès, blocages, 401/403/404/409/422/429/5xx, offline et retry.
- Vérifier secret scan, backup et restauration.

**Gate G8** : aucune fuite, duplication, erreur console ou violation axe
critical/serious.

## Phase 9 — Campagne réelle comparative

- Créer une clé et un projet OpenAI dédiés hors dépôt.
- Calibrer au plus deux modèles dans l'enveloppe de 10 USD.
- Geler modèle, prompts et schémas ; exécuter le protocole A/B.
- Agréger uniquement des métriques et identifiants expurgés.

**Gate G9** : budget et tous les seuils de `validation.md` respectés.

## Phase 10 — Context Proof propriétaire

- Exécuter dix boucles réelles réparties sur trois projets pendant une semaine.
- Créer les PR dans les outils externes, jamais depuis AI Center.
- Corriger les P0 et rejouer toutes les preuves impactées.

**Gate G10** : avantage mesuré et registre de preuves courant.

## Phase 11 — Alpha équipe et décision V1

- Déployer une web privée HTTPS et inviter deux à trois utilisateurs.
- Activer OpenAI puis GitHub par feature flags.
- Exploiter deux semaines avec métriques et restauration testée.
- Classer les constats `prouvé`, `à consolider`, `non prouvé`, `différé`.

**Gate G11** : `v0.2.0-alpha.1` promue ou décision explicite de ne pas
promouvoir. La V1 est planifiée uniquement à partir de ces preuves.

## Ordre des migrations et compatibilité

1. Modifier le schéma déclaratif.
2. Générer puis relire la migration SQL.
3. Ajouter les nouvelles colonnes/tables sans supprimer les anciennes.
4. Dual-write des références texte et structurées.
5. Backfill idempotent et contrôles de cardinalité.
6. Basculer les lectures.
7. Supprimer l'ancien format dans une migration ultérieure seulement.

Chaque foreign key ajoutée reçoit son index, chaque version durable une
contrainte unique, et chaque transaction externe suit le pattern
pending → appel hors transaction → finalisation optimiste.

## Traçabilité

Toute pull request d'implémentation doit citer au moins un identifiant ACP et
indiquer : preuves ajoutées, migrations, risques de rollback et statut de la
validation. Les gates ne sont jamais déclarées atteintes par la seule présence
de code.
