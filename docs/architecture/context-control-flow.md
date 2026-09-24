# Flow contextuel de bout en bout

```mermaid
flowchart LR
    U["Responsable Produit / Tech"]

    subgraph AIC["AI Center — plan de contrôle contextuel"]
        I["Intention"]
        K["Connaissances confirmées<br/>atomiques, sourcées, versionnées"]
        G{"Gate courant ?"}
        CC["Context Compiler<br/>obligations + sélection optionnelle"]
        CP["ContextPack immutable<br/>budget, provenance, digest"]
        H["Handoff explicite"]
        ER["ExternalReference<br/>observations append-only"]
        EV["Preuve candidate → valide"]
        CV["Couverture des exigences"]
        ST["Steward<br/>cohérence et obsolescence"]
        RE["Décision humaine<br/>accept / dismiss / resolve"]
    end

    subgraph TOOLS["Systèmes de production existants"]
        T["Codex / CLI / IDE / agent spécialisé"]
        GH["GitHub<br/>repository / PR / commit / checks"]
    end

    U --> I
    I --> K
    K --> G
    G -- "non" --> U
    G -- "oui" --> CC
    CC --> CP
    CP --> H
    H -->|"JSON / Markdown"| T
    T -->|"production hors AI Center"| GH
    GH -->|"lecture allowlistée"| ER
    ER --> EV
    EV --> CV
    K --> ST
    EV --> ST
    CV --> ST
    ST --> RE
    RE -->|"resolve = révision réelle"| K
    ST -->|"invalidation ciblée"| CC
```

## Invariants

1. Le gate, le pack et le handoff portent une graph version explicite.
2. La compilation ne peut inclure qu'une version source existante dans le même
   projet et le même workspace.
3. La session Tech ne recharge pas silencieusement le graphe complet.
4. Une preuve externe reste candidate jusqu'à validation humaine.
5. Le steward conseille et bloque ; seule une décision humaine modifie une
   connaissance confirmée.
6. AI Center n'écrit jamais dans GitHub pendant ce jalon.
