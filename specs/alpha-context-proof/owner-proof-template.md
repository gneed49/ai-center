# Registre vierge — Context Proof propriétaire

Ce modèle prépare G10. Il ne constate aucune utilisation et ne valide aucun
gate. Le propriétaire le remplit au fil de ses usages réels ; l'agent ne réalise
pas les E2E à sa place. Les résultats de tests et les exemples simulés restent
séparés des boucles comptées ici.

Références : [spécification](spec.md), [registre de validation](validation.md),
[protocole comparatif](evaluation/README.md) et
[connexion GitHub privée](../../docs/operations/github-app-connection.md).

## Ouvrir une période

Copier ce modèle dans un emplacement privé approuvé. Les exports de packs,
corpus, traces et PR privées restent dans cet emplacement ; le dépôt reçoit
seulement un bilan expurgé. Utiliser des pseudonymes et des références de preuve
opaques, sans contenu métier, e-mail, token, clé, lien de connexion ou secret.

Une case vide ou « à renseigner » signifie inconnu, jamais zéro, absence
d'incident ou réussite. Écrire les dates réellement observées en ISO 8601 avec
leur décalage UTC. Distinguer la date de l'événement de la date du relevé.

| Paramètre                                                           | Valeur à renseigner |
| ------------------------------------------------------------------- | ------------------- |
| Identifiant du registre et responsable                              |                     |
| Emplacement privé des preuves et droits d'accès                     |                     |
| Période prévue ; fuseau utilisé                                     |                     |
| Début réel ; fin réelle ; durée écoulée                             |                     |
| Commit de l'app et versions serveur/schéma                          |                     |
| Environnement et workspace pseudonymisé                             |                     |
| Accord d'utilisation des trois projets : référence privée et portée |                     |
| Rapport G9, comparaison mesurée et décision d'ouverture de G10      |                     |
| Règle de reformulation majeure arrêtée avant le premier relevé      |                     |

| Projet                         | Pseudonyme du projet | Référence privée de l'autorisation | Boucles associées |
| ------------------------------ | -------------------- | ---------------------------------- | ----------------- |
| AI Center                      |                      |                                    |                   |
| Projet approuvé supplémentaire |                      |                                    |                   |
| Projet approuvé supplémentaire |                      |                                    |                   |

## Définir une boucle comptable

Une boucle complète relie une intention réelle, ses connaissances confirmées,
le gate/FeatureBrief, le ContextPack courant et son handoff, un plan technique,
un export effectivement transmis à l'outil externe, une PR réellement créée
après cette transmission, puis son ExternalReference, au moins une preuve validée
humainement et la couverture actualisée. Une preuve candidate, une déclaration
du modèle ou un lien de PR seul ne suffit pas.

Une boucle incomplète ou interrompue reste dans le registre avec son motif ; elle
ne contribue pas au seuil de dix boucles complètes. Un replay, un retry, un
rechargement ou la consultation sur un second client ne crée pas une nouvelle
boucle. Ne pas compter deux fois la même intention et la même chaîne de preuve.

La création de la PR appartient à la personne ou à son outil externe autorisé.
AI Center observe GitHub en lecture seule. Une PR antérieure à l'export ou à la
transmission ne satisfait pas la preuve chronologique attendue pour G10.

## Index des usages

Ajouter des lignes si nécessaire. Les numéros sont des emplacements vierges,
pas des usages prévus ni accomplis. Conserver aussi les tentatives interrompues.

| Fiche | Projet pseudonyme | Début / fin réels | Client(s) et version(s) | État complet / incomplet / interrompu | Preuve privée / incident |
| ----- | ----------------- | ----------------- | ----------------------- | ------------------------------------- | ------------------------ |
| 01    |                   |                   |                         |                                       |                          |
| 02    |                   |                   |                         |                                       |                          |
| 03    |                   |                   |                         |                                       |                          |
| 04    |                   |                   |                         |                                       |                          |
| 05    |                   |                   |                         |                                       |                          |
| 06    |                   |                   |                         |                                       |                          |
| 07    |                   |                   |                         |                                       |                          |
| 08    |                   |                   |                         |                                       |                          |
| 09    |                   |                   |                         |                                       |                          |
| 10    |                   |                   |                         |                                       |                          |

