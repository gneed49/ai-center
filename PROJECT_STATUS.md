---
project: AI Center
status_schema: 2
last_reviewed: 2026-09-23
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

Le développement utilise le checkout indiqué en métadonnées ; l'ancien checkout
est préservé. Les jalons `9be97b6`, `596cfee` et `f5c5a28` sont commités et poussés dans la
[PR de travail n°3](https://github.com/gneed49/ai-center/pull/3), avec leurs huit
contrôles CI distincts réussis. Aucun déploiement de cette V1 ni appel facturé
ou qualification de destinations Notion/Linear/GitHub réelles n'est déclaré.

## Écarts logiciels prioritaires du 23 septembre

La [revue de continuité du contexte](specs/company-context-v1/acceptance-review-2026-09-23.md)
a trouvé trois P1 malgré la CI verte : absence d'historique conversationnel dans
l'entrée modèle, circuit séparé entre génération et artefacts publiables, et
absence de progression du steward au-delà de ses premières sources/paires.
Les trois corrections sont livrées dans `30619bf` ; la recette locale est réussie après une correction de sélecteur E2E. La revue a aussi renforcé les reprises après réponse perdue,
l’attribution des appels IA lors d’un changement de rôle et le verrouillage
du steward. La revue du parcours vers les outils a aussi relevé un quatrième
écart : un document de plusieurs tickets devient encore une seule issue
Linear/GitHub. Le [lot tickets distincts](specs/ticket-publication/spec.md)
prévoit une publication et un lien par ticket, sans nouveau tableau de tâches.
Le rattachement ciblé du contexte Notion/Linear préexistant et son utilisation par les agents restent aussi à réaliser. G1 reste ouvert jusqu’à réalisation de ces écarts et qualification du même commit.

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

- **Dernier candidat poussé : `f5c5a28`.** Huit contrôles CI distincts réussis,
  157 tests web, 155 unités Rust, 21 scénarios société, deux parcours navigateur
  API/DB réelles et 150 scénarios navigateur avec doubles HTTP. La bibliothèque
  de connaissances et les sources exactes du graphe font partie de ce commit.
  Le [rapport du 22 septembre](specs/company-context-v1/candidate-validation-2026-09-22.md)
  conserve la portée détaillée et les preuves des jalons précédents.
- **Jalon du 23 septembre : `30619bf`, poussé ; CI en cours.** Qualité :
  164 tests web, 164 unités Rust, trois contrats synthétiques, 78 contrôles Python,
  format/lint/Clippy et builds. PostgreSQL : 71 scénarios réussis, dont 32 société
  et cinq artefacts ; TLS, restauration de 58 tables/138 politiques et Auth API
  réussis. [Preuves et corrections](specs/company-context-v1/candidate-validation-2026-09-23.md).
- **Deux comptes Auth dans Chromium : réussi sur base jetable.** Société,
  invitation, projet partagé, lecture seule, promotion, retrait et ancien lien
  refusé ; rechargement montrant le refus. OTP obtenu auprès d’Auth local,
  aucune délivrabilité SMTP externe prouvée. Cette recette entre dans la CI.
- **Parcours web : quatre sur API/DB réelles réussis en 39,4 s.** Génération,
  perte de réponse et reprise, édition/validation, sources exactes/graphe,
  conversion historique et relais PM/technique. Le dernier correctif ne change
  qu’un sélecteur de test. Sur la matrice avec doubles HTTP, 146/150 ont passé,
  puis les quatre échecs de fixture ont passé après correction sur les tailles
  Chromium concernées. La présentation des tickets sera simplifiée dans T16.

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
