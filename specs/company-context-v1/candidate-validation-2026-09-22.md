# Company Context V1 — consolidation du 22 septembre 2026

Branche `feat/company-context-v1`, base auditée
`966ce9bbb597327e0ebbc3084836c6dedd759eca`. Travaux autorisés le 21 septembre.
Ce rapport accompagne une candidate en développement, pas une ouverture de
production. Les identités et contenus de test sont synthétiques `[FICTIF]`.

## Fonctionnement présent

- Société privée, équipe invitée, rôles propriétaire/éditeur/lecteur ; tous les
  projets de la société sont partagés. Création de société contrôlée côté serveur.
- Cinq agents métier, conversations durables, contexte choisi parmi des sources
  autorisées et versionnées ; contraintes obligatoires et omissions explicites.
- Documents et versions, validation, comparaison, bibliothèque paginée,
  export et destinations héritées ou spécifiques au projet.
- Notion, Linear et GitHub : connexions chiffrées, création explicite, jobs
  durables, réconciliation et observation des changements. Mise à jour distante
  non offerte ; aucune exécution de code. Accès tiers réels non qualifiés.
- Graphes de projets fédérés, liens persistés, steward asynchrone, contradictions
  sourcées et qualification distincte des preuves. Le contenu GitHub est borné,
  rattaché à des commits/fichiers précis et ne donne pas une certification du dépôt.
- Arrêt, quotas société/membre, reprise des commandes et fencing des traitements.
- Export société/projet, archive/restauration et maintenance opérateur hors API.
  L'effacement d'un projet refuse les dépendances entrantes/sortantes ; les
  historiques interconnectés exigent une analyse de rétention explicite.

## Preuves reproduites

Les journaux `.run/` sont locaux et ignorés par Git. Les scripts et tests qui
les produisent sont conservés dans le dépôt. Un journal n'est pas publié sans
vérification de son contenu. Les photos intermédiaires ne certifient pas un
commit ultérieur.

| Domaine | Résultat et portée | Preuve reproductible |
| --- | --- | --- |
| Qualité intégrée courante | Formatage, lint, Clippy, 154 tests web, 152 unités Rust, 3 contrats synthétiques et 77 tests Python passent ; 9 tests DB intentionnellement ignorés dans la suite unitaire | `scripts/ci-desktop.sh quality` ; `.run/company-v1-quality-graph-navigation.log` |
| Nouvelle consultation des sources | Build web, Clippy serveur et 154 tests web passent ; vérification des identités, ancienne version, provenance, contenu échappé et tâche sans exécution | `graph-source-page.test.tsx`, `company_context/graph_navigation.rs` ; `.run/graph-navigation-*` |
| Intégration courante | 20 scénarios société, 2 artefacts, 4 invitations, 6 outils, 4 connexions/reprise, 6 quotas, 2 outbox, 1 persistance GitHub et les suites historiques passent | `.run/company-v1-graph-navigation-integrated.log` |
| Maintenance | Effacement société/projet sur fixtures, rollback d'un effet secondaire inattendu, export ciblé, clôture abandonnée avec trois états de publication et protection des anciennes commandes passent | `company_context/{erasure,project_data,abandoned}.rs` |
| Permissions et migration | Baseline conservée, inventaire runtime/RLS et 32 pgTAP passent ; 13 migrations depuis la baseline avec l'ajout des conversations au graphe | `scripts/integration-stack.sh run integration` ; `.run/company-v1-graph-navigation-integrated.log` |
| Sauvegarde/restauration | 56 tables et 133 politiques restaurées dans une base séparée ; preuve locale | `scripts/postgres-backup-restore-smoke.sh` |
| Auth réel local | Identité éphémère, magic-link/OTP, rôle lecteur, refus d'écriture, refus inter-sociétés, ancien JWT après retrait et accès sans bearer passent | `scripts/auth-magic-link-smoke.sh` ; SMTP externe non exercé |
| Navigateur ciblé avec doubles API | 15 scénarios passent sur Chromium et Firefox ; isolation/reprise/publication complétées par les rapports frontend du 21 septembre | `.run/company-v1-browser-20260922.log` |
| Navigateur avec API/DB réelles | Deux parcours passent : document à quatre versions, export projet ; conversation → contexte → plan/couverture/historique et navigation depuis le graphe vers conversation, connaissance et pack exact | `apps/web/real-e2e/` ; fournisseur déterministe, aucun appel facturé |
| Image web en exécution | Pages, liens directs, cache HTML/assets, en-têtes, 404, santé et indisponibilité API passent en conteneur local non privilégié | `scripts/oci-web-smoke.py` ; `.run/company-web-image-current-smoke.log` |