## Fiche à dupliquer pour chaque boucle

### Identité et point de départ

| Champ                                                            | Relevé / référence privée |
| ---------------------------------------------------------------- | ------------------------- |
| Fiche, projet et workspace pseudonymisés                         |                           |
| Opérateur pseudonyme ; date du relevé                            |                           |
| Début réel du parcours ; objectif résumé sans contenu sensible   |                           |
| App/serveur : commit ; schéma/migrations                         |                           |
| Client : navigateur desktop ou Tauri Linux, version et OS        |                           |
| Connexion et fournisseur/modèle choisis, sans identifiant secret |                           |
| Sessions Produit et Tech ; connaissances et versions confirmées  |                           |
| Gate/FeatureBrief : état, identifiant et version                 |                           |

### Pack, export, transmission et PR

| Étape                                | Identifiant / état / empreinte                                               | Date réelle et origine de l'horodatage    | Référence de preuve privée |
| ------------------------------------ | ---------------------------------------------------------------------------- | ----------------------------------------- | -------------------------- |
| Compilation                          | Pack, version, `content_hash`, `source_graph_version`, `compiler_version`    | `compiled_at` serveur                     |                            |
| État à l'export et à la transmission | Statut observé ; contrôle attendu : `current` ; pack remplacé/stale éventuel |                                           |                            |
| Handoff et plan technique            | Handoff, session Tech, livrable/version, runs plan et couverture             |                                           |                            |
| Export JSON ou Markdown              | Format ; empreinte du fichier exporté                                        | Date d'export observée                    |                            |
| Transmission à l'outil externe       | Outil/destination autorisée pseudonymisés ; même pack/version/hash           | Date de transmission réellement constatée |                            |
| Création de la PR                    | Référence privée de la PR ; SHA complet de tête                              | Date de création fournie par GitHub       |                            |
| Vérification chronologique           | Export ≤ transmission < création PR ; écarts d'horloge éventuels             | Date de la vérification                   |                            |

Le `content_hash` identifie le contenu canonique du pack. L'empreinte du fichier
exporté identifie les octets du JSON ou Markdown : ces deux empreintes n'ont pas
à être égales. Conserver la correspondance entre fichier transmis et pack.
Le champ `transmission_confirmed` de l'application est une déclaration humaine ;
il ne fournit pas à lui seul une preuve de la date effective d'envoi. Si une
date manque ou si l'ordre reste ambigu, indiquer « chronologie non prouvée ».

### Référence, décision humaine et couverture

| Étape                                            | Relevé / référence privée                                                          |
| ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| ExternalReference importée                       | Identifiant, PR ciblée, statut de sync, date du relevé                             |
| Observation immuable                             | Identifiant, date d'observation, SHA observé, état des checks                      |
| Suivi du travail externe                         | Identifiants de l'enveloppe/exécution/artefact et lien avec le pack transmis       |
| Preuve candidate                                 | Identifiant, exigence et version ciblées, livrable/section                         |
| Revue humaine                                    | Décision réelle, pseudonyme du relecteur, date, motif et référence d'audit         |
| Preuve après revue                               | `valid`, `rejected`, `stale`, `unavailable` ou autre état réellement affiché       |
| Couverture avant / après                         | Exigences et versions concernées, `covered` / `partial` / `missing`, date          |
| Cohérence après mutation ou nouvelle observation | Pack/livrable/preuve invalidés ou blocage stale observé ; recompilation éventuelle |
| Fin de boucle                                    | Date réelle, état complet/incomplet/interrompu et motif                            |

