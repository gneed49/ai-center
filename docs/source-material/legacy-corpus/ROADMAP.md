# AI Center — Roadmap & phases (MVP)
_1 août 2026. Découpage de construction. Voir CONTEXT.md, VISION.md, ARCHITECTURE.md, DESIGN.md, DATA-MODEL.md._

## Principe de séquencement
Prouver le plus tôt possible le **différenciateur** (détection de contradiction sur un graphe scopé), et repousser l'infra lourde (multi-agents, connecteurs, auto-ingestion, vue graphe éditable, vocal, multi-projets). **Un workspace = un projet** pour tout le MVP.

## Phase de dé-risquage — SPIKE (avant tout le reste) ⚡
But : vérifier que le cœur est atteignable, au coût le plus bas, AVANT de construire le moindre échafaudage.
- Aucune UI, aucune persistance. 2-3 entrées écrites en dur dont deux se contredisent (ex : règle licence vs cache 24h).
- On prompte un agent « CTO » → repère-t-il la contradiction de façon fiable et l'explique-t-il bien ?
- Décision : ça marche → on construit. Ça ne marche pas → on repense la prémisse avant d'investir.
- STATUT : réalisé (dossier `spike/`). Échafaudage, faux graphe (12 nœuds / 22 entrées), détecteurs structurels et suite de tests = verts. Reste à valider la détection sémantique en live (avec clé `ANTHROPIC_API_KEY`).

## Phase 0 — Socle : le graphe en fichiers
- Data-model markdown-first (nœud / entrée / arête = fichiers `.md` + frontmatter).
- Créer un projet, générer la structure de nœuds de base (Produit, Tech). CRUD + persistance, pas encore d'IA.
- Livrable : je crée un projet et j'y dépose des entrées structurées.

## Phase 1 — L'agent qui interviewe et rédige (boucle centrale)
- Fenêtre de chat unique ; un agent qui endosse le rôle selon le nœud courant (prompt).
- Il discute, challenge, puis **écrit les entrées** (décisions, règles) — plus de saisie manuelle.
- Livrable : « je parle, l'IA structure et rédige ». Premier moment de valeur.

## Phase 2 — Détection de contradiction (LE « waouh ») ⭐
- L'agent global lit la couche condensée des décisions et pose une arête `contradicts` quand il détecte un clash → alerte rouge.
- Cycle de vie : open → accepted (justification / qui / quand) → resolved ; ré-ouverture si une entrée liée change.
- Livrable : le différenciateur prouvé de bout en bout. Phase la plus risquée = à valider tôt (cf. spike).

## Phase 3 — Retrieval & navigation
- Index métadonnées + vectoriel sur les entrées ; l'agent suit les arêtes pour charger le contexte pertinent (pas tout).
- Livrable : l'agent reste tenable quand le graphe grossit, détection fiabilisée.

## Phase 4 — Vue graphe + liens code / tickets
- Vue graphe en **lecture seule** : se balader, voir nœuds / arêtes, contradictions rouge / ambre.
- Liens manuels `implemented_by` / `tracked_by` (coller commit / ticket).
- Livrable : le graphe devient tangible + amorce de la traçabilité intent → implémentation.

## Au-delà du MVP (Phase 5+)
Couche workspace multi-projets + CTO inter-projets + promotion → connecteurs (Slack / Notion / Linear / Figma / git) → auto-ingestion → détection de drift auto → vue graphe éditable → vocal / omniprésence.

## Explicitement HORS MVP
Multi-agents autonomes, connecteurs branchés, auto-ingestion, vue graphe éditable, vocal, omniprésence sur chaque page, multi-projets.
