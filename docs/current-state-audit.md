# AI Center — état courant Alpha Context Proof

> Revue du 5 septembre 2026 ; point de contrôle `90ca758`.
>
> Branche locale : `feat/alpha-context-proof` ; base PR : `abb779c`.
>
> La [PR #1](https://github.com/gneed49/ai-center/pull/1) reste draft au commit
> distant `8a6e0da`. Les changements de cette reprise ne sont pas encore poussés.

## État de la consolidation

La reprise réunit les tickets T01 à T10 dans une seule branche. La revue
Standards/Spec a relevé huit corrections encore en cours : historique GitHub
lors d'un retour à un état déjà observé, renouvellement des commandes longues,
mesure des entrées d'évaluation, intégrité des exports utilisés pour la campagne,
publication concurrente des Feature Briefs et trois points de cohérence du code
et de l'ADR. Les résultats ci-dessous précèdent ces corrections et ne les
certifient pas.

AI Center conserve son rôle de contrôle du contexte : connaissances versionnées,
ContextPacks, passages de relais, références observées, preuves et couverture.
Les outils externes produisent le code et les résultats. Le connecteur GitHub
de l'application reste strictement en lecture seule. Web desktop et Tauri Linux
sont les seuls clients de ce jalon.

## Méthode de preuve

| Statut | Signification |
| --- | --- |
| `Vérifié` | Reproduit sur la version et dans l'environnement indiqués. |
| `Présent mais non reproduit` | Code ou artefact identifiable, parcours complet non exécuté. |
| `Déclaré` | Résultat historique sans reproduction sur la version courante. |
| `Futur` | Résultat ou exploitation encore à réaliser. |

Une suite UI avec API simulée ne prouve pas PostgreSQL. Une intégration avec
faux fournisseur ne prouve pas une GitHub App privée ou la qualité OpenAI. Un
build Linux ne prouve pas le parcours natif. La validation locale ne constitue
ni une CI distante, ni une publication, ni un déploiement.

## Changements réunis

| Domaine | Comportement livré |
| --- | --- |
| Stack isolée | Identité, ports, configuration et nettoyage propres ; refus du reset d'une cible de développement. |
| Connaissances et stale | Invalidation des projections dépendant des versions réellement modifiées ; fallback global explicite si la provenance manque ; résolution sans mutation refusée. |
| Couverture | Opération structurée indépendante du plan technique, UUID contrôlés, run propre et absence de livrable partiel après échec. |
| Auth desktop | Changement d'identité atomique, annulation des requêtes antérieures, cache réinitialisé et réponses tardives rejetées ; saisies préservées lors des erreurs réessayables. |
| GitHub | Repositories, PR, commits, checks et statuses ; limites 403/429 temporaires, ETag et références canoniques. |
| Traçabilité externe | ContextPack transmis explicitement relié à tâche, exécution observée, événements, artefact, référence et preuve humaine ; aucune exécution de code par AI Center. |
| Évaluation | Calibration et gel du corpus/contrat, répétitions principales/réserve, évaluateurs distincts et contrôles d'aveuglement renforcés ; corrections des métriques en cours de revue. |
| Desktop natif | Build avec configuration loopback isolée, script de parcours métier et CI manuelle ; exécution interactive non reproduite. |
| Dépendances | Résolutions web corrigées et installation propre contrôlée. |

## Validations locales reproduites

| Contrôle | Version et résultat | Limite |
| --- | --- | --- |
| Qualité | `c8a723b` : fmt, lint, Clippy strict, liens Markdown, builds web/serveur ; 89 tests Rust, 6 Vitest, 20 tests d'évaluation offline, 17 gardes stack | Un test DB est ignoré dans la suite unitaire et exécuté dans la phase DB. |
| Desktop avec API/Auth simulées | `90ca758` : 126/126, soit 42 scénarios sur Chromium 1440×900, Chromium 1024×768 et Firefox 1440×900 ; aucun retry | Contrats simulés, pas de fournisseur réel. |
| PostgreSQL | `90ca758` : base neuve, upgrade baseline→alpha, 32 pgTAP, neuf tests métier et vérification du rôle runtime NOBYPASSRLS | Stack Supabase locale jetable. |
| Auth | `90ca758` : magic link local, réponses attendues 200/200/403/403/401 | Pas de domaine privé HTTPS ni rotation JWKS réelle. |
| Sauvegarde | `90ca758` : restauration et comparaison de 39 tables et 96 policies, nettoyage confirmé | Pas de restauration sur l'hébergement alpha. |
| Linux | `90ca758` : build du vrai client avec overlay de smoke, ELF et empreinte produits ; quatre tests de garde hors interface | Le client métier n'a pas été lancé. |
| Dépendances | npm : zéro avis au contrôle ; cargo audit : aucun avis bloquant, 18 avertissements et exception limitée à SQLx MySQL inactive documentés | État ponctuel des avis, pas une garantie générale. |
| Navigateur sur API/DB réelles | `37921d8` : Chromium 1/1, intention → connaissances → gate → pack → handoff → reload | Preuve antérieure ; l'extension plan/couverture/historique du scénario actuel reste non reproduite. |

Les commandes et journaux de ces contrôles sont détaillés dans le
[registre de validation](../specs/alpha-context-proof/validation.md). Les
journaux bruts restent locaux et ne contiennent pas de corpus live publié.

## Limites et gates ouverts

| Gate | Statut et preuve encore nécessaire |
| --- | --- |
| Revue finale | Corrections des huit constats en cours, puis nouvelles validations ciblées. |
| CI du nouveau HEAD | `Présent mais non reproduit` : workflows versionnés ; pas encore de run distant des nouveaux commits. |
| Exploration UI et smoke natif | `Présent mais non reproduit` : exécution bloquée par le contrôle d'autorisation de l'environnement ; pas de relance indirecte. Voir le [runbook natif](operations/native-desktop-smoke.md). |
| Valeur comparative | `Futur` : trois projets réels autorisés, corpus annoté, calibration, campagne A/B et notation humaine ; budget ferme 100 USD. |
| GitHub live | `Futur` : GitHub App privée read-only et boucle réelle après transmission du ContextPack. |
| Propriétaire | `Futur` : dix boucles sur trois projets pendant une semaine. |
| Équipe | `Futur` : HTTPS privé sur invitation, deux à trois utilisateurs pendant deux semaines, alertes et restauration dans cet environnement. |
| Release | `Futur` : aucune promotion `v0.2.0-alpha.1` tant que les gates réels restent ouverts. |

Le refus OpenAI `insufficient_quota` rapporté le 25 août est historique : le
quota, les credentials et les fournisseurs réels n'ont pas été recontrôlés
pendant cette reprise. Aucun consentement de corpus ni résultat live n'a été
inventé.

## Historique et sources

L'[audit du 25 août au commit publié](https://github.com/gneed49/ai-center/blob/8a6e0da08bc2cd47088e0b25af398ec3ba30312c/docs/current-state-audit.md)
reste consultable. Les anciens nombres de tests et résultats CI sont propres
à ce snapshot. La PR distante référence Desktop CI #22 et OCI #21 sur `8a6e0da` ;
ces résultats ne s'étendent pas aux commits locaux de reprise.

- [Spécification](../specs/alpha-context-proof/spec.md), [plan](../specs/alpha-context-proof/plan.md) et [tickets](../specs/alpha-context-proof/tickets.md).
- [Validation et gates](../specs/alpha-context-proof/validation.md).
- [Architecture et diagrammes](architecture/README.md).
- [Protocole d'évaluation](../specs/alpha-context-proof/evaluation/README.md).
- [Statut durable du projet](../PROJECT_STATUS.md).

Le checkout canonique `/home/gneed49/Documents/projects/AICenter` n'a pas été
modifié. La reprise travaille dans le checkout du projet ChatGPT. Les sources
historiques immuables et les fichiers synchronisés sont préservés.