Un check GitHub vert et une évaluation IA de couverture ne remplacent pas la
validation humaine de la preuve. Un rejet utile reste une observation de valeur,
mais ne doit pas être présenté comme une preuve validée. Documenter séparément
la clôture effective de la boucle et les exigences encore sans preuve.

### Reformulations, retries et incidents

| Relevé                         | Valeur / preuve privée                                                                            |
| ------------------------------ | ------------------------------------------------------------------------------------------------- |
| Reformulations observées       | Nombre, majeures/mineures selon la règle annoncée, étape et cause ; sans recopier le prompt       |
| Incident ou interruption       | Référence, gravité, moment, effet utilisateur et périmètre                                        |
| Commande et tentative          | Opération, référence opaque de commande/idempotence et request/run ID utile                       |
| Erreur                         | Code expurgé, date, état de l'app et des objets avant reprise                                     |
| Retry ou replay                | Date, même commande ou nouvelle commande explicite, justification                                 |
| Résultat après reprise/reload  | Intention conservée, absence de doublon, états des connaissances/pack/preuves réellement vérifiés |
| Correction éventuelle          | Commit, contrôles touchés, résultat et artefact du rejeu complet                                  |
| Réexécution sur l'autre client | Navigateur desktop/Tauri Linux, version, scénario et preuve ; ne pas recompter la boucle          |

Ne provoquer ni perte de données ni incident de production pour remplir une
case. Si un scénario n'a pas été joué, l'indiquer. Un P0 constaté suspend la
promotion : conserver sa trace, corriger et rejouer entièrement les validations
touchées avant de le clôturer.

## Bilan de la semaine propriétaire

| Critère                                                | Mesure réelle | Dénominateur / preuve                                                           | Décision motivée |
| ------------------------------------------------------ | ------------- | ------------------------------------------------------------------------------- | ---------------- |
| Durée observée d'au moins 7 jours                      |               | Dates de début/fin ; jours d'usage effectif séparés                             |                  |
| Au moins 10 boucles complètes distinctes               |               | Complètes / démarrées ; fiches associées                                        |                  |
| Répartition sur les 3 projets autorisés                |               | Nombre de boucles par projet                                                    |                  |
| PR créée après transmission pour chaque boucle comptée |               | Chronologies prouvées / boucles comptées                                        |                  |
| Navigateur desktop effectivement utilisé               |               | Versions, fiches et scénarios                                                   |                  |
| Tauri Linux effectivement utilisé                      |               | Versions, fiches et scénarios                                                   |                  |
| Retries sans duplication et reprise sans perte         |               | Scénarios réellement joués, incidents et limites                                |                  |
| P0 corrigés et validations touchées rejouées           |               | Liste des P0 ; preuves de clôture                                               |                  |
| Avantage mesuré face à « dump brut + chat »            |               | Rapport G9 et observations G10 ; ne pas substituer une impression au comparatif |                  |

Une période prévue de sept jours ne prouve pas sept jours écoulés ; dix lignes
remplies ne prouvent pas dix boucles complètes. Ne pas extrapoler un client testé
vers l'autre, ni vers Android/iOS, qui restent hors périmètre.

| Clôture                                                                         | À renseigner |
| ------------------------------------------------------------------------------- | ------------ |
| Constats classés `prouvé`, `à consolider`, `non prouvé`, `différé`              |              |
| Limitations et données manquantes                                               |              |
| Références de la mise à jour du README, de l'audit et du registre de validation |              |
| Décision motivée : G10 clos ou poursuite nécessaire                             |              |
| Décideur ; date réelle ; preuve de décision                                     |              |

Le passage à l'équipe reste interdit si les
[interdictions de promotion](validation.md#interdictions-de-promotion)
s'appliquent, notamment si l'avantage comparatif n'est pas démontré. Une décision
G10 ne vaut ni déploiement HTTPS, ni deux semaines d'alpha privée accomplies.
