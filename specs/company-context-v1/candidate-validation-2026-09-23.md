# Qualification Company Context V1 — 23 septembre 2026

Statut : recette corrective en cours ; CC-G1, G2, G3 et G4 restent ouverts.
Branche : `feat/company-context-v1`. Dernier jalon poussé : `f5c5a28`.
Le commit de ce document identifie le complément local ; sa recette corrective
et sa CI doivent être rattachées avant de le qualifier intégralement.

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
| Migration et rôle runtime | Baseline vers 18 migrations réussie ; schéma inventorié, droits runtime contrôlés. L’intégration complète du candidat est encore en cours. |
| PostgreSQL : reprise de conversation | Le premier scénario de capture passe. Le scénario d’identité ancienne retenait une transaction dont le verrou FK bloquait son propre appel. Fixture corrigée : réservation d’identité puis insertion tardive, délais bornés ; exécution DB corrigée à vérifier. Ce défaut de test ne vaut pas une preuve de reprise réussie. |
| PostgreSQL : retrait de droits / suppression | Scénarios de changement de rôle pendant chat/artefact/steward, garde d’effacement pendant analyse et suppression des seuls reçus projet réussis dans la recette en cours. |
| PostgreSQL : progression du steward | Reprise de bail, ancien worker et fichiers distincts d’un même corpus passent. Les assertions au-delà de 48 paires et de 128 sources passent avant un échec de lecture du bilan : les compteurs COUNT/INT8 étaient lus en i32. Correctif i64 appliqué ; le test du membre révoqué accepte les deux refus sûrs Forbidden/NotFound. Clippy ciblé passe, relance DB en cours. |
| PostgreSQL : artefacts | 4/5 au premier passage ; le scénario d’expiration simulée violait lui-même la contrainte expires_at > created_at. Création/expiration de la fixture corrigées, assertions de refus et d’absence d’appel supplémentaire conservées ; relance en cours. |
| Navigateurs avec API/base réelles | Conversion du plan existant réussie au passage précédent. Génération/reprise de brouillon en attente de recette commune stabilisée ; les deux échecs précédents ne sont pas clos par des tests unitaires. |

## Écart supplémentaire de parcours

Le transport actuel crée une issue par document de tickets. Il faut N issues
pour N tickets sélectionnés, avec leurs titres, critères, sources, reçus et liens
individuels. Le [contrat tickets distincts](../ticket-publication/spec.md) et son
[plan](../ticket-publication/plan.md) précisent la correction, les quotas et
l’avertissement nécessaire avant de recréer des tâches depuis une nouvelle
version. Ce travail reste ouvert et fait partie de CC-G1.

## Limites externes

Aucun appel IA facturé, publication Notion/Linear/GitHub réelle, délivrabilité
SMTP externe, déploiement ou pilote d’équipe n’est prouvé par cette recette.
Les demandes de budget et de cibles autorisées restent en attente. Les jalons
précédents et leurs CI vertes restent documentés dans le
[rapport du 22 septembre](candidate-validation-2026-09-22.md) ; ils ne qualifient
pas automatiquement ces nouveaux compléments.
