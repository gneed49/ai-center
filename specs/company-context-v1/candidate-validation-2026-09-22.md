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
| CC-022 | Complément bibliothèque implémenté et vérifié localement ; commit/CI communs à rattacher ci-dessous |
| CC-031/033 | Navigation fichier/tâche et parcours clavier/compact vérifiés ; commit/CI communs à rattacher ci-dessous |
| CC-053 | Création et transmission en langage métier corrigées ; nouveaux libellés passés dans les 147 scénarios navigateur existants |
| CC-055/057, G1 | Rattacher le complément courant à un commit, sa CI et ses images reconstruites ; conserver les limites de revue et les preuves externes séparées |
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


## Publication du jalon et correction transport/images

Le jalon **9be97b6eb052e1b3a6933e954f26f9c9756724f5** est poussé dans la
[PR de travail n°3](https://github.com/gneed49/ai-center/pull/3). Les contrôles
qualité, intégration PostgreSQL, navigateur Chromium, build Tauri Linux,
audits, politique packaging et builds API/web passent pour ce commit
([CI](https://github.com/gneed49/ai-center/actions/runs/35704464351),
[images](https://github.com/gneed49/ai-center/actions/runs/35704464360)).

La correction qui suit ajoute TLS PostgreSQL vérifié, healthcheck intégré et
images minimisées. 155 unités Rust passent (10 tests DB ignorés dans cette
commande), Clippy passe ; le test TLS réel, les deux recettes d'images, Auth
local et les deux parcours navigateur passent. Le
[rapport de qualification des images](../../docs/operations/image-security-2026-09-22.md)
conserve les résidus et les identités exactes. La correction est poussée au commit `596cfee`. Ses huit contrôles CI
distincts passent également ([CI](https://github.com/gneed49/ai-center/actions/runs/35706108325),
[images](https://github.com/gneed49/ai-center/actions/runs/35706108145)).


## Complément bibliothèque, navigation et langage métier

Le jalon suivant ajoute `/knowledge`, pagination/recherche, sélection des versions
historiques et consultation des projets archivés. La recette de 31 connaissances
vérifie la dernière page, les révisions, la recherche littérale, le lecteur et
le refus inter-sociétés. Un fichier du graphe ouvre son corpus et son UUID exact ;
une référence absente ne sélectionne jamais silencieusement un autre fichier.
Les actions de création et transmission décrivent désormais les cinq agents et
les étapes métier réelles.

- Qualité : **157 tests web / 41 fichiers**, **155 unités Rust / 10 tests DB
  ignorés dans cette commande**, 3 contrats synthétiques, 77 tests Python,
  formatage/lint/Clippy et builds passent (`company-library-navigation-quality-final.log`).
- PostgreSQL : **21 scénarios société** et les suites intégrées passent, dont
  bibliothèque et liens exacts des fichiers ; **2/2 parcours navigateur réels**
  passent avec recherche → connaissance et nouveaux libellés
  (`company-library-navigation-integrated.log`).
- Navigateur avec doubles HTTP : **147/147** scénarios existants passent sur
  Chromium desktop/compact et Firefox ; **3/3** compléments graphe → tâche/fichier
  passent, avec clavier, reload, référence absente, axe et largeur 390 px.
  Journaux `company-library-navigation-browser.log` et `company-exact-source-browser.log`.
- Contrôle visuel de la capture 390 px : contenu et actions lisibles, code avec
  défilement interne, pas de débordement de page. Capture locale
  `.run/company-exact-source-mobile.png`.
- Revue ciblée de coordination : RLS et filtres/compteurs cohérents, identité
  de version et route interne contrôlée, aucune écriture ou requête fournisseur
  déclenchée par la bibliothèque. Les deux constats de fiabilité précédents ont
  une résolution documentée ; aucune seconde revue indépendante des ajouts
  courants n'est revendiquée.

Les journaux non qualifiés par un chemin complet ci-dessus sont dans `.run/`.
Les tests restent synthétiques : aucune écriture dans un espace client, aucun
appel IA facturé et aucune preuve de valeur par des utilisateurs réels.
