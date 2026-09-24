# Spécification — MVP du plan de contrôle contextuel

> Statut : implemented  
> Responsable : AI Center  
> Dernière mise à jour : 2026-08-18

## Intention

Livrer une première version utilisable d’AI Center qui prouve la continuité
`intention → connaissances → handoff → livrable → preuve`, sans réduire le
produit à un chat ou à un gestionnaire de documents.

Le premier utilisateur est un solo builder AI-native. Il doit pouvoir cadrer
`Credits v2` avec l’agent Produit, transmettre le contexte à l’agent Tech sans
le reformuler, obtenir un plan traçable, puis traiter une contradiction détectée
proactivement.

## Résultat attendu

Une application responsive fonctionne dans un navigateur et dans une enveloppe
Tauri v2 sous Linux. Elle communique avec un serveur Rust déployable, persiste
l’état dans Supabase PostgreSQL et utilise un moteur agentique côté serveur.

Après fermeture et redémarrage, le projet, ses sessions, connaissances,
versions, ContextPacks, livrables, preuves, handoffs et insights sont retrouvés.

## Contexte et sources

- Décisions applicables :
  - `docs/decisions/0001-cross-platform-control-plane.md`
  - `docs/decisions/0002-supabase-postgres-persistence.md`
  - `docs/decisions/0003-server-side-agent-engine.md`
- Documents liés :
  - `docs/product/04-mvp-scope.md`
  - `docs/product/05-product-differentiation.md`
  - `docs/ux/ai-center-design-system.md`
- Concepts de référence :
  - `docs/ux/concepts/credits-v2-command-center-desktop.png`
  - `docs/ux/concepts/credits-v2-command-center-mobile.png`
  - `docs/ux/concepts/credits-v2-product-session-desktop.png`
- Contraintes :
  - Linux est la plateforme desktop de développement et de validation.
  - La même interface doit rester utilisable comme web app responsive.
  - Les secrets OpenAI et PostgreSQL restent exclusivement côté serveur.
  - Les templates, profils et contrats sont des données, pas des branches UI.

## Périmètre

### Inclus

- Workspace mono-utilisateur explicite et plusieurs projets.
- Template système `Software Product Delivery` avec nœuds Produit et Tech.
- Profils d’agents data-driven, prompts et politiques de retrieval distincts.
- Sessions persistantes, messages et propositions de mutations atomiques.
- Confirmation, rejet, versionnement et audit des connaissances.
- Graphe projet : entrées versionnées et arêtes typées.
- `ProductReadyGate`, `FeatureBrief`, `ContextPack`, handoff Produit → Tech.
- `TechnicalDeliveryPlan`, sections reliées aux exigences et couverture.
- Preuves documentaires, limites et trous de couverture.
- Steward déclenché par commit, contradiction sourcée et cycle de vie complet.
- Invalidation et réévaluation des projections dépendantes.
- Center, création de projet, projet, session, livrables, handoff,
  Decision Inbox, détail d’insight et historique.
- API Rust, PostgreSQL Supabase, frontend React/TypeScript/shadcn et Tauri v2.
- Executor documentaire simulé derrière un contrat d’adapter.

### Exclu

- Runtime macOS, applications mobiles natives et publication sur les stores.
- Collaboration multi-utilisateur, organisations et SSO.
- Canvas de graphe libre, templates personnalisables et agent inter-projets.
- Agent de code réel, pull request automatique et édition de code mobile.
- Voix bidirectionnelle, écoute permanente et widget système.

## Architecture fonctionnelle

1. Le client ne modifie jamais directement le graphe.
2. Une session envoie un message à l’API avec son nœud et son profil.
3. Le moteur agentique retourne une réponse et des mutations structurées.
4. Les mutations restent `proposed` jusqu’à une confirmation explicite.
5. Le commit crée une version, un événement et un audit.
6. Le steward sélectionne les connaissances liées, évalue la cohérence,
   matérialise les insights et invalide les projections dépendantes.
7. Le compilateur produit un ContextPack immutable dont chaque élément
   référence une version exacte.
8. Les livrables et preuves sont reliés aux exigences par des identifiants
   stables et leur couverture est recalculable.

## Parcours et comportements

### Création et reprise

L’utilisateur crée `Credits v2`. Le serveur instancie le template et ses profils.
Le projet apparaît au Center et reste accessible après redémarrage.

