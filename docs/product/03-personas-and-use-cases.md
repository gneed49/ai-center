# AI Center — Personas et cas d’usage

> Statut : brouillon à challenger — version 0.1  
> Date : 13 août 2026  
> Périmètre : premier domaine **Projets**, expérience mobile Android et runtime macOS.

## Objectif du document

Ce document identifie l’utilisateur pour lequel AI Center doit être excellent en premier et les situations qui doivent guider le MVP. Le persona n’est pas une cible marketing définitive : il sert à arbitrer les fonctionnalités, le niveau de complexité visible et les compromis d’architecture.

## Décisions de cadrage déjà prises

- AI Center est un centre de commandement global ; **Projets** est sa première section fonctionnelle.
- Une session peut commencer dans le Center ou directement dans un projet.
- Le téléphone est souvent le point de départ, et non la continuation d’une session Mac.
- La v0 utilise une dictée fluide avec réponses écrites ; la conversation audio bidirectionnelle vient plus tard.
- Les sessions peuvent durer trente à soixante minutes, être interrompues et reprises.
- Une session documentaire réussie est un résultat complet.
- L’exécution de code est optionnelle et peut prolonger un travail de cadrage déjà abouti.
- L’utilisateur doit pouvoir entrer directement dans un agent spécialisé sans devoir inspecter le code.
- Sur mobile, la restitution privilégie synthèse, couverture, tests, risques et actions ; preview et pull request servent à l’inspection détaillée.
- Le modèle de contexte doit être structuré dès la v0. Les documents Markdown sont des vues ou exports, pas l’unique source de vérité.
- L’installation du runtime doit être simple, même pour une cible initiale techniquement avancée.

## Persona primaire — le solo builder AI-native

### Profil

Développeur expérimenté, fondateur technique ou product builder menant plusieurs side projects. Il utilise déjà ChatGPT, Claude, Codex ou des agents de code et pense de plus en plus en termes d’intention, de délégation et d’évaluation plutôt que d’édition manuelle continue.

Il possède un Mac pouvant servir de runtime, mais travaille souvent ailleurs pendant la journée. Le soir ou en mobilité, il préfère quitter l’ordinateur et poursuivre depuis son téléphone. Il parle plus vite et plus naturellement qu’il n’écrit sur mobile.

### Comportements déterminants

- démarre fréquemment une nouvelle session depuis son téléphone ;
- mène de longues discussions de produit, d’architecture ou de planification ;
- alterne les casquettes produit, design, tech et développement ;
- veut que les décisions deviennent du contexte durable sans maintenance documentaire manuelle ;
- délègue le code et évalue d’abord le résultat fonctionnel ;
- ouvre une preview ou une pull request lorsque la revue détaillée est justifiée ;
- accepte un système avancé mais refuse une installation artisanale et fragile ;
- travaille sur plusieurs projets et veut reprendre chacun sans réamorcer l’IA.

### Motivations

- transformer les moments loin du bureau en avancement réel ;
- réduire la charge cognitive du passage idée → spécification → code ;
- ne plus transporter manuellement le contexte entre agents ;
- conserver une trace exploitable des décisions et de leurs implémentations ;
- utiliser les meilleurs agents et outils sans être enfermé dans une seule interface.

### Frustrations actuelles

- les conversations utiles ne mettent pas réellement à jour le projet ;
- le clavier mobile rend le travail long pénible ;
- les agents redemandent un contexte déjà fourni ailleurs ;
- le Mac est puissant mais inaccessible comme surface de travail naturelle en mobilité ;
- les sorties techniques sont trop détaillées pour une évaluation mobile ;
- les outils existants séparent réflexion, documentation, exécution et revue.

### Critères de confiance

- savoir dans quel workspace, projet et espace contextuel l’IA agit ;
- pouvoir corriger une décision structurée sans réécrire un document entier ;
- comprendre ce qui va être exécuté avant une action sensible ;
- pouvoir interrompre ou réorienter ;
- distinguer clairement proposition, décision validée, tâche et résultat ;
- retrouver un historique cohérent après une interruption.

## Persona secondaire — le fondateur technique ou lead multi-projets

Il pilote plusieurs initiatives et plusieurs agents, parfois avec une petite équipe. Sa douleur principale est moins l’accès au Mac que la cohérence entre produit, design, tech et exécution.

Il valorise :

- une vue condensée de plusieurs projets ;
- la traçabilité décision → tâche → implémentation ;
- la détection de contradictions ou d’informations manquantes ;
- la délégation à des agents spécialisés ;
- des rapports partageables avec l’équipe.

