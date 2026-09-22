# Plan d'implémentation — Company Context V1

> Statut : active — réalisation autorisée, lots en attente  
> Spec liée : [spec.md](spec.md)  
> Dernière mise à jour : 2026-09-21

## Approche

Étendre le socle React/Vite, Rust/Axum/SQLx et PostgreSQL/Supabase. Le workspace
porte la société ; les objets existants sont conservés et migrés sans reset.
L'espace général est le conteneur typé `scope_kind = 'company'` unique par
workspace défini dans l'ADR 0007, distinct des projets métier. Les relations
inter-projets étendent les arêtes par une cible de projet validée ; les sources
de packs portent `source_project_id`, leurs versions et les versions de scopes
nécessaires, sans retirer les contraintes d'appartenance existantes.
Les nouveaux services partagent les contrôles d'autorisation, versions,
idempotence, moteur IA et outbox existants. Des modules métiers distincts
portent société/membres, agents/scopes, artefacts/destinations, graphe et
cohérence ; aucune seconde implémentation du même domaine dans le frontend.

Les adapters de modèle restent séparés des adapters Notion/Linear/GitHub. Une
connexion expose des capacités vérifiées et une destination précise. Les jobs
de publication ou d'analyse portent leur identité, acteur, permissions et
version de source ; les appels externes restent hors transaction SQL. Une
publication ambiguë après timeout est réconciliée avant une nouvelle création.

Le graphe est une vue requêtable des relations réelles. Le graphe société
fédère des sources filtrées par droits, avec une alternative en liste dans
l'interface. Il n'est ni une image figée ni un dump des données de l'entreprise.
L'agent de cohérence réutilise la chaîne événementielle et propose des constats
révisables. Le code reste produit et exécuté dans les outils externes.

Alternatives non retenues : réécriture du framework, seconde base graphe sans
besoin mesuré, remplacement des outils métier, runner interne, index global
non filtré, synchronisation exhaustive et autonomie de correction externe.

## Découpage

Les tickets, responsables et preuves sont dans [tickets.md](tickets.md). Le
coordinateur attribue un seul auteur par module partagé et intègre les lots
après revue ciblée ; les agents peuvent travailler en parallèle sur des
modules indépendants.

### 1. Walking skeleton — CC-T01 et CC-T02

1. Figer baseline/branche/HEAD et conserver les changements existants.
2. Formaliser les contrats communs : société, permissions projet, scope,
   profile d'agent, version d'artefact, destination, nœud/arête et job.
3. Poser types/API et migrations additives communes avec fixtures `[FICTIF]`.
4. Démontrer le trajet société → projet → conversation → artefact → relation
   persistée → affichage, d'abord sans fournisseur réel.

Cette tranche n'est pas déclarée complète si les nouvelles surfaces ne sont
que des exemples frontend sans persistance/autorisation.

### 2. Domaine et persistance — CC-T03 à CC-T05

- T03 : setup société, invitations, membres, projets partagés et révocation.
- T04 : profils et conversations scopées, compilation autorisée société/projet.
- T05 : artefacts et versions, bibliothèque, édition et export, destinations.

Les contrats T02 permettent T03, T04 et T05 en parallèle. Les modifications
au schéma déclaratif et aux migrations sont intégrées par un responsable
unique pour éviter les migrations concurrentes divergentes.

### 3. Intégrations — CC-T06 et CC-T07

- T06 : interface commune de destination, secrets serveur, publication
  durable, reprise, conflit, refresh et capacités explicites.
- T07 : adapters Notion, Linear, GitHub, puis lecture ciblée GitHub des sources
  utiles aux écarts ; tests de contrat indépendants par adapter.

Les trois adapters peuvent être implémentés en parallèle une fois T06 fixé.
La qualification réelle ne doit pas retarder la validation des autres modules
si une connexion manque ; un statut visible conserve la limite.

### 4. Graphes et cohérence — CC-T08 et CC-T09

- T08 : nœuds/arêtes/provenance et queries bornées de projet puis société,
  filtres d'accès, recherche, projections et invalidation inter-projets.
- T09 : événements de connaissance/artefact/observation, travaux de steward,
  comparaison sourcée, données insuffisantes, inbox et résolution.

Le modèle du graphe ne dépend pas d'une librairie d'affichage. T08 peut
progresser avec des fixtures pendant la construction des adapters. T09 utilise
les contrats de sources et les événements intégrés, pas du scraping libre.

### 5. Interface et fiabilité — CC-T10 et CC-T11

- T10 : navigation société/projet, agents, conversations, bibliothèque,
  destinations, graphes et inbox ; vocabulaire métier, clavier et états d'erreur.
- T11 : opérations longues, idempotence/reprise, saisies conservées,
  concurrence/usage/arrêt IA, vrais états et instrumentation expurgée.

Le frontend peut démarrer avec les types validés ; les doubles restent limités
aux tests et ne deviennent pas le chemin de fonctionnement livré. Vérifier
chaque tranche sur API/DB réelles avant de la déclarer intégrée.

### 6. Données et exploitation — CC-T12

