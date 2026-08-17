# AI Center — Data model (v0, à itérer)
_1 août 2026. Socle technique de départ. Voir ARCHITECTURE.md, DESIGN.md._

## Format : markdown-first
Chaque entrée = un fichier `.md` : **frontmatter YAML** (métadonnées structurées) + **corps** en prose markdown. Le graphe = les liens portés par le frontmatter. Type vault Obsidian / codebase : versionnable, lisible, indexable. Une vraie base graphe pourra se brancher par-dessus plus tard.

## Les 3 primitives

### Nœud (scope)
Une partie du projet (Produit, Design, Tech...) ou un sous-nœud.
Champs communs : `id`, `type`, `titre`, `parent`, `résumé` (vue condensée lue par le CTO), `enfants`.

### Entrée (unité atomique de connaissance)
Vit dans un nœud. Types : décision, règle, question ouverte, entité, livrable, note.
Champs communs : `id`, `type`, `titre`, `corps` (markdown), `statut` (proposé / accepté / rejeté / remplacé / ouvert), `rôle-auteur`, `créé_le` / `modifié_le`, `tags`, `nœud`, `liens` (arêtes).

### Arête (relation typée et orientée)
Relie deux entrées (ou deux nœuds pour la structure).
Champs communs : `id`, `type`, `source`, `cible`, `statut`, `créée-par` (humain / agent), `note`.

## Types d'arêtes
- `depends_on` / `references` : besoin du contexte d'une autre entrée (ex : dashboard → modèle de licence). Autorisé cross-projet (voir DESIGN.md, lien vs promotion).
- `informs` : nourrit / influence.
- `supersedes` : remplace une version précédente.
- `contradicts` : conflit détecté (voir cycle de vie ci-dessous).
- `implemented_by` : relie une entrée (décision / règle produit ou tech) à son **implémentation** — commit, PR, fichier. (Réf. externe.)
- `tracked_by` : relie une entrée à un **ticket** (Linear / Jira...). (Réf. externe, plus tard.)

## Cycle de vie d'une contradiction (`contradicts`)
Créée **par l'agent CTO** quand il détecte un clash entre deux entrées. Statuts :
- `open` : non résolue → **rouge**, visible, potentiellement bloquante.
- `accepted` : incohérence assumée volontairement → **ambre**, visible mais non bloquante. Porte une **justification** (pourquoi on l'accepte), **qui** l'a acceptée, **quand**. Ex : « acceptable qu'un crédit périmé ne bloque qu'après les 24h de cache ».
- `resolved` : corrigée (une des deux entrées a changé).

Règles :
- Une contradiction acceptée **reste visible** et **n'est jamais supprimée** → traçabilité / audit des compromis, retour possible plus tard.
- Si une entrée liée change, l'agent **ré-ouvre** la contradiction acceptée (la justification peut ne plus tenir). Accepté ≠ oublié.

## Liens vers le code / les tickets (traçabilité intent → implémentation)
Chaque nœud / entrée produit ou tech peut pointer vers ce qui l'implémente (`implemented_by`) et vers ses tickets (`tracked_by`).
Bénéfice : répondre à « est-ce que cette décision est implémentée ou pas ? » (suivre `implemented_by` ; aucun lien = spec-only).
Classe de contradiction avancée : **drift intent ↔ implémentation** (le code a divergé de la décision). La plus utile pour un lead dev.
⚠️ Périmètre : poser le **lien** manuellement (coller un commit / ticket) = simple, MVP-able. **Détecter automatiquement** qu'un nouveau commit contredit une décision (ou écrase un commit précédent) = avancé : nécessite de surveiller le git + diff sémantique → même famille que les connecteurs, donc **plus tard**.
