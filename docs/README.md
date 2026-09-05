# Documentation

Commencer par [la reconnaissance complète de l’état actuel](current-state-audit.md)
pour distinguer les fonctions implémentées, les démonstrateurs ciblés, les
validations reproduites et la roadmap.

La documentation est séparée selon son niveau d’autorité :

- `product/` : vision et décisions produit consolidées ;
- `architecture/` : flows et invariants du plan de contrôle contextuel ;
- `source-material/` : documents historiques fournis en entrée ;
- `ux/` : concepts, parcours et futures spécifications visuelles ;
- `decisions/` : décisions acceptées de produit ou d’architecture.
- `operations/` : procédures reproductibles de provisionnement, connexion et
  reprise, dont le [smoke PostgreSQL backup/restore](operations/postgres-backup-restore.md).

Les changements encore en discussion appartiennent à `../specs/` avant d’être intégrés aux documents consolidés.

Le jalon actif est la spécification
[`Alpha Context Proof`](../specs/alpha-context-proof/spec.md), avec son
[plan séquencé](../specs/alpha-context-proof/plan.md) et son
[registre de validation](../specs/alpha-context-proof/validation.md).

La décision structurante est
[`0005 — Piloter le contexte sans remplacer les outils de production`](decisions/0005-context-control-not-tool-replacement.md) :
AI Center gouverne le contexte partagé et se relie aux systèmes existants, qui
restent responsables de leurs objets canoniques et de la production.
