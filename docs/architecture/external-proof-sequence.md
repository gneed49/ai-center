# Séquence export → outil → GitHub → preuve

```mermaid
sequenceDiagram
    autonumber
    actor U as Responsable Produit/Tech
    participant A as AI Center
    participant C as Context Compiler
    participant T as Outil externe
    participant G as GitHub API (read-only)
    participant S as Steward

    U->>A: Demander un ContextPack pour une tâche
    A->>C: Compiler(graph version, budget, candidats)
    C-->>A: Pack immutable + raisons + digest
    A-->>U: Export JSON/Markdown
    U->>T: Fournir le pack à l'outil choisi
    Note over T: Le travail est produit hors AI Center
    T->>G: Créer branche/commit/PR/checks
    U->>A: Attacher l'URL github.com
    A->>A: Valider host, owner, repo et type d'objet
    A->>G: GET métadonnées allowlistées + ETag
    G-->>A: PR/commit/checks + head SHA
    A->>A: Ajouter ExternalReference et observation
    A-->>U: Proposer une preuve candidate
    U->>A: Valider la preuve
    A->>S: Recalculer couverture et cohérence
    S-->>A: Couverture + insights + projections stale
    A-->>U: Résultat sourcé et actionnable

    opt Refresh ultérieur
        A->>G: GET conditionnel If-None-Match
        alt inchangé
            G-->>A: 304 Not Modified
            A->>A: Conserver la preuve courante
        else SHA, état ou accès modifié
            G-->>A: Nouvel état / 404 / 403
            A->>A: Ajouter une observation
            A->>A: Marquer preuve stale/unavailable
            A->>S: Recalculer les dépendances
        end
    end
```

## Contrat de sécurité

- Le connecteur accepte seulement des URLs HTTPS sur `github.com` et reconstruit
  lui-même l'URL d'API ; il ne suit pas une URL arbitraire fournie par le client.
- La GitHub App demande uniquement des permissions de lecture sur metadata,
  contents, pull requests, checks et commit statuses.
- L'allowlist importée exclut contenu de fichier, patch et diff.
- Une installation ou clé privée n'est jamais stockée dans une observation,
  une preuve, un log ou un export.
- Les erreurs 401, 403, 404, 429 et 5xx sont visibles, réessayables selon leur
  classe et n'effacent jamais l'état précédemment observé.
