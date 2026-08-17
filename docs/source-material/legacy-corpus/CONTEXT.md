# AI Center — Contexte
_Document vivant — 1 août 2026. Voir aussi VISION.md (cap long terme) et MVP.md (périmètre du premier build)._

## Le problème
Dans une startup IA, le contexte d'un projet est **dilué** partout : Slack, Notion, Linear, Figma, meetings, et chaque équipe (produit, design, sales, tech) utilise ses propres agents IA (Claude, Cursor, Codex, GPT, Grok...) chacun dans son coin.

Conséquences :
- On perd beaucoup de temps en meetings et en écrits pour se re-transmettre le contexte.
- Douleur n°1 : quand un dev passe des specs au code, il doit **reconstruire tout le contexte de tête** (specs éparpillées entre Figma, Notion, meetings...) pour le redonner à son agent de dev. Charge cognitive énorme, surtout avec plusieurs agents en parallèle.
- Résultat : specs minimales, allers-retours avec l'IA, on laisse passer des choses, et des décisions prises à la volée ni tracées ni transmises.

## L'idée (cœur)
Un **agent centralisé** qui garde la connaissance globale d'un projet. La boucle centrale, LE truc à réussir : l'utilisateur **parle**, l'IA **discute, challenge, puis rédige elle-même le contexte**. Le contexte devient un actif partagé, vivant, structuré, qu'aucune équipe (ni aucun agent de dev) n'a besoin de reconstruire de tête.

## Cadrage de départ
- **Nom de travail** : AI Center
- **Premier utilisateur** : Geoffrey seul, avec des « casquettes multiples » pour simuler toutes les parties prenantes (produit, design, sales, tech).
- **Douleur ciblée en premier** : le passage specs → code et la reconstruction manuelle du contexte.
- **Ambition** : side project, à proposer à l'équipe si ça fonctionne.

## Le « waouh » visé (étoile polaire)
« Il a compris tout seul de quoi je voulais parler, et il m'a prévenu d'un truc que je ne savais pas au fil de la discussion. » → L'agent comprend l'intention et fait remonter proactivement ce qu'on ignorait.
