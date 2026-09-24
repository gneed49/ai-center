# Dossier opérateur — préparation de l'alpha privée

Ce dossier prépare G11 : une surface web privée sous HTTPS, un seul workspace,
2 à 3 participants invités et 14 jours d'usage observé. Il ne choisit aucun
hébergeur, ne déploie rien et ne constitue ni une invitation, ni un consentement,
ni une preuve d'exploitation. Les E2E et essais réels appartiennent au
propriétaire. Les champs restent vides jusqu'à leur renseignement effectif.

Sources : [contrat Alpha Context Proof](../../specs/alpha-context-proof/spec.md),
[validation et critères de promotion](../../specs/alpha-context-proof/validation.md),
[registre propriétaire](../../specs/alpha-context-proof/owner-proof-template.md).
Les anciens résultats de CI et de smoke ne certifient pas l'environnement à
renseigner ici. Classer chaque constat comme `prouvé`, `à consolider`,
`non prouvé` ou `différé`, avec une preuve datée et un périmètre précis.

## 1. Décision d'ouverture et paramètres privés

Conserver le dossier rempli dans un espace privé autorisé. Le bilan versionné
utilise des pseudonymes et des références opaques. Les adresses des participants,
URLs privées détaillées, contenus de packs, credentials, certificats privés et
liens de connexion ne vont pas dans Git, les tickets ou les alertes.

| Paramètre                                                                 | Valeur / référence privée à renseigner |
| ------------------------------------------------------------------------- | -------------------------------------- |
| Responsable du déploiement ; responsable des données ; suppléant          |                                        |
| Rapport G9 et avantage comparatif mesuré                                  |                                        |
| Bilan G10 : ≥10 boucles, trois projets, ≥7 jours ; décision de passage    |                                        |
| P0 ouverts/clos ; preuve des validations touchées rejouées                |                                        |
| Commit app/API/web ; digests OCI ; version du schéma et migrations        |                                        |
| Hébergement et région retenus par le propriétaire                         |                                        |
| Domaine HTTPS et propriétaire DNS/TLS ; renouvellement prévu              |                                        |
| Environnement Supabase dédié, périmètre de données autorisé               |                                        |
| Workspace unique pseudonymisé et propriétaire                             |                                        |
| Stockage privé des journaux/rapports ; accès ; rétention                  |                                        |
| Période prévue ; début et fin réellement observés ; fuseau                |                                        |
| Budget autorisé et responsable ; référence de l'autorisation, sans secret |                                        |
| Procédure de suspension et interlocuteur joignable                        |                                        |

L'ouverture reste soumise aux interdictions de promotion du registre, notamment
l'isolation, la conservation des connaissances, l'absence de doublon après
retry, l'absence de source inventée, le blocage stale, GitHub read-only, les
secrets, la SSRF, la restauration vérifiée et l'avantage comparatif. Une case
manquante est une preuve manquante ; ce dossier ne lève aucun de ces gates.

## 2. Surface HTTPS, Supabase et secrets

Les [images OCI](oci-images.md) et le
[proxy nginx versionné](../../deploy/oci/nginx/default.conf.template) sont des
moyens de livraison portables. Nginx écoute actuellement en HTTP interne sur
8080 ; sa présence ne fournit pas la terminaison HTTPS publique. L'opérateur
doit documenter cette terminaison, son renouvellement, le réseau privé vers
Axum et PostgreSQL, et leurs contrôles d'accès sur la cible choisie.

| Élément à préparer             | Contrat existant / décision attendue                                                                                         | Preuve réelle et état |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| Origine web HTTPS et proxy     | Domaine choisi, terminaison TLS, HTTP redirigé/refusé selon politique retenue, accès interne à l'API                         |                       |
| API centrale                   | `AI_CENTER_AUTH_MODE=supabase` ; mode `local` réservé au loopback ; bind interne selon la cible                              |                       |
| Origine et CORS                | `/api` sur la même origine via `AI_CENTER_API_UPSTREAM` ; `AI_CENTER_CORS_ORIGINS` limité aux origines réellement autorisées |                       |
| Auth Supabase                  | `SUPABASE_URL` côté serveur ; URL du site et callback HTTPS `/auth/callback` configurés sur la cible                         |                       |
| Configuration cliente publique | `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY` ; jamais de clé service-role ni de clé fournisseur dans `VITE_*`               |                       |
| PostgreSQL                     | `DATABASE_URL` privé sous `ai_center_runtime`, séparé de l'accès migration/administration                                    |                       |
| Schéma et serveur              | Migrations relues et livraison conjointe sur l'environnement identifié ; sauvegarde et chemin de reprise préparés            |                       |
| SMTP/magic link                | Expéditeur, service, quotas et délivrabilité vérifiés par l'opérateur ; aucune valeur choisie dans ce modèle                 |                       |
| Secrets applicatifs            | Injection au runtime par le gestionnaire choisi, rotation et personnes autorisées ; aucune valeur dans l'image               |                       |
| Connexions IA personnelles     | `AI_CENTER_CREDENTIAL_ENCRYPTION_KEY` privé si le stockage est activé ; sauvegarde séparée de la base chiffrée               |                       |
| Client Tauri Linux optionnel   | Même API centrale et même workspace ; origine cliente et authentification réellement vérifiées                               |                       |

