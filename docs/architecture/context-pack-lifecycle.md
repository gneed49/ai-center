# Cycle de vie d'un ContextPack

```mermaid
stateDiagram-v2
    [*] --> Requested: gate courant + tâche explicite
    Requested --> Compiling: idempotency enregistrée
    Compiling --> Failed: budget / provider / source invalide
    Failed --> Compiling: retry même intention
    Compiling --> Current: nouveau pack immutable

    Current --> Exported: JSON ou Markdown
    Exported --> Current: aucune mutation du pack
    Current --> Consumed: handoff accepté
    Consumed --> Current: état toujours lisible

    Current --> Stale: une version source dépendante change
    Exported --> Stale: une version source dépendante change
    Consumed --> Stale: une version source dépendante change

    Stale --> RecompileRequested: action utilisateur
    RecompileRequested --> CompilingNewVersion
    CompilingNewVersion --> FailedNewVersion: échec sans altérer l'ancien pack
    FailedNewVersion --> CompilingNewVersion: retry
    CompilingNewVersion --> Recompiled: nouveau pack et nouveau digest
    Recompiled --> Current: nouvelle version courante

    Stale --> Archived: projet archivé
    Current --> Archived: projet archivé
    Archived --> [*]
```

## Règles de transition

- `Current`, `Exported`, `Consumed` et `Stale` sont des projections de statut ;
  le contenu et les sources d'une version de pack ne changent jamais.
- Une recompilation crée une nouvelle identité. Elle ne réactive pas l'ancien
  pack.
- Un handoff ou une génération depuis `Stale` est refusé ; aucun override
  utilisateur n'est disponible dans l'alpha.
- Un échec de recompilation conserve l'ancien pack stale pour l'audit.
- L'invalidation est calculée à partir des versions sources enregistrées. Le
  fallback global, s'il est nécessaire, doit être marqué et audité.
