# Fiabilité — constat et travail borné CC-T11

Date : 21 septembre 2026. Ce document distingue les mécanismes présents des
preuves locales et des gates externes, sans déclarer le ticket entièrement fini.

| Chemin                         | Présent dans le code                                                                                                                                                 | Manque ou limite                                                                                                                 |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Commandes HTTP                 | `idempotency.rs` : engagement du corps, bail de 30 s renouvelé par `with_lease`, réponse durable atomique, même clé en cas de réponse perdue                         | Toutes les opérations longues doivent utiliser l'enveloppe ; fermeture du navigateur ne prouve pas annulation d'un effet distant |
| Fournisseurs IA                | Requêtes structurées bornées, délais, nombre limité de retries typés ; tokens et coût optionnels stockés dans `model_runs`                                           | Pas encore de quota global société ni arrêt partagé démontré pour tous fournisseurs/personnels/jobs                              |
| Abonnement personnel           | `provider_subscriptions.rs` : quatre processus concurrents, groupe de processus détruit à annulation, oubli du compte annule les générations ; délai 120 s           | Ne remplace pas un arrêt global société ni un budget monétaire                                                                   |
| Publication externe            | Job SQL, version/cible/connexion capturées, tentative externe hors transaction, une seule création automatique ; expiration → `needs_review`, jamais requeue aveugle | Échec définitif demande une nouvelle version/publication ; aucun retry automatique distant, aucune mise à jour d'objet externe   |
| Lecture/refresh/réconciliation | Appels bornés, idempotence et bail renouvelé pendant requête, observations immuables, permissions recontrôlées                                                       | Lecture synchrone explicite ; pas encore de job de refresh autonome durable                                                      |
| Arrêt publication              | Annulation d'un `queued` atomique contre claim ; connexion désactivée/révisée ou acteur révoqué empêche démarrage                                                    | Un appel déjà envoyé peut avoir créé un objet ; interface impose vérification, aucun faux « annulé »                             |
| Admission et usage outils      | Quotas société persistants protégés par verrou SQL commun aux replicas ; usage public sans secret et coût `null`                                                     | Compteurs d'admission, pas facture fournisseur ; pas limitation de concurrence inter-replicas supplémentaire à la taille de file |

Les quotas outils valent par défaut 25 publications en attente/traitement,
100 demandes de publication par heure et 120 opérations de lecture par heure.
Une lecture Notion peut demander deux requêtes HTTP. Les variables serveur
`AI_CENTER_WORK_TOOLS_MAX_PENDING` (1–100),
`AI_CENTER_WORK_TOOLS_PUBLICATIONS_PER_HOUR` (1–1 000) et
`AI_CENTER_WORK_TOOLS_READS_PER_HOUR` (1–1 200) permettent de réduire ou augmenter
ces limites dans les bornes. Une valeur invalide bloque l'admission ; aucun zéro
ne supprime la protection. Tous les replicas doivent partager cette configuration.

`GET /api/work-tools` annonce limites et usage observés. Le dépassement renvoie
`capacity_exceeded` / HTTP 429 ; seule une erreur explicitement marquée retryable
peut reprendre la même commande. Ni cette réponse ni l'absence de coût mesuré
ne signifient coût nul.

Preuves locales acquises à cette étape : 5 tests unité/HTTP simulé work_tools ;
Clippy lib + acceptances work_tools/artifacts sans warning ; compilation des
acceptances PostgreSQL. Le coordinateur consigne séparément leur exécution sur
stack isolée. Aucun compte distant ni publication réelle utilisé par ce lot.

Travail restant CC-T11 hors module outils : admission IA persistante société,
arrêt couvrant API IA + CLI personnels + steward, limites et usage IA exposés
dans l'interface, reprise des opérations longues et état de saisie après reload,
puis preuve intégrée derrière le proxy cible. Le live IA et la durée du pilote
restent des gates distincts.
