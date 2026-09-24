# Rapport de validation — MVP du plan de contrôle contextuel

> Résultat : 20/20 critères acceptés  
> Plateforme validée : Linux, web responsive et Tauri v2  
> Date : 2026-08-18

## Matrice d’acceptation

| Critère | Statut | Preuve directe |
| --- | --- | --- |
| AC-01 | Pass | `create_project` instancie les nœuds et profils Produit/Tech ; vérifié par le scénario PostgreSQL. |
| AC-02 | Pass | Profils, prompts, contrats et politiques de contexte sont stockés séparément et injectés au moteur. |
| AC-03 | Pass | Le test obtient trois propositions `proposed`, puis les confirme explicitement. |
| AC-04 | Pass | Chaque commit crée connaissance, version, source, événement et audit dans une transaction. |
| AC-05 | Pass | Le snapshot relit projet, deux nœuds, sessions et graphe depuis PostgreSQL après le parcours. |
| AC-06 | Pass | `ProductReadyGate` calcule règles, exigences et critères, explique les manques et passe après commit. |
| AC-07 | Pass | Le FeatureBrief est généré depuis le contrat seedé et conserve ses références de versions. |
| AC-08 | Pass | Le ContextPack immutable contient objectif, contrat, connaissances et provenance versionnée. |
| AC-09 | Pass | Le handoff crée et ouvre la session Tech avec le ContextPack, sans ressaisie de l’intention. |
| AC-10 | Pass | Le TechnicalDeliveryPlan persiste des sections et leurs liens aux exigences. |
| AC-11 | Pass | Le rapport vérifie `total = covered + partial + missing` pour toutes les exigences. |
| AC-12 | Pass | Les trous de preuve sont exposés dans la Decision Inbox avec actions et sources. |
| AC-13 | Pass | Le commit Tech « purge 90 jours » crée exactement un conflit avec « n’expirent jamais ». |
| AC-14 | Pass | Le détail du conflit contient deux sources, sévérité `blocking`, confiance et explication. |
| AC-15 | Pass | Les actions accepter, rejeter et résoudre ont une route dédiée et produisent événement/audit. |
| AC-16 | Pass | Une révision crée v2, invalide les projections dépendantes puis résout/réévalue le conflit. |
| AC-17 | Pass | Les 10 routes P0 ont été ouvertes avec contenu attendu ; vues bureau 1440 px et mobile 390 px inspectées. |
| AC-18 | Pass | `apps/web/dist` est consommé par le web et par le build Tauri ; binaire, `.deb` et `.rpm` générés. |
| AC-19 | Pass | Secrets absents du code et du bundle ; `OPENAI_API_KEY` est uniquement lu par le serveur. |
| AC-20 | Pass | `credits_v2_crosses_the_full_context_control_plane` exécute le parcours entier sur Supabase local. |

## Inventaire des surfaces

1. Center — `/`
2. Création — `/projects/new`
3. Projet — `/projects/:projectId`
4. Session — `/projects/:projectId/sessions/:sessionId`
5. Livrables — `/projects/:projectId/deliverables`
6. Détail livrable — `/projects/:projectId/deliverables/:deliverableId`
7. Handoff — `/projects/:projectId/handoff`
8. Decision Inbox — `/insights`
9. Détail insight — `/insights/:insightId`
10. Historique — `/projects/:projectId/history`

## Inventaire API

- Santé et reprise : healthcheck, liste/création de projets, snapshot, historique.
- Sessions : création, lecture, message, confirmation/rejet idempotent des propositions.
- Contrôle : évaluation du gate, FeatureBrief, handoff, TechnicalDeliveryPlan et couverture.
- Connaissances : révision versionnée et invalidation des projections.
- Insights : liste, détail et action auditée.

## Registre de fidélité visuelle

1. Rail bleu nuit fixe et accents indigo conservés depuis les concepts.
2. Hiérarchie éditoriale blanche/gris clair, bordures fines et absence de gradients.
3. Chaîne Produit → Tech préservée ; elle se replie sous 1400 px pour éviter tout écrasement.
4. Métriques du Center, ProductReadyGate et signal d’insight gardent la hiérarchie du concept.
5. La session donne la priorité à la file de propositions, avec le chat en contexte secondaire.
6. Sur mobile, le rail devient un menu et les métriques/cartes s’empilent sans débordement.
7. Couleurs sémantiques stables : indigo actif, émeraude validé, ambre partiel, rouge bloquant.

Écart de copie volontaire : les libellés statiques des concepts ont été remplacés
par des données réelles et une formulation française, tout en conservant
`ProductReadyGate`, `Feature Brief`, `ContextPack`, handoff et Decision Inbox.

## Commandes de preuve

- `npm run lint`
- `npm test`
- `npm run build`
- `npm run build:desktop`
- `npx supabase db lint --local`
- audit RLS, privilèges du schéma, secrets et dépendances

Le mode OpenAI démarre et le parseur de réponse structurée est testé unitairement.
Le parcours automatisé utilise le moteur déterministe afin d’être reproductible et
de ne pas transmettre de données projet à un service externe pendant la validation.
