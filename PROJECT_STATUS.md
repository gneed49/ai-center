---
project: AI Center
status_schema: 1
last_reviewed: 2026-09-08
stage: candidate-alpha
health: amber
publication_status: public_baseline_pr_open_local_changes_release_gates_pending
canonical_path: /home/gneed49/Documents/projects/AICenter
audit_base_commit: a22335cd1849c9b8c5f317eb94905fa3489a51a8
verified_consolidation_commit: b20d36fa9dd95b0a5b2fad8266b81ca9e31acb64
verified_synthetic_cases_commit: 5e0e98d82f4f538bbf83060001141a2dd3729d09
verified_public_commit: 91dc76b95b798ec6adc22480a86db9e9e09da200
tracking_commit: resolve-with-git-log--1---PROJECT_STATUS.md
---

# État du projet — AI Center

## Résumé

AI Center est une application desktop/web de contrôle du contexte et de la
cohérence. La branche locale `feat/alpha-context-proof` du projet ChatGPT
contient les connexions IA personnelles du 7 septembre et les corrections
ACP-T11/T12/T13 du 8 septembre, ainsi que les tests des deux projets fictifs
Médiathèque Partagée et Atelier Réparable. Le checkout canonique ci-dessus
reste intact.

La [PR n°1](https://github.com/gneed49/ai-center/pull/1), « feat: consolider la
boucle Alpha Context Proof », est ouverte et prête pour revue au point public
`91dc76b`, vérifié en lecture seule le 8 septembre. Les ajouts suivants restent
locaux ; aucun nouveau push, déploiement ou lancement de CI n'a eu lieu.

## Ce qui est en place

- Context Compiler, ContextPacks immuables, provenance, plans et couverture.
- Workspaces, JWT Supabase, rôle PostgreSQL runtime et RLS forcée.
- Steward, contradictions, résolution et recompilation ciblée.
- Sessions techniques, preuves, outbox/reprise et GitHub en lecture seule.
- Plusieurs profils OpenAI, Anthropic, Kimi, DeepSeek et OpenRouter ; clés
  personnelles chiffrées côté serveur et sélection dans **Réglages IA**.
- Abonnement Claude via le client officiel Linux 2.1.220, comptes personnels
  Pro/Max admissibles. ChatGPT/Codex reste indisponible dans cette version.
- Upgrade sur toutes les migrations, conflit explicite quand une identité de
  message est réutilisée avec un autre contenu, transport d'évaluation borné.
- React/Vite, Rust/Axum/SQLx, Tauri, images OCI et sauvegarde/restauration SQL.

## Preuves datées

- **8 septembre, SYN-T01/T02 — `Vérifié` logiciel :** les deux projets
  fictifs parcourent les services et PostgreSQL dans un même workspace.
  3 nouveaux contrats du compilateur et 1 intégration couplée réussis :
  confirmation, sélection/provenance, isolation, contradiction, révision et
  recompilation avec historique conservé et atelier inchangé. Également
  reproduits : 125 unités serveur (1 ignorée), 32 pgTAP, 41 tests Alpha et
  17 gardes. Les [preuves et limites](specs/synthetic-context-cases/validation-2026-09-08.md)
  et les [deux revues sans constat](specs/synthetic-context-cases/review-2026-09-08.md)
  sont consignées. Base jetable, volumes et worktree nettoyés ; aucun E2E.
- **8 septembre, ACP-T14 — `Vérifié` documentaire :** la
  [vue FigJam Alpha Context Proof](https://www.figma.com/board/gn5z1F1tnQpQ23580fQ5QC?node-id=7-213)
  ajoute les 14 nœuds et 18 connecteurs du flow à côté du dessin historique
  conservé. Lecture API et inspection visuelle sont consignées dans la
  [note du schéma](docs/architecture/diagram-validation-2026-09-08.md).
  Mermaid et les exports sont inchangés ; les liens et le suivi Git restent
  locaux. Cette preuve ne valide aucun parcours fonctionnel supplémentaire.
- **8 septembre, sources intégrées à `b20d36f` sans retouche** : 7 intégrations
  PostgreSQL ciblées, 7 unités de migration, 17 unités de garde de cible,
  upgrade réel sur les 5 migrations et restauration de 41 tables/98 policies.
  Données logiques, objets, grants et RLS concordants ; le déchiffrement des
  clés après restauration avec la clé maîtresse séparée n'a pas été rejoué.
- **8 septembre** : 41 tests du harness Alpha, dont 14 nouveaux tests de
  transport ; Clippy serveur tous targets, formatage et syntaxe réussis.
  La stack et les volumes de test exclusifs ont été supprimés.
- **7 septembre, `5ea7ce3` intégré à `e8b0541`** : 125 unités Rust, 49 tests
  web, 32 pgTAP et 11 intégrations PostgreSQL ; revues Standards/Spec clôturées.
  Ces nombres ne désignent pas une nouvelle exécution le 8 septembre.
- **CI publique du 5 septembre à `91dc76b`, relue le 8 septembre** : chacun des
  deux déclenchements push/PR a réussi ses 5 contrôles Desktop CI et 3 OCI.
  Ces résultats ne certifient pas les changements locaux ultérieurs.
- Rapports : [reprise du 8 septembre](specs/alpha-context-proof/review-2026-09-08.md),
  [connexions du 7 septembre](specs/provider-connections/validation-2026-09-07.md),
  [historique du 5 septembre](specs/alpha-context-proof/review-2026-09-05.md).

## Preuves encore attendues

- Les E2E, l'exploration UI, le smoke Tauri et les essais avec les comptes réels
  sont confiés au propriétaire depuis sa consigne du 7 septembre. Aucun de
  ces parcours n'a été exécuté pendant les livraisons des 7/8 septembre.
- Campagne comparative et annotations humaines pour une preuve de valeur
  réelle. Pour la validation logicielle demandée le 8 septembre, le propriétaire
  autorise [deux projets fictifs](specs/synthetic-context-cases/spec.md) choisis
  et testés par l'agent ; aucun choix de projets réels n'est attendu pour ce lot.
- GitHub App privée en lecture seule et boucle réelle d'export jusqu'à la
  couverture ; dix boucles propriétaires sur trois projets pendant une semaine.
- Hébergement HTTPS privé, deux à trois invités, alertes et sauvegardes actives,
  restauration dans cet environnement et deux semaines d'observation.
- Schéma et serveur doivent être mis à niveau ensemble. Android reste hors
  périmètre. Aucun tag `v0.2.0-alpha.1` tant que les gates réels restent ouverts.

## Prochaine étape

Le propriétaire suit la [recette manuelle](specs/provider-connections/manual-verification.md)
et le [guide de démarrage](docs/development.md). Le
[registre de semaine propriétaire](specs/alpha-context-proof/owner-proof-template.md)
et le [dossier d'alpha privée](docs/operations/private-alpha-handoff.md) sont
prêts à renseigner ; ils ne constituent ni des résultats ni des services activés.

## Mise à jour par un agent

Lire `AGENTS.md`, les spécifications actives et les registres de preuve.
Vérifier branche, HEAD, PR/CI et gates. Séparer contrôle local, CI distante,
usage réel, publication et exploitation. Préserver les sources originales et
les secrets ; actualiser ce fichier dans le même commit que la nouvelle preuve.