Ce persona influence l’extensibilité du modèle, mais ne doit pas imposer à la v0 la gestion complète des organisations, rôles et permissions.

## Personas futurs

### Équipe produit–tech

Plusieurs personnes contribuent au même contexte, utilisent des agents différents et ont besoin d’un audit, de permissions et de règles de gouvernance.

### Opérateur d’entreprise non développeur

Il utilise le Center pour d’autres domaines — opérations, comptabilité, support, contenu — avec des agents et outils spécialisés. Ce persona appartient à la vision globale, pas au MVP Projets.

## Anti-personas initiaux

### Le développeur cherchant un IDE mobile

Il veut éditer, naviguer, déboguer et relire précisément chaque ligne sur son téléphone. AI Center peut l’orienter vers une preview ou une pull request, mais ne doit pas optimiser la v0 autour de ce comportement.

### L’utilisateur non technique sans runtime ni projet structuré

L’expérience finale devra pouvoir s’ouvrir à lui, mais ses besoins d’onboarding, d’hébergement et de simplification détourneraient la première validation.

### L’organisation exigeant immédiatement gouvernance et conformité avancées

SSO, politiques d’entreprise, audit complet et multi-équipe sont importants à terme mais incompatibles avec une première boucle personnelle rapide à tester.

## Contextes d’usage prioritaires

| Situation | Durée typique | Modalité | Objectif |
| --- | ---: | --- | --- |
| Canapé ou autre pièce | 20–60 min | Dictée + lecture | Brainstorming, décisions, planification, délégation |
| Déplacement ou voyage | Plusieurs séquences | Dictée asynchrone | Avancer puis reprendre sans perdre le contexte |
| Temps d’attente | 5–20 min | Dictée ou actions rapides | Répondre, arbitrer, autoriser, réorienter |
| Mac disponible à distance | Exécution variable | Supervision mobile | Faire produire code, tests, preview ou PR |
| Retour sur desktop | Selon résultat | Preview, GitHub, IDE | Revue détaillée et finalisation |

La disponibilité pour parler est un facteur plus déterminant que le lieu précis.

## Jobs to be done prioritaires

### JTBD-1 — Faire avancer un projet par la voix

> Lorsque je suis loin de mon ordinateur mais disponible pour réfléchir, je veux dicter une intention et mener une discussion approfondie afin de produire une décision ou un plan exploitable.

### JTBD-2 — Transformer la conversation en contexte durable

> Lorsque des décisions, exigences ou questions émergent, je veux que l’IA les structure et les relie au bon périmètre afin de ne pas devoir reconstruire le contexte plus tard.

### JTBD-3 — Déléguer au bon agent sans coordonner manuellement les fils

> Lorsque le travail relève du produit, de la tech ou du code, je veux entrer dans le bon espace ou laisser l’agent Center router la tâche afin que le contexte pertinent soit utilisé automatiquement.

### JTBD-4 — Faire exécuter le travail sur mon environnement réel

> Lorsque la tâche est prête, je veux la déléguer au runtime de mon Mac afin d’utiliser mes dépôts, outils et configurations sans manipuler un desktop distant.

### JTBD-5 — Garder le contrôle sans suivre chaque détail

> Lorsque l’agent travaille, je veux intervenir uniquement aux décisions importantes afin de conserver mon autorité sans surveiller un flux de terminal.

### JTBD-6 — Évaluer le résultat au bon niveau

> Lorsque le travail se termine, je veux une synthèse et des preuves adaptées au mobile afin de décider de valider, corriger, poursuivre ou ouvrir la preview/PR.

### JTBD-7 — Reprendre sans réamorcer

> Lorsque je reviens plus tard, je veux retrouver l’état, les décisions et les prochaines actions afin de poursuivre immédiatement.

## Entonnoir des cas d’usage

