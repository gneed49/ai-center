# Qualification Company Context V1 — 23 septembre 2026

Statut : recette locale du jalon `30619bf` réussie après correction du sélecteur E2E ; CC-G1, G2, G3 et G4 restent ouverts pour les écarts produit et les qualifications externes.
Branche : `feat/company-context-v1`. Jalon logiciel : `30619bf3dd6882527bc92448612a4699dd0a0644`, poussé ; complément de sélecteur et rapport : `11bc60f8b759c5523c9ea7616cfb9511458eaf7c`.
Mise à jour le 24 septembre : les cinq contrôles de
[Desktop CI](https://github.com/gneed49/ai-center/actions/runs/35934774241)
et les trois de [OCI Build](https://github.com/gneed49/ai-center/actions/runs/35934774264)
réussissent pour ce dernier commit. Aucun déploiement n’en découle.

## Compléments qualifiés

Historique conversationnel borné et capturé durablement ; cinq formats
d’artefacts générés et éditables ; conversion des anciens livrables ; reprise
explicite des commandes après réponse perdue ; progression persistée du steward,
déduplication des paires, plafond indépendant d’appels et protection des baux.
L’attribution immuable des appels permet leur clôture sans contenu lorsque les
droits changent pendant leur exécution. Les plans et la revue détaillent les
limites : conversations non confirmées, sélection de contexte bornée et absence
de certification exhaustive des contradictions.

## Preuves locales

| Contrôle | Résultat et portée |
| --- | --- |
| Qualité, format, lint, Clippy et builds | Réussis. 164 unités web, 164 unités Rust, 3 contrats synthétiques et 78 contrôles Python ; journal privé `.run/company-context-quality-final.log`. Les tests ignorés de PostgreSQL sont exécutés séparément. |
| Secrets et diff | Scan local réussi ; aucun changement des sources historiques. Journaux privés hors dépôt. |
| Navigateurs avec doubles HTTP | 146/150 au premier passage Chromium desktop/compact et Firefox ; quatre échecs dus au mock manquant de `/api/company/steward`. Les quatre scénarios concernés passent ensuite, un worker, 16,2 s ; journal `.run/steward-progress-mock-targeted.log`. Aucun correctif produit pour ces quatre échecs. |
| Auth locale, deux vrais comptes | Société, invitation, projet partagé, viewer, promotion, révocation et rechargement refusé réussis dans Chromium sur Supabase jetable. Aucun SMTP externe : l’OTP est obtenu auprès d’Auth local. La phase `auth-browser` rejoint la recette par défaut. |
| Migration et rôle runtime | Baseline vers 18 migrations réussie ; schéma inventorié, droits runtime contrôlés. Les 71 scénarios PostgreSQL passent, dont 32 société, 5 artefacts, 4 invitations et 6 outils. |
| PostgreSQL : reprise et droits | Capture/reprise fidèle des conversations, clôture des appels lors des changements de rôle, protections de suppression et progression durable : tous réussis. La fixture d’identité ancienne est bornée et ne retient plus le verrou qui bloquait son propre appel. |
| PostgreSQL : steward | Plus de 48 paires distinctes, ancienne règle société face à un projet tardif parmi 132 sources, reprise après expiration et perte de rôle : réussis. Le bilan lit désormais les compteurs en i64, y compris pour un lecteur. Les 32 scénarios société passent en 56,98 s. |
| PostgreSQL : artefacts | 5/5 réussis après correction de la fixture d’expiration ; aucun changement de la règle d’expiration. Génération, exactitude des sources, conversion historique, archivage en cours d’appel et isolation couverts. |
| TLS / restauration / Auth | TLS : CA autorisée acceptée, CA inconnue et mauvais nom refusés. Restauration locale vérifiée sur 58 tables et 138 politiques ; Auth API et navigateur à deux comptes réussis. Journal `.run/company-context-final-integration-recheck.log`. |
| Navigateurs avec API/base réelles | **4/4 réussis en 39,4 s**, Chromium, aucun mock API. Génération → édition → validation → sources/graphe ; réponse perdue → reçu GET → même brouillon ; versions/export ; PM → relais → plan technique → conversion. Le sélecteur du champ Description utilisait un label DOM concaténé à son contenu : corrigé vers le nom accessible textbox, sans changement produit. Journal `.run/company-context-real-e2e-recheck.log`. |
| Mobile et accessibilité | Parcours tickets à 390 px sans débordement, axe réussi ; image locale `.run/company-30619bf-agent-tickets-mobile.png`. La présentation brute des sections et identifiants reste à simplifier dans le lot tickets. |

## Écart supplémentaire de parcours

Le transport actuel crée une issue par document de tickets. Il faut N issues
pour N tickets sélectionnés, avec leurs titres, critères, sources, reçus et liens
individuels. Le [contrat tickets distincts](../ticket-publication/spec.md) et son
[plan](../ticket-publication/plan.md) précisent la correction, les quotas et
l’avertissement nécessaire avant de recréer des tâches depuis une nouvelle
version. Ce travail reste ouvert et fait partie de CC-G1. Une revue complémentaire a aussi confirmé l’absence de rattachement ciblé de pages Notion/issues Linear préexistantes et leur absence du contexte interrogé par les agents. Le cycle publication/observation existant ne remplit pas ce besoin ; son complément est en conception, sans promesse de synchronisation exhaustive.

## Limites externes

Aucun appel IA facturé, publication Notion/Linear/GitHub réelle, délivrabilité
SMTP externe, déploiement ou pilote d’équipe n’est prouvé par cette recette.
Les demandes de budget et de cibles autorisées restent en attente. Les jalons
précédents et leurs CI vertes restent documentés dans le
[rapport du 22 septembre](candidate-validation-2026-09-22.md) ; ils ne qualifient
pas automatiquement ces nouveaux compléments.