Exporter/supprimer une société ou un projet avec procédure contrôlée ;
configuration et secrets, migrations, images publiables, HTTPS/Auth/SMTP,
readiness, alertes, sauvegardes/restauration séparée et documentation opérateur.
Les actions irréversibles s'exercent sur données fictives ; les opérations
réelles exigent une cible identifiée et l'autorisation déjà applicable.

### 7. Validation et publication — CC-T13 à CC-T15

- T13 : revue Standards/Spec, corrections, suites ciblées puis candidate
  intégrée, E2E web et parcours multi-sociétés ; publication/PR/CI autorisées.
- T14 : configuration et preuves sur comptes/cible réels disponibles, budget
  explicite, chemins fournisseurs et outils, reprise et alertes de la cible.
- T15 : pilote réel, incidents, utilité et décision de promotion documentée.

CC-G1 est un résultat logiciel distinct de CC-G2/G3/G4. Les lots exécutables
continuent lorsqu'un accès externe manque. Ne pas multiplier les tests simulés
en espérant remplacer la preuve manquante.

## Impacts

- **Domaine :** workspace présenté comme société ; scopes explicites,
  profils/contrats, artefacts/destinations, relations inter-projets, constats.
- **API :** endpoints autorisés pour setup/membres/projets, agents/sessions,
  bibliothèque/versions/export, destinations/jobs, graphes et inbox. Pagination
  ou limites explicites ; erreurs actionnables et identités stables.
- **Données :** migrations additives, identifiants stables, contraintes de
  société/projet, RLS/grants, provenance/version, indexes et backfill idempotent.
- **Frontend :** société et projet courants, parcours métiers, liste complète,
  artefacts, connexions et graphes interrogés depuis l'API ; zéro secret privé.
- **Sécurité :** matrice rôles/opérations/scopes, retrait effectif, filtrage
  avant retrieval/génération, sources non fiables, URLs/tailles/délais bornés,
  permissions connecteurs et privilèges minimaux.
- **Exploitation :** version/schéma associés, tâches persistantes, coûts connus
  et inconnus, métriques sans contenu, quotas/coupure, sauvegarde et reprise.
- **Documents :** ADR et docs produit actualisés dans leurs lots ; état durable
  et preuve du candidat dans T13–T15. Les sources historiques restent intactes.

## Validation

- [ ] Types et contrats partagés, typecheck, lint et formatage.
- [ ] Unités métier significatives : autorisation, scope, versions, provenance,
      destinations, reprise, invalidation, comparaison et limites.
- [ ] Contrats Notion/Linear/GitHub avec faux HTTP : permissions, erreurs,
      pagination, timeout, replay, conflit, source manquante et données malveillantes.
- [ ] PostgreSQL réel isolé : migrations depuis baseline, RLS/grants,
      concurrence, jobs, révocation, deux sociétés isolées et projets partagés.
- [ ] E2E navigateur, Chromium desktop puis Firefox : CC-U01 à CC-U07,
      clavier, reload, offline/réponse perdue, espace changé, destination indisponible.
- [ ] Recette UI visuelle des graphes et des principaux écrans sur données
      fictives, avec détail sourcé et alternative accessible.
- [ ] Chaîne PM → lead → outil externe → preuve et retour de cohérence sur
      API/DB réelles ; mode IA indiqué dans chaque rapport.
- [ ] Revue Standards/Spec suivie des corrections et tests affectés, sans
      élargissement répétitif sans nouvelle raison.
- [ ] CI du commit réellement publié et artefacts de livraison identifiables.
- [ ] Gates réels CC-G2/G3 puis pilote CC-G4 consignés séparément ; aucun
      check coché avant exécution et aucune clé dans les preuves.

## Déploiement et retour arrière

1. Relire les migrations additives et les permissions ; sauvegarder la cible
   identifiée avant migration. Ne jamais utiliser un reset de développement
   contre une base utilisateur.
2. Déployer API/web et schéma compatibles en préproduction ; vérifier setup,
   auth, scope, jobs et opérations longues derrière le vrai proxy.
3. Conserver les anciennes versions et les backfills relançables. Bloquer la
   mise à jour si une source legacy ne peut être rattachée sans ambiguïté.
4. Préparer images/digests, configuration et secrets runtime séparés ; activer
   seulement les capacités/connecteurs configurés, avec statut honnête.
5. Exercer restauration sur infrastructure isolée en coupant SMTP, IA,
   publications et jobs sortants. Vérifier droits, données et clés.
6. Pour un rollback, suspendre les jobs et publier la version compatible
   précédente ; ne pas détruire les nouvelles données pour faire réussir le
   retour. Une migration inverse destructive requiert une procédure distincte.
7. Ouvrir la cible privée uniquement avec CC-G2/G3 satisfaits pour les chemins
   annoncés, support défini et limites publiées. Consigner le commit, le schéma,
   les contrôles et la décision ; poursuivre le pilote CC-G4 en temps réel.

La préparation, les tests, la revue et le push sont autorisés. L'absence de
budget, de domaine ou de credential n'autorise ni achat, ni utilisation d'un
compte découvert par hasard. Les décisions encore nécessaires sont circonscrites
à la ressource manquante et ne suspendent pas les lots indépendants.
