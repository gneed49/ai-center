# AI Center — état courant Alpha Context Proof

> Revue du 8 septembre 2026. Sources logicielles intégrées à `b20d36f`.
>
> Branche locale : `feat/alpha-context-proof` dans le checkout du projet ChatGPT.
>
> [PR #1](https://github.com/gneed49/ai-center/pull/1),
> [issue de consolidation #2](https://github.com/gneed49/ai-center/issues/2).

Les connexions IA personnelles du 7 septembre et les corrections ACP-T11 à
ACP-T13 du 8 septembre sont intégrées localement. Les E2E et essais avec des
comptes réels restent à effectuer par le propriétaire, conformément à sa
consigne du 7 septembre. Aucun navigateur, client natif ou fournisseur réel
n'a été lancé pour ces deux livraisons.

## Méthode de preuve

| Statut                       | Signification                                                  |
| ---------------------------- | -------------------------------------------------------------- |
| `Vérifié`                    | Reproduit sur la version et dans l'environnement indiqués.     |
| `Présent mais non reproduit` | Code ou artefact identifiable, parcours complet non exécuté.   |
| `Déclaré`                    | Résultat historique sans reproduction sur la version courante. |
| `Futur`                      | Résultat ou exploitation encore à réaliser.                    |

Une intégration PostgreSQL avec faux fournisseur ne prouve pas une génération
réelle ni la qualité du contexte. Une compilation ne prouve pas le parcours
natif. Un résultat local n'est ni une CI distante ni un déploiement.

Le 8 septembre, le propriétaire a demandé à l'agent d'inventer et tester
[deux projets](../specs/synthetic-context-cases/spec.md). Leur validation
logicielle n'attend donc plus de corpus réel fourni par le propriétaire.
Les gates ci-dessous décrivent les preuves opérationnelles du jalon Alpha ;
elles ne sont pas des prérequis pour exécuter ces tests synthétiques.

## Comportements et preuves récentes

| Domaine                          | Résultat livré et preuve datée                                                                                                                                                    | Limite                                                                                                   |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Connexions IA                    | 7 septembre : profils privés OpenAI, Anthropic, Kimi, DeepSeek et OpenRouter ; chiffrement serveur, identité acteur/workspace, choix persistant et moteur commun.                 | Aucune génération réelle certifiée ; catalogue et qualité restent distincts.                             |
| Abonnement                       | 7 septembre : client officiel Claude Linux 2.1.220, comptes personnels Pro/Max admissibles, authentification séparée de l'éligibilité, récupération et logout.                    | Clients simulés en tests ; ChatGPT/Codex indisponible, aucun abonnement commercial AI Center ajouté.     |
| Fiabilité                        | 8 septembre : contenu normalisé comparé avant replay ou nouveau run ; conflit 409 sans nouvel appel pour une identité réutilisée avec un autre texte.                             | Tests HTTP/DB locaux, pas d'exploitation prolongée.                                                      |
| Upgrade                          | 8 septembre : découverte de toutes les migrations, refus avant création de base, arrêt sur erreur et préservation des données legacy ; 7 unités et 5 migrations réelles réussies. | Base exclusive jetable ; aucune base utilisateur migrée.                                                 |
| Sauvegarde                       | 8 septembre : 41 tables, 98 policies, données logiques/grants/RLS concordants après restauration SQL.                                                                             | Déchiffrement après restauration avec la clé maîtresse séparée non rejoué ; hébergement privé non testé. |
| Harness Alpha                    | 8 septembre : 41 tests, dont 14 nouveaux ; origine fixe, redirections/proxies hérités refusés, délai total et échanges de 2 Mio au maximum.                                       | Loopback et clés fictives ; Unix/fork monothread requis ; campagne réelle non exécutée.                  |
| Intégrations ciblées             | 8 septembre : 7 tests PostgreSQL, 17 unités des gardes de cible ; Clippy serveur tous targets, formatage et syntaxe réussis.                                                      | Aucun pgTAP complet, build desktop ou batterie frontend répété ce jour.                                  |
| Connexions — batterie antérieure | 7 septembre : 125 unités Rust, 49 tests web, 32 pgTAP et 11 intégrations PostgreSQL sur `5ea7ce3`, intégré sans changement de source à `e8b0541`.                                 | Preuves datées ; ne pas les compter comme réexécutées le 8 septembre.                                    |
| Documents opérateur              | 8 septembre : registre vierge des dix boucles et dossier de l'alpha privée avec sources des mesures et inconnus explicites.                                                       | Aucun hébergeur, invité, calendrier de sauvegarde, alerte ou résultat réel créé.                         |

Le [rapport du 8 septembre](../specs/alpha-context-proof/review-2026-09-08.md)
conserve les versions, journaux et deux axes de revue. Les preuves du
[7 septembre](../specs/provider-connections/validation-2026-09-07.md) et du
[5 septembre](../specs/alpha-context-proof/review-2026-09-05.md) restent attachées
à leurs versions. La stack, ses volumes et son service Podman temporaires ont
été supprimés après les tests du 8 septembre.

## État distant et gates ouverts

La consultation en lecture seule du 8 septembre confirme la PR ouverte et
prête pour revue au HEAD public `91dc76b95b798ec6adc22480a86db9e9e09da200`.
Les déclenchements push/PR du 5 septembre ont chacun réussi Desktop CI 5/5 et
OCI 3/3. Ils ne valident pas les connexions ou corrections locales suivantes.
Aucun nouveau push, relancement de workflow, déploiement ou tag n'a été effectué.

| Gate                      | Preuve encore nécessaire                                                                                                                                                 |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| CI des changements locaux | Exécution distante sur la version qui sera publiée ; ne pas réutiliser le succès de `91dc76b`.                                                                           |
| Parcours utilisateur      | E2E, exploration UI et smoke Tauri par le propriétaire ; [recette](../specs/provider-connections/manual-verification.md).                                                |
| Valeur comparative        | Trois projets réels autorisés, corpus annoté, calibration et campagne A/B avec notation humaine ; enveloppe de 100 USD du plan.                                          |
| GitHub live               | GitHub App privée read-only et boucle réelle après transmission du ContextPack.                                                                                          |
| Propriétaire              | Dix boucles sur trois projets pendant une semaine ; [registre vierge](../specs/alpha-context-proof/owner-proof-template.md).                                             |
| Équipe                    | HTTPS privé, deux à trois utilisateurs pendant deux semaines, alertes et restauration dans cet environnement ; [dossier opérateur](operations/private-alpha-handoff.md). |
| Release                   | Aucune promotion `v0.2.0-alpha.1` tant que les gates réels restent ouverts.                                                                                              |

Le refus OpenAI d'août est historique ; aucun quota ou compte réel n'a été
recontrôlé. Le schéma et le serveur doivent être mis à niveau ensemble.
Le checkout canonique `/home/gneed49/Documents/projects/AICenter`, les sources
historiques immuables et les fichiers synchronisés restent intacts.

## Sources de suivi

- [Spécification](../specs/alpha-context-proof/spec.md),
  [plan](../specs/alpha-context-proof/plan.md),
  [tickets](../specs/alpha-context-proof/tickets.md) et
  [validation](../specs/alpha-context-proof/validation.md).
- [Architecture](architecture/README.md),
  [protocole d'évaluation](../specs/alpha-context-proof/evaluation/README.md) et
  [statut durable](../PROJECT_STATUS.md).
- [Audit historique du 25 août](https://github.com/gneed49/ai-center/blob/8a6e0da08bc2cd47088e0b25af398ec3ba30312c/docs/current-state-audit.md).