| Priorité | ID | Cas d’usage | Valeur principale | v0 |
| --- | --- | --- | --- | --- |
| P0 | UC-01 | Démarrer ou reprendre par dictée une session dans un projet | Entrée naturelle en mobilité | Oui |
| P0 | UC-02 | Challenger une idée et structurer décisions, exigences et questions | Contexte durable | Oui |
| P0 | UC-03 | Produire une synthèse, un plan ou un livrable documentaire | Résultat complet sans code | Oui |
| P0 | UC-04 | Déléguer une tâche préparée au runtime macOS | Action réelle | Oui, tranche étroite |
| P0 | UC-05 | Répondre, autoriser, réorienter ou interrompre | Contrôle et confiance | Oui |
| P0 | UC-06 | Recevoir un rapport orienté résultat | Évaluation mobile | Oui |
| P0 | UC-07 | Reprendre une session et son état après interruption | Continuité | Oui |
| P1 | UC-08 | Commencer dans le Center puis rattacher la session à un projet | Flexibilité du centre global | Version simplifiée |
| P1 | UC-09 | Entrer directement dans un espace et son agent spécialisé | Profondeur contextuelle | 1–2 espaces pilotes |
| P1 | UC-10 | Ouvrir une preview ou une pull request | Revue détaillée hors du chat | Selon intégration |
| P2 | UC-11 | Naviguer dans un graphe de connaissance | Compréhension des relations | Plus tard |
| P2 | UC-12 | Détecter proactivement contradictions et contexte manquant | Différenciation forte | Après le socle |
| P2 | UC-13 | Piloter plusieurs workspaces et équipes | Extension entreprise | Plus tard |
| P2 | UC-14 | Utiliser le copilote ambiant par widget et voix bidirectionnelle | Vision cible | Plus tard |

## Parcours principal v0

### UC-01 à UC-07 — De l’intention au résultat

1. L’utilisateur ouvre AI Center sur Android.
2. Il sélectionne un projet récent ou recherche le bon projet.
3. Il démarre la dictée et expose une idée de fonctionnalité ou un problème.
4. L’agent récupère le résumé du projet et les objets contextuels pertinents.
5. Il challenge l’intention, pose les questions nécessaires et propose une formulation.
6. Au fil de la discussion, il identifie les décisions, exigences, questions ouvertes et tâches.
7. L’utilisateur confirme les changements importants ; le système met à jour les objets structurés.
8. L’utilisateur peut s’arrêter avec un rapport ou un plan : la session est déjà réussie.
9. S’il demande l’implémentation, le système prépare la tâche et la délègue au runtime macOS.
10. L’utilisateur reçoit les demandes d’autorisation ou de décision et peut répondre, réorienter ou interrompre.
11. L’exécution se termine par un rapport : statut, résultat, couverture, tests, limites et prochaines actions.
12. L’utilisateur ouvre éventuellement une preview ou une pull request.
13. La session, les objets créés et l’état de l’exécution restent disponibles pour la reprise.

### Moment de valeur documentaire

> « J’ai parlé pendant vingt minutes ; AI Center a compris, challengé et transformé la discussion en décisions et plan structurés, rattachés au bon projet. »

### Moment de valeur opérationnel

> « Depuis mon téléphone, j’ai transformé ce plan en tâche exécutée sur mon Mac et reçu un résultat vérifiable sans ouvrir de bureau distant. »

La v0 doit pouvoir démontrer les deux moments, mais le second s’appuie sur le premier.

## Parcours Center → projet

Une session peut commencer sans projet lorsqu’il s’agit de lecture, brainstorming ou exploration générale.

Lorsque l’intention devient liée à un travail durable, l’agent Center propose :

- de rattacher la session à un workspace et un projet existants ;
- de créer un nouveau projet dans un workspace ;
- ou de laisser la conversation générale si elle ne mérite pas de devenir du contexte de projet.

Le rattachement ne doit pas simplement déplacer un transcript. Il doit identifier les décisions, sources, exigences et tâches à intégrer, puis demander une confirmation proportionnée à leur importance.

## Parcours direct vers un agent spécialisé

Depuis un projet, l’utilisateur peut ouvrir un espace contextuel — par exemple Produit ou Tech — et s’adresser directement à l’agent correspondant.

L’agent spécialisé reçoit :

- les instructions et outils de son rôle ;
- le contexte détaillé de l’espace courant ;
- le résumé du projet ;
- les relations explicitement pertinentes vers d’autres espaces ;
- l’objectif et l’état de la session.

L’agent Center reste accessible comme couche globale, mais ne doit pas être un intermédiaire obligatoire à chaque échange.

## Modèle contextuel minimal requis par les cas d’usage

| Objet | Rôle dans la v0 | Exemples |
| --- | --- | --- |
| `Workspace` | Frontière durable de connaissance | Produit SaaS, client d’agence, side project |
| `Project` | Initiative contextualisée | MVP, refonte onboarding, nouveau billing |
| `ContextSpace` | Périmètre spécialisé | Global, Produit, Design, Tech |
| `Session` | Continuité de la conversation | Brainstorming onboarding du 13 août |
| `KnowledgeEntry` | Atome typé et versionné | Décision, exigence, règle, question ouverte |
| `Task` | Travail à réaliser | Rédiger une spec, implémenter OAuth |
| `Execution` | Tentative réalisée par un agent ou outil | Run Codex sur le Mac |
| `Artifact` | Résultat produit ou référencé | Rapport, document, test, preview, PR |
| `Relation` | Lien explicite entre objets | `depends_on`, `supersedes`, `implemented_by` |