Consulter le [contrat du rôle runtime](postgres-runtime-role.md), les
[réglages IA et leur chiffrement](../../specs/provider-connections/backend.md)
et les [limites d'abonnement](../../specs/provider-connections/subscriptions.md).
Les abonnements locaux Claude ne sont pas un mode d'authentification à activer
sur le serveur partagé. La posture du rôle et les politiques du schéma actuel
font foi ; ne pas déduire les droits d'une ancienne matrice documentaire seule.

Préparer et faire relire une procédure de migration propre à la cible avant
exécution. Ce dossier ne contient pas de reset ni de restauration sur une base
utilisateur. Les scripts de smoke locaux ne doivent pas être transposés contre
la base de l'alpha hébergée.

## 3. Invitations et accès au workspace

La [page de connexion](../../apps/web/src/auth/auth-pages.tsx) demande un magic
link avec `shouldCreateUser: false`. Recevoir un lien ne crée pas une adhésion
au workspace. Le [schéma](../../supabase/schemas/01_domain.sql) exige une adhésion
acceptée avec un rôle `owner`, `editor` ou `viewer` ; l'API applique ce scope.

L'opérateur prépare une procédure d'administration autorisée pour créer les
comptes nécessaires et les adhésions, accepte uniquement les personnes prévues,
et conserve les preuves de leurs accords hors dépôt. Ce document n'ajoute pas
d'interface de gestion des invitations ni d'automatisation de provisionnement.

| Participant pseudonyme | Accord/portée et référence privée | Invitation réelle / date | Adhésion et rôle vérifiés / date | Première utilisation réelle | Retrait/révocation si applicable |
| ---------------------- | --------------------------------- | ------------------------ | -------------------------------- | --------------------------- | -------------------------------- |
|                        |                                   |                          |                                  |                             |                                  |
|                        |                                   |                          |                                  |                             |                                  |
|                        |                                   |                          |                                  |                             |                                  |

Avant ouverture générale aux participants prévus, le propriétaire vérifie le
callback, l'absence d'inscription libre, le refus d'un non-membre et les droits
des rôles utilisés. Relever le scénario et sa preuve ; une configuration Auth
ou une ligne `accepted` ne prouve pas que le participant a accepté l'usage du
corpus ou effectivement utilisé l'application. Ne pas envoyer d'invitations
depuis ce modèle.

## 4. Activation progressive des fournisseurs

| Étape                 | Mécanisme disponible                                                                                                                  | Activation réelle / responsable / preuve | Retour arrière préparé |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ---------------------- |
| Sans fournisseur réel | Mode déterministe, données autorisées et environnement privé                                                                          |                                          |                        |
| OpenAI                | `AI_CENTER_AGENT_MODE=openai`, modèle explicite et secret serveur privé pour le défaut, ou profil personnel explicitement sélectionné |                                          |                        |
| GitHub read-only      | GitHub App privée sur dépôts autorisés ; connexion `pending`, puis `active` après vérification réelle                                 |                                          |                        |

Le mode du serveur choisit son défaut : **il ne désactive pas les connexions IA
personnelles déjà sélectionnées**. Le dépôt n'apporte pas un service général de
feature flags permettant de certifier un arrêt global de tous les fournisseurs.
Renseigner la politique d'activation et de suspension effective des profils et
des comptes avant d'annoncer cette garantie ; ne pas inventer de variable de
configuration. Un fournisseur absent, une erreur ou une suspension ne doit pas
entraîner de substitution silencieuse.

Le [provisionnement GitHub](github-app-connection.md) décrit le passage
`pending` → `active`. Conserver les secrets hors dépôt, l'installation limitée
aux dépôts approuvés et les permissions en lecture seule. Un faux HTTP vert ne
prouve pas l'accès réel. La création de PR reste extérieure à AI Center.

## 5. Relevé des métriques : sources et limites

Préparer un relevé périodique en lecture seule, filtré sur l'environnement, le
workspace, les projets autorisés et la période. Utiliser un accès approuvé qui
respecte la RLS ; ne pas ouvrir le schéma privé à la Data API ni désactiver la
RLS pour produire un tableau. Ne relever que les identifiants pseudonymisés,
horodatages, statuts, classes d'erreur et compteurs nécessaires.

Les champs cités ci-dessous existent dans le
[schéma applicatif](../../supabase/schemas/01_domain.sql). Leur collecte agrégée,
sa planification et sa conservation ne sont pas automatisées par ce dossier.
Un champ nullable absent reste `inconnu`. Afficher le nombre de données
manquantes, le dénominateur et les exclusions à côté de chaque mesure.

| Mesure demandée             | Définition du relevé                                                                                                          | Source existante                                                                                                                                                            | Inconnu ou saisie humaine nécessaire                                                                                                                                       |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Temps jusqu'au premier pack | Premier `compiled_at` valide moins début réel du parcours ; médiane et nombre de parcours complets observés                   | `context_packs.compiled_at`, `project_id`, `status` ; `projects.created_at` seulement pour une mesure distincte « depuis création du projet »                               | Début réel du parcours non instrumenté ; ne pas remplacer silencieusement par l'heure de création d'un projet ancien                                                       |
| Reformulations              | Nombre par boucle et part des boucles avec reformulation majeure, selon définition préalable                                  | Registre propriétaire/participant ; identifiants de sessions et messages comme repères                                                                                      | Pas de compteur sémantique fiable ; le nombre de messages ne mesure pas les reformulations                                                                                 |
| Packs recompilés            | Packs distincts créés sur la période avec `supersedes_context_pack_id` renseigné ; rapporter aussi total de packs et motifs   | `context_packs` : filiation, `compiled_at`, `invalidated_at`, `stale_reason` ; audit `context_pack.compiled`                                                                | Un refresh d'écran ne compte pas ; raison produit et utilité à qualifier humainement                                                                                       |
| Contradictions utiles       | Constats jugés utiles par un humain / constats effectivement revus ; inclure ambiguïtés et faux positifs                      | `steward_assessments`, `insights`, `insight_resolutions`, versions de connaissances liées                                                                                   | `resolved` ou `accepted` ne signifie pas utile ; jugement, date, évaluateur et mutation réelle à relever                                                                   |
| Preuves validées            | Nombre d'objets distincts revus `valid` pendant la période ; séparer ceux encore valides à sa fin                             | `audit_events.action='evidence.validated'`, `object_public_id`, `occurred_at` ; `evidences.status`, versions d'exigence ; `requirement_coverage`                            | `evidences.created_at` n'est pas une date de validation ; candidat, checks verts et score IA ne valent pas validation humaine                                              |
| Succès de sync              | Commandes logiques d'import/refresh réussies / commandes observées, avec erreurs, limites et inconnus séparés                 | `idempotency_records` des opérations GitHub, statuts HTTP ; `external_references.sync_status`, `last_synced_at`, `last_error_code` ; observations et événements de suivi    | Dernier état d'une référence ≠ historique des tentatives ; même commande rejouée à dédupliquer ; snapshots nécessaires pour connaître les échecs avant une reprise réussie |
| Erreurs et retries          | Séries séparées par surface/opération et classe : runs, commandes, outbox ; taux seulement avec dénominateur explicite        | `model_runs.status/error_class/attempt_count/latency_ms`, `idempotency_records.status/error_code`, `domain_events.status/attempt_count/available_at/locked_until/failed_at` | Ne pas sommer les mêmes incidents présents dans plusieurs tables ; pas de compteur complet des erreurs UI ou de toutes les requêtes HTTP fourni ici                        |
| Coûts                       | Somme des coûts connus par fournisseur/modèle, compte des runs sans coût, puis rapprochement avec relevé fournisseur autorisé | `model_runs.estimated_cost`, `input_tokens`, `output_tokens`, `usage`, fournisseur/modèle ; justificatif fournisseur privé                                                  | Valeur absente ≠ zéro ; pas de total certifié si incomplet ; tarif daté et devise à renseigner, coût estimé distinct du facturé et de l'abonnement                         |
| Complétion des parcours     | Boucles complètes / boucles réellement démarrées ; durées, abandons, causes et client séparés                                 | Fiches de boucle, handoffs, livrables, références, preuves et couverture liées                                                                                              | Aucun compteur d'abandon/navigation exhaustif ; `sessions.status` seul ne prouve pas une boucle complète                                                                   |

Les opérations GitHub utilisent actuellement les clés
`external_reference.github_pr.create` et `external_reference.github_pr.refresh`,
y compris pour les types de référence pris en charge ; relever le type réel.
Les [observations](../../apps/server/src/external_references.rs) et le
[suivi externe](../../apps/server/src/external_references/tracking.rs) ne doivent
pas être exportés intégralement : leurs données peuvent identifier le dépôt.

### Feuille périodique à dupliquer

| Relevé                                                             | À renseigner |
| ------------------------------------------------------------------ | ------------ |
| Fenêtre réelle de mesure ; instant d'extraction ; fuseau           |              |
| Environnement, workspace/projets et clients inclus ; commit        |              |
| Personne responsable ; méthode/requête en lecture seule et version |              |
| Métrique ; numérateur ; dénominateur ; valeur ; unité/devise       |              |
| Sources/artefacts privés ; nombre d'inconnus ; exclusions          |              |
| Écart au relevé précédent ; incident ou changement de version lié  |              |
| Classification du constat et limite d'interprétation               |              |

## 6. Préparer les alertes sans contenu

Les routes `/healthz` du web et `/api/health` de l'API/connexion DB existent.
Elles ne certifient ni le parcours Auth, ni les fournisseurs, ni la disponibilité
des sauvegardes. Les journaux HTTP et les classes d'erreur sont des sources ;
un système de collecte, de seuils et d'envoi d'alertes reste à configurer sur
l'infrastructure retenue. Aucun canal ni seuil opérationnel n'est fixé ici.

| Signal                                                   | Source possible                                                     | Seuil/fenêtre et fréquence à décider | Destinataire/astreinte, action et preuve d'essai |
| -------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------ | ------------------------------------------------ |
| Web/API indisponible, latence anormale                   | Healthchecks et métriques de la cible                               |                                      |                                                  |
| Certificat proche de l'expiration/renouvellement échoué  | Terminaison TLS choisie                                             |                                      |                                                  |
| Erreurs Auth, refus inter-scope ou événement de sécurité | Codes HTTP agrégés et incident confirmé, sans identité personnelle  |                                      |                                                  |
| Échecs fournisseur, quota/limite, sortie hors contrat    | Classes de runs et commandes                                        |                                      |                                                  |
| Outbox bloquée ou `dead_letter`                          | Statut, échéance et âge des événements                              |                                      |                                                  |
| Échecs ou accès GitHub indisponible                      | Commandes, classes de sync et observations                          |                                      |                                                  |
| Sauvegarde absente/échouée, restauration en retard       | Relevés du mécanisme de sauvegarde choisi                           |                                      |                                                  |
| Dépense proche de la limite autorisée                    | Compteurs rapprochés et relevé fournisseur, disponibilité à prouver |                                      |                                                  |

Préparer un essai de chaque règle avec des métadonnées synthétiques sur une
cible isolée, puis conserver le résultat réel lors de son exécution autorisée.
Une alerte contient au plus l'environnement pseudonymisé, la fenêtre, le signal,
la gravité, des compteurs et une référence opaque d'incident. Exclure prompts,
outputs, corps HTTP, URLs avec paramètres, adresses e-mail, tokens et dumps de
`before_state`/`after_state`. Définir accès et rétention des journaux séparément.

## 7. Sauvegardes automatiques et restauration séparée

Le [smoke PostgreSQL](postgres-backup-restore.md) restaure le domaine `app` dans
une base temporaire du même cluster local. Il ne planifie pas les sauvegardes
de l'alpha et ne démontre pas une reprise sur infrastructure séparée.

| Politique à arrêter avant ouverture                                                                   | Valeur / référence privée |
| ----------------------------------------------------------------------------------------------------- | ------------------------- |
| Responsable, suppléant et procédure d'accès aux sauvegardes                                           |                           |
| Mécanisme automatique, fréquence et fenêtre ; preuve de programmation                                 |                           |
| Périmètre : domaine `app`, Auth/identités, rôles/politiques, configuration et dépendances nécessaires |                           |
| Données ou fichiers non couverts ; procédure complémentaire                                           |                           |
| Stockage distinct, chiffrement, accès et rétention/destruction                                        |                           |
| Sauvegarde séparée de la clé de chiffrement des connexions ; rotation                                 |                           |
| Perte maximale acceptable (RPO) et durée de reprise cible (RTO)                                       |                           |
| Fréquence des exercices ; environnement de restauration séparé                                        |                           |
| Alertes d'échec/retard et responsable de leur traitement                                              |                           |

Préparer la procédure suivante pour la cible choisie, avec un responsable pour
chaque étape. Son existence ne prouve aucune sauvegarde ni restauration :

1. Identifier la sauvegarde, son instant, son périmètre et sa preuve d'intégrité.
   Vérifier qu'elle est lisible et que les secrets indispensables sont
   récupérables par une personne autorisée, sans les copier dans le rapport.
2. Provisionner une infrastructure de reprise **séparée** de l'environnement
   utilisateur et vérifier son identité avant toute restauration. Bloquer
   sorties fournisseur/GitHub, SMTP et traitements externes pendant l'exercice.
3. Restaurer dans cette cible vide selon la procédure de son infrastructure,
   sans écraser la base de l'alpha et sans affaiblir la RLS pour faire réussir
   le contrôle. Documenter Auth, rôles et configuration en plus du domaine `app`.
4. Comparer inventaires, volumes, versions, contraintes, grants/RLS et empreintes
   appropriées. Vérifier les connaissances confirmées, les packs et leurs
   lignées, références/preuves/audits et clés chiffrées récupérables. Les contrôles
   applicatifs autorisés et E2E restent à exécuter par le propriétaire.
5. Mesurer durée et point récupéré, calculer le RPO/RTO réellement obtenu,
   consigner écarts et décision. Faire approuver toute bascule réelle dans un
   acte distinct ; cet exercice ne bascule aucun trafic.
6. Décider le devenir de la cible de reprise et des copies selon la rétention
   approuvée. Documenter son nettoyage ciblé lorsque réalisé ; ne jamais
   transposer une commande destructive du smoke vers une base utilisateur.

| Exercice de restauration                                        | À renseigner après exécution |
| --------------------------------------------------------------- | ---------------------------- |
| Sauvegarde, origine et cible séparée identifiées ; autorisation |                              |
| Début/fin réels ; durée ; point récupéré ; RPO/RTO obtenus      |                              |
| Contrôles exacts et preuves privées ; écarts                    |                              |
| Décision de restaurabilité ; responsable et date réelle         |                              |
| Réutilisation ou nettoyage de la cible, réellement constaté     |                              |

## 8. Incidents et bilan des 14 jours

Tenir un journal privé avec date réelle, gravité, environnement/versions, effet
utilisateur, périmètre, action et preuve. Relier les incidents aux relevés sans
copier le contenu des données. En cas de rupture de confiance ou P0, suspendre
la promotion, préserver les faits, corriger et rejouer les validations touchées.
L'absence de ticket ouvert n'est pas une preuve d'absence d'incident.

| Bilan                                                                   | Mesure / référence réelle | Classification et limite |
| ----------------------------------------------------------------------- | ------------------------- | ------------------------ |
| Participants invités, accords et adhésions vérifiés                     |                           |                          |
| 2 à 3 participants ayant effectivement utilisé l'alpha                  |                           |                          |
| Début/fin et ≥14 jours écoulés ; jours d'activité distincts             |                           |                          |
| Un workspace contrôlé ; web desktop ; Tauri Linux si réellement utilisé |                           |                          |
| Temps jusqu'au premier pack et complétion des parcours                  |                           |                          |
| Reformulations, recompilations et contradictions utiles                 |                           |                          |
| Preuves validées et succès de sync                                      |                           |                          |
| Erreurs, reprises, P0 et effets sur la confiance                        |                           |                          |
| Coûts connus, inconnus et rapprochement fournisseur                     |                           |                          |
| Alertes configurées, essais et incidents effectivement détectés         |                           |                          |
| Sauvegardes automatiques effectuées et restauration séparée mesurée     |                           |                          |
| Limitations, données manquantes et validation des participants          |                           |                          |

Une invitation envoyée ne vaut pas un participant actif ; quatorze lignes de
journal ne valent pas quatorze jours écoulés. Ne pas déduire un taux à partir
d'un dénominateur absent, ni un avantage produit d'une compilation réussie.

Après ce bilan seulement, préparer les candidats V1 dans cet ordre : rupture
de confiance/sécurité, rupture d'un parcours contextuel, gain mesuré de valeur,
intégration externe suivante, puis confort/extension. Pour chaque candidat,
indiquer la preuve, l'impact, le classement et ce qui reste inconnu. Les autres
connecteurs, vues inter-projets, tâches et fonctions natives de production ne
sont pas présupposés. Le tag/release exige une décision distincte avec les
limites publiées du jalon ; ce dossier n'en autorise pas la création.

| Décision finale                                                 | À renseigner |
| --------------------------------------------------------------- | ------------ |
| G11 clos, prolongation ou suspension ; motifs                   |              |
| Décideur ; date réelle ; rapport expurgé et preuves privées     |              |
| Limites publiables et références du plan V1 préparé après bilan |              |
