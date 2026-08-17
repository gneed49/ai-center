# AI Center — Design & UI (premières idées, en vrac)
_1 août 2026. Premières idées, à itérer. Voir CONTEXT.md, VISION.md, ARCHITECTURE.md, MVP.md._

## Structure de l'app (hiérarchie)
- App AI Center = pour une entreprise. On y crée un **Workspace**.
- Workspace = une entreprise OU un « projet au sens global » (une application entière, un client). Le workspace est lui-même un **graphe de connaissance**.
- Dans un workspace : plusieurs sections à terme ; la première = **Projets**.
- Un **Projet** est granulaire, à géométrie variable :
  - soit une **app standalone complète** (cas entrepreneur solo, projets sans lien entre eux),
  - soit une **grosse feature** (plusieurs mois) d'une app SaaS existante (cas startup, projets qui partagent du contexte).
- Chaque projet = son propre **graphe de connaissance** (nœuds Produit / Design / Tech / ...).

## Décision d'architecture à trancher : la profondeur du graphe
Risque : un seul graphe général monolithique → profondeur énorme, usine à gaz, agents noyés.
Reco proposée (à valider) : NE PAS fusionner en un graphe géant.
- **Graphes scopés par projet** : les agents de section (dev, produit...) ne voient QUE le graphe de leur projet courant. Profondeur bornée.
- **Couche inter-projets fine** au niveau workspace : pas le détail, juste les décisions / entités / résumés partagés entre projets. C'est cette couche condensée que l'agent global (CTO) lit pour faire le lien entre projets.
- Résultat : « workspace » et « projet » sont le **même primitif à deux échelles** (un scope + son graphe), mais le contexte des agents reste scopé à un niveau à la fois — pas de traversée du graphe entier.
- Pour le MVP : un workspace = un projet ; on ne construit QUE le graphe projet. Le lien multi-projets (couche workspace) vient après. Le schéma de données doit juste le permettre.

## Contexte partagé entre projets : lien vs promotion
Question : quand un projet a besoin du contexte d'un autre (ex : un dashboard data a besoin de comprendre le système de licences défini dans le projet « licence manager »), comment le représenter ?

Règle proposée — le critère est la **propriété (ownership)** :
- **Par défaut = LIEN.** Si un concept est *possédé* par un projet (il y est né, ce projet est sa source de vérité), il **reste chez lui** ; les autres projets créent une **arête de référence** et vont chercher le contexte à la demande (retrieval). Source de vérité unique préservée : le « licence manager » garde la logique de licence, le dashboard **pointe** vers elle sans la copier.
- **Promotion (exception) = couche partagée workspace.** Réservée aux concepts *possédés par aucun projet* : entités canoniques (User, License, Org), glossaire / domaine, règles métier transverses. Ils **vivent** au niveau workspace et tous les graphes projet les référencent.

⚠️ Piège à éviter : « promouvoir » ne veut PAS dire **copier** vers une couche globale. Copier = duplication → les deux versions divergent. Promouvoir = **déplacer la source de vérité** vers le workspace ; le projet la référence.

Discipline : la promotion doit être un **acte délibéré** (manuel ou **suggéré par l'agent CTO**), jamais automatique — sinon tout remonte et la couche partagée redevient un monolithe. Un agent qui **suggère** un lien inter-projets OU une promotion = usage parfait de la proactivité (cf. « lien logique suggéré »).

## Deux vues UI pour un projet
- **Vue classique** (MVP) : fenêtre conversationnelle IA + sidebar avec dossiers / fichiers par section. Familier, rapide à construire.
- **Vue graphe** (différenciateur) : on se balade dans le graphe, on inspecte nœuds et arêtes. Commencer en **lecture seule / visualisation** ; l'édition dans le graphe vient après.
- La **même fenêtre de chat partout** ; l'agent change selon le nœud / la section courante (voir ARCHITECTURE.md).

## Récupération de l'info (retrieval) — comme une codebase
L'agent ne peut pas « tout charger ». Modèle type codebase (Cursor & co n'avalent pas tout : ils indexent / grep / embeddent).
- Chaque livrable / contexte de nœud doit être **structuré** et porter des **métadonnées** (titre, tags / mots-clés, nœud, type, date, rôle-auteur).
- Récupération = **métadonnées + recherche vectorielle** sur titres / mots-clés → on sait vite OÙ est l'info, puis on va la chercher.
- Sert aussi la **navigation** dans le graphe et l'agent principal du projet.

## Les agents « globaux » à deux niveaux
- Chaque **projet** a son agent global (connaissance globale du projet).
- Au-dessus, le **CTO** = le plus global de tout l'AI Center ; il fait le lien **entre projets**.
- Déclenchement : pas à chaque token. Il tourne quand des **décisions / choix** se posent, recoupe avec son contexte condensé, et réagit.

## UX de la proactivité (surfaces possibles, à itérer)
- Notifications (contradiction détectée, décision non transmise).
- Blocage d'un chemin / nœud marqué en **rouge**.
- Suggestions inline pendant la rédaction.
- Enjeu central : **pertinence + timing** (cf. risque « Clippy » dans VISION.md).

## Prochaines étapes
1. Trancher la hiérarchie workspace / projet (ci-dessus).
2. Schéma de données du graphe (nœud, arête, « entrée de décision », livrable, métadonnées).
3. Découper les premières parties → specs fonctionnelles puis techniques.