Les anciennes suites utilisent la même identité de fixture. Elles atteignaient
le plafond membre de 60 appels/heure, ce qui refusait correctement les appels
suivants. Les seules recettes déterministes lèvent leurs budgets horaires à
10 000. Les tests de quotas conservent des limites basses explicites et prouvent
le refus avant transport ainsi que la capacité restante pour un collègue.
Les valeurs de production restent 120 appels/société et 60/membre par heure,
avec concurrences 4/société et 2/membre. L'attente du faux fournisseur est bornée
pour qu'une régression échoue sans bloquer indéfiniment la CI.

## Revue et corrections

Les revues précédentes sont conservées dans `backend-review.md`,
`reliability-audit.md` et `independent-reliability-review.md`. Corrections de
coordination supplémentaires : route export projet réellement raccordée,
fixture de publication cohérente avec les versions immuables, fermeture de
commande terminale après maintenance, numéros de version dans le graphe,
navigation interne autorisée distincte des URL HTTPS, en-têtes Nginx hérités
dans toutes les locations et délai proxy compatible avec les opérations longues.

La dernière revue du graphe a ajouté les conversations, tâches, liens vers les
packs et lectures exactes de connaissances historiques. La recette intégrée passe : 20 scénarios société et deux parcours navigateur
sur API/DB réelles. Les sous-agents ont ensuite atteint leur limite de service ; la
coordination assure l'intégration, sans prétendre à une nouvelle revue
indépendante exhaustive des derniers ajouts.

## Critères encore ouverts

| Gate / exigence | Travail restant |
| --- | --- |
| CC-022 | Consultation/recherche de toutes les connaissances au-delà de l'aperçu projet et de la limite de projection du graphe ; vérifier les anciens éléments |
| CC-031/033 | Navigation exacte vers les fichiers GitHub observés ; complément compact/clavier des nouveaux objets, preuve navigateur tâche |
| CC-053 | Remplacer les libellés techniques résiduels des écrans hérités dans les parcours principaux |
| CC-055/057, G1 | Consolider commit, revue corrective, CI distante et images reconstruites sur un état identifié ; scan complet des images et reconstruction API encore requis |
| CC-014/026/027/041, G2 | Compte/modèle IA et budget autorisés ; destinations de test Notion/Linear/GitHub ; preuve de valeur des analyses par modèle réel |
| CC-055/056, G3 | Hébergeur/domaine, Auth/SMTP, configuration secrète, HTTPS, sauvegardes automatiques, restauration/alertes et responsable d'exploitation sur la cible |
| G4 | Projets et utilisateurs réels sur la durée prévue ; utilité des contradictions, coût et frictions effectivement mesurés |

La clé éventuellement présente localement n'est ni relue dans les rapports ni
utilisée pour facturer sans réponse à la question de budget. Aucun connecteur
installé dans Codex ne remplace la connexion sécurisée de l'entreprise dans
AI Center. Aucun push, conteneur construit ou test local ne prouve un déploiement.


## Jalon enregistré

Le commit qui introduit ce rapport est un jalon de consolidation local. Son
identité se retrouve par `git log -1 -- specs/company-context-v1/candidate-validation-2026-09-22.md`.
Il ne clôt pas G1 : les écarts du tableau ci-dessus restent des travaux actifs.
Le scan initial Grype 0.119.0 de l'image web antérieure aux dernières pages trouve
zéro vulnérabilité avec correctif disponible et neuf correspondances sans
correctif classées à part. Ce résultat filtré ne signifie pas zéro vulnérabilité ;
l'analyse non filtrée et l'image finale doivent encore être qualifiées.
