---
project: AI Center
status_schema: 2
last_reviewed: 2026-09-22
stage: company-context-v1-integration
health: amber
publication_status: draft_pr_in_progress_not_deployed
canonical_path: /home/gneed49/.codex/.chatgpt-projects/g-p-6a777a349e208191a064f18e7f93230d/ai-center
preserved_previous_checkout: /home/gneed49/Documents/projects/AICenter
branch: feat/company-context-v1
audit_base_commit: 966ce9bbb597327e0ebbc3084836c6dedd759eca
implementation_committed: true
push_effectue: true
deployment_effectue: false
tracking_commit: resolve-with-git-log--1---PROJECT_STATUS.md
---

# État du projet — AI Center

## Décision et objectif actifs

Le 21 septembre, le propriétaire a autorisé tous les lots de développement,
tests, revue et préparation à la production. La cible est une application web
pour une société et ses équipes : projets partagés, agents métier, artefacts
versionnés, outils existants reliés, graphes projet fédérés et contrôle de
cohérence asynchrone. [La spécification active](specs/company-context-v1/spec.md)
et [les tickets](specs/company-context-v1/tickets.md) remplacent les limites
mono-utilisateur de l'ancien MVP. Le code de production des projets clients
reste produit dans les outils externes.

La réalisation est locale sur le checkout indiqué en métadonnées. L'ancien
checkout est préservé. Aucun changement de cette V1 n'est encore commité, poussé,
déployé ou publié. Aucun fournisseur IA facturant ni compte Notion/Linear/GitHub
réel n'a été appelé pour qualifier ces nouveaux chemins.

## Réalisation présente

- Société/workspace, membres et invitations, accès owner/editor/viewer,
  espace général et cinq profils métier ; projets partagés par société.
- Conversations et artefacts par projet ; versions, validation, historique,
  exports Markdown/JSON, destinations héritées ou remplacées.
- Connecteurs Notion, Linear et GitHub : secrets chiffrés côté serveur,
  publications explicites, jobs durables et réconciliation des résultats ambigus.
- Graphes persistés projet/entreprise avec provenance, interface graphique et
  liste accessible ; contexte autorisé, borné et versionné ; invalidation ciblée.
- Steward asynchrone alimenté par connaissances, artefacts, observations externes
  et lectures GitHub ciblées à un commit/fichier ; états incomplet/inconnu explicites.
- Suspension et quotas persistants des appels et publications ; coûts absents
  distingués de zéro ; admission privée à la création de société.
- Export de société, archivage/restauration, effacement opérateur en qualification,
  préparation des images et du déploiement. Reprise des conversations et contrôles
  de rétention en cours de recette intégrée.

Ces fonctionnalités présentes ne valent pas une acceptation automatique de
chaque exigence. Le registre de preuve du candidat indique leurs contrôles.

## Preuves courantes

Le [rapport du 22 septembre](specs/company-context-v1/candidate-validation-2026-09-22.md)
précise les validations reproduites et les écarts ouverts. Il prévaut sur la
photo intermédiaire ci-dessous. Les deux parcours API/DB réelles et les 20 scénarios société passent après
l’extension du graphe. Un jalon local est enregistré, avec les écarts restants
explicitement ouverts ; le jalon `9be97b6` est poussé dans la [PR de travail n°3](https://github.com/gneed49/ai-center/pull/3).
Ses huit contrôles CI distincts passent. Les corrections d’images/TLS suivantes
sont en consolidation ; aucune production n’est annoncée.

## Preuves intermédiaires du 21 septembre

- Une première photo qualité passe : formatage, lint, compilation web/serveur,
  Clippy workspace, 144 tests unitaires Rust, 122 tests web, 69 contrôles Python.
  Des corrections ultérieures demandent une nouvelle photo commune.
- Frontend élargi : 133 tests web dans 36 fichiers passent, ainsi que les suites
  ciblées invitations/arrêt, publication et preuves GitHub. Les mocks de ces
  tests ne prouvent pas un accès fournisseur réel.
- Intégration PostgreSQL isolée : migration depuis la baseline, 32 pgTAP,
  vérification des privilèges runtime/RLS ; modules société (9 tests), artefacts
  (2), invitations (3), outils (5) et quotas (2) passés dans une première photo.
  Deux régressions de reprise/fixture ont été corrigées ; nouvelle recette globale
  requise avec les tests ajoutés et les dernières migrations.
- Sauvegarde/restauration locale réussie sur 56 tables et 132 politiques ; cette
  photo précède les dernières colonnes/politiques de reprise et d'effacement.
- Supabase Auth réel local : création d'identité éphémère, magic-link/OTP,
  sélection de société, rôle viewer, refus des mutations et des accès forgés.
  Aucune délivrabilité SMTP externe prouvée.
- Navigateur avec vrai serveur et PostgreSQL : parcours Produit → contexte →
  plan technique/couverture/historique passé. Nouveau parcours artefact/graphe
  en reprise après correction d'une URL de test ; aucun défaut de rendu déduit
  de ce seul échec.
- Deux images construites localement via Podman, sans publication. Elles devront
  être reconstruites sur le candidat final et configurées pour la cible Auth.

## Ce qui reste ouvert

1. Terminer la recette intégrée et la revue corrective sur un même état de code,
   puis commit, PR et CI du candidat exact (CC-G1).
2. Compte/modèle IA et budget explicitement autorisés ; destinations de
   qualification Notion/Linear/GitHub (CC-G2). Une question de budget est en attente.
3. Hébergement/domaine, configuration Auth/SMTP, secrets, restauration séparée,
   alertes et ouverture contrôlée sur la cible identifiée (CC-G3). Les noms/liens
   des ressources sont demandés sans leurs secrets.
4. Usage réel par le propriétaire et une première équipe, analyse des incidents
   et de l'utilité sur la durée prévue (CC-G4). Ce temps ne peut être simulé.

L'autonomie de développement reste active. Une ressource externe manquante ne
suspend pas les autres lots. Aucune production n'est annoncée sur la seule base
d'une compilation, d'un test synthétique ou d'une configuration écrite.

## Repères historiques

Le socle du 8 septembre contenait Alpha Context Proof, les connexions IA
personnelles et les deux projets fictifs de preuve. Ses résultats restent
consultables dans [le rapport du 8 septembre](specs/synthetic-context-cases/validation-2026-09-08.md)
et [la revue Alpha](specs/alpha-context-proof/review-2026-09-08.md).
La PR historique n°1 et ses CI antérieures ne qualifient pas cette V1 locale.

## Mise à jour par un agent

Lire AGENTS.md, la spécification active et les preuves ; vérifier branche,
HEAD, PR/CI et gates. Séparer logiciel local, comptes réels, publication et
exploitation. Préserver sources et secrets ; actualiser ce statut avec les
preuves du candidat effectivement livré.