### Boucle Produit

Dans une session Produit, l’utilisateur écrit que les crédits achetés
n’expirent jamais. L’agent challenge l’intention, expose les sources utilisées et
propose au moins une règle métier, une exigence et des critères d’acceptation.
Rien ne devient vérité avant confirmation.

### Gate, livrable et handoff

Le gate explique précisément ses éléments manquants. Lorsqu’il passe, un
FeatureBrief structuré peut être produit. Le ContextPack Tech contient objectif,
décisions, exigences, contraintes, questions, contrat et provenance. Le handoff
ouvre une session Tech sans demander de reformulation.

### Boucle Tech et couverture

L’agent Tech reçoit le ContextPack, produit un TechnicalDeliveryPlan par
sections, relie ces sections aux exigences et crée des preuves documentaires.
Chaque exigence est `covered`, `partial` ou `missing`.

### Contradiction proactive

Le commit de la règle Tech « purger après 90 jours » déclenche le steward. Un
insight relie les deux versions concernées, explique le conflit, expose confiance
et sévérité, puis propose `accepter`, `rejeter` ou `résoudre`. Une modification
réévalue le signal et les projections dépendantes.

### Dégradation

Une indisponibilité du fournisseur IA ne supprime aucune donnée confirmée.
L’API retourne une erreur actionnable et conserve message et session. Les tests
utilisent un moteur déterministe contractuellement équivalent.

## Critères d’acceptation

- [x] AC-01 — Un projet est créé avec Produit, Tech et leurs profils explicites.
- [x] AC-02 — Les deux profils produisent des instructions et contextes distincts.
- [x] AC-03 — Une session produit des mutations proposées puis confirmables.
- [x] AC-04 — Une connaissance confirmée est versionnée, sourcée et auditée.
- [x] AC-05 — Projet, session et connaissances survivent à un redémarrage.
- [x] AC-06 — Le ProductReadyGate explique les manques puis passe ou avertit.
- [x] AC-07 — Le FeatureBrief respecte son contrat et cite ses sources.
- [x] AC-08 — Le ContextPack Tech référence toutes ses versions et provenances.
- [x] AC-09 — Le handoff crée une session Tech sans reformulation Produit.
- [x] AC-10 — Le plan Tech relie ses sections aux exigences.
- [x] AC-11 — 100 % des exigences ont un état de couverture.
- [x] AC-12 — Au moins un trou de preuve devient un insight actionnable.
- [x] AC-13 — Le conflit expiration / purge 90 jours est détecté sur commit.
- [x] AC-14 — Le conflit cite les deux entrées, confiance, sévérité et explication.
- [x] AC-15 — Le conflit peut être accepté, rejeté ou résolu avec audit.
- [x] AC-16 — Une nouvelle version invalide puis réévalue les projections liées.
- [x] AC-17 — Les dix surfaces P0 sont accessibles et responsive.
- [x] AC-18 — Le même build frontend fonctionne sur le web et dans Tauri Linux.
- [x] AC-19 — La clé OpenAI n’est jamais exposée au bundle client.
- [x] AC-20 — Le scénario Credits v2 passe en test end-to-end.

## Preuves attendues

- Tests :
  - unitaires Rust pour gates, compilation, couverture et contradictions ;
  - intégration PostgreSQL pour transactions, versions et reprise ;
  - composants et flux React ;
  - parcours Playwright desktop et mobile ;
  - smoke test Tauri et build Linux.
- Captures :
  - Center/projet desktop et mobile ;
  - session Produit ;
  - handoff ;
  - insight et couverture.
- Artefacts :
  - schéma déclaratif et migration Supabase ;
  - inventaire des routes ;
  - bundles web et Tauri ;
  - rapport de couverture des critères.

## Risques et questions ouvertes

- Le fournisseur IA peut être coûteux ou indisponible : le domaine ne dépend pas
  de son SDK et les validations métier restent déterministes.
- Une détection sémantique trop bruyante détruit la valeur : filtre déterministe
  avant appel modèle, seuil configurable et faux positifs conservés.
- Tauri ne doit pas devenir le serveur : il reste un client natif léger.
- Supabase Auth est différé pour le mono-utilisateur ; l’API conserve néanmoins
  des identifiants d’acteur et de workspace sur chaque mutation.
