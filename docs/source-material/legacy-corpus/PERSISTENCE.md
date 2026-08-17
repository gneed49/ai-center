# AI Center — Persistance & format (v0, à itérer)
_1 août 2026. Voir DATA-MODEL.md, ARCHITECTURE.md._

## Distinction fondatrice
Deux sujets à ne pas confondre : le **format d'un atome de connaissance** et le **backend de persistance**.

## Backend de persistance
- **Spike / MVP tout début** : fichiers `.md` sur le disque. Zéro infra, versionnable, inspectable.
- **Produit réel** : une **base de données**. Les fichiers plats (SFTP) ne tiennent pas : pas de requêtes, pas d'index, pas de concurrence. On requête sans cesse « toutes les contradictions ouvertes du nœud X », « toutes les entrées qui référencent Y ».
- **Reco : Postgres.**
  - Métadonnées et surtout **arêtes** = données requêtables (table `edges` : source, cible, type, statut, créé-par = le graphe).
  - Corps en prose = colonne `text`.
  - **pgvector** pour le retrieval.
  - Pas de base graphe dédiée (Neo4j) à ce stade : Postgres suffit (traversée via CTE récursives).

## JSON vs markdown = fausse question
En interne, c'est de la **donnée structurée**. JSON = sérialisation naturelle en TS (API, transport). Markdown + frontmatter = **vue** exportable, lisible, git-friendly. Deux projections du même objet ; on n'en élit pas une seule.

## Format AI-native vs lisible humain — résolution
Principe : la **donnée structurée EST le format optimisé IA, ET ce qui rend l'édition humaine sûre**. Pas un compromis.
- Un format « optimisé IA » = des **atomes typés et granulaires** (décision = énoncé + justification + statut + liens), pas un blob de prose. Cette granularité rend l'IA fiable (retrieval, détection sur des faits discrets).
- La même granularité rend l'humain efficace : il corrige UN atome (statut, énoncé, lien) sans réécrire un doc.
- **Source de vérité = les atomes structurés. Le « document lisible » = une VUE générée à la demande.** Le document est une projection, jamais la source. On ne retombe jamais dans « l'IA écrit un doc que l'humain maintient ».

## Intervention humaine — deux niveaux
1. **Via l'agent** (principal) : tu parles, il met à jour les atomes.
2. **Édition directe d'un atome** (trappe de secours) : atome petit et typé → formulaire d'édition trivial et sûr. Bonus : une édition manuelle repasse par le **même détecteur** (l'agent revérifie si le changement crée une contradiction).

## Garde-fou
Ne pas pousser le « 100% optimisé IA » jusqu'à l'**opacité**. Sans lisibilité, pas de confiance — et pour un cerveau d'équipe partagé, la confiance est tout. Garder les atomes en **langage naturel lisible** (énoncés clairs + métadonnées), pas des codes machine ni des embeddings seuls. Sweet spot : lisible pour l'humain, structuré pour l'IA.