La source de vérité est ce modèle structuré. Une page ou un fichier Markdown est une projection lisible de ces objets, générée pour l’utilisateur ou synchronisée avec Git.

## Hiérarchie Workspace / Projet — règle proposée à confirmer

Le terme « projet » peut désigner soit une application entière, soit une grosse fonctionnalité. Cette flexibilité est intuitive au début, mais elle crée vite des ambiguïtés de propriété et de partage de contexte.

La règle recommandée est :

- **Workspace** = frontière stable de connaissance, de propriété et de permissions ;
- **Project** = initiative bornée ayant un objectif, un état et des livrables ;
- **ContextSpace** = zone de spécialité à l’intérieur du projet.

Heuristique : deux initiatives appartiennent au même workspace si elles partagent durablement le même domaine, les mêmes règles, les mêmes personnes autorisées et une source de vérité commune.

Exemples :

| Situation | Workspace | Projets |
| --- | --- | --- |
| Startup avec un SaaS | Le produit SaaS | Refonte onboarding, billing v2, app mobile |
| Agence avec clients isolés | Un workspace par client ou produit | MVP, migration, nouvelle fonctionnalité |
| Solo builder avec apps indépendantes | Un workspace par app | Foundation, lancement, feature majeure |

Pour préserver une installation simple, la v0 peut créer automatiquement un workspace par défaut et ne révéler la gestion multi-workspaces que lorsque l’utilisateur en a besoin. Le modèle reste correct sans imposer immédiatement toute la hiérarchie à l’écran.

## Règles d’expérience dérivées

1. La dictée doit être lançable en une action depuis la session.
2. Une réponse interrompue ou une perte de réseau ne doit pas perdre le texte dicté ni l’état du travail.
3. Le système indique toujours le scope courant : Center, workspace, projet et espace.
4. La structuration est automatique ; l’utilisateur confirme surtout les décisions importantes.
5. Les objets créés pendant la conversation sont visibles sous forme de changements proposés, pas de formulaires permanents.
6. Un agent spécialisé est accessible directement, mais les décisions importantes remontent à la couche condensée du projet.
7. Le rapport d’exécution précède les détails techniques.
8. L’absence de code n’est pas un échec si un livrable durable a été produit.
9. Le Mac est présenté comme un runtime disponible ou indisponible, pas comme un écran distant.
10. La complexité du graphe ne doit pas apparaître avant d’aider réellement la décision.

## Contenu minimal du rapport de résultat

1. statut global ;
2. résultat obtenu ;
3. couverture de la demande ou des critères ;
4. validations et tests réalisés ;
5. décisions prises et contexte mis à jour ;
6. risques, limites ou questions restantes ;
7. actions disponibles : poursuivre, corriger, ouvrir la preview, ouvrir la PR ou arrêter.

## Critères de succès du persona primaire

- Il choisit spontanément AI Center plutôt que ChatGPT + notes + bureau distant.
- Il initie plusieurs sessions réelles par semaine depuis son téléphone.
- Il peut parler pendant une session longue sans être ralenti par l’interface.
- Il retrouve les décisions d’une session précédente sans les réexpliquer.
- Il obtient un livrable utile même lorsqu’aucun code n’est produit.
- Il délègue au moins certaines tâches au Mac sans revenir au desktop pour les piloter.
- Il sait à quel agent et à quel périmètre il s’adresse.
- Il comprend le rapport et sait quelle action prendre ensuite.

## Hors cible v0

- édition manuelle exhaustive du graphe ;
- revue ligne par ligne et débogage avancé sur mobile ;
- conversation vocale bidirectionnelle temps réel ;
- widget système omniprésent ;
- proactivité permanente ;
- plusieurs organisations et gouvernance complète ;
- orchestration autonome d’un grand nombre d’agents ;
- ingestion automatique de toutes les sources externes.

## Questions à traiter dans le MVP scope

- Quels deux espaces contextuels prouvent le mieux la valeur : Produit + Tech, ou Global + Tech ?
- Quels types de `KnowledgeEntry` sont indispensables au premier parcours ?
- La mise à jour structurée est-elle appliquée automatiquement avec possibilité d’annuler, ou proposée avant validation ?
- Le premier parcours d’exécution cible-t-il la création d’un document, une modification de code très bornée, ou les deux ?
- Quel seuil de risque déclenche une autorisation explicite ?
- Quelle part du Center général doit être visible avant que la section Projets soit validée ?
