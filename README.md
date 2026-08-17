# AI Center

AI Center est un plan de contrôle contextuel pour projets agentiques. Il structure les décisions, prépare le contexte destiné aux agents, orchestre les passages de relais et vérifie que les livrables restent cohérents avec l’intention du projet.

> **From intent to evidence, without context drift.**

## État du projet

Le projet est actuellement en phase de cadrage produit et de prototype UX. Le premier parcours ciblé couvre :

```text
Intention Produit
→ connaissances structurées
→ Feature Brief
→ ContextPack Tech
→ handoff
→ Technical Delivery Plan
→ preuves et couverture
→ contrôle de cohérence
```

La première surface envisagée est une application web responsive. Les applications natives et les adapters d’agents de code viendront après validation du cœur contextuel.

## Documentation

| Dossier | Rôle |
| --- | --- |
| [`docs/product`](docs/product) | Documentation produit consolidée et maintenue |
| [`docs/source-material`](docs/source-material) | Sources originales ayant alimenté le cadrage |
| [`docs/ux`](docs/ux) | Concepts et futures spécifications d’expérience |
| [`docs/decisions`](docs/decisions) | Décisions durables de produit et d’architecture |
| [`specs`](specs) | Spécifications exécutables ou orientées livraison |
| [`apps`](apps) | Futures applications déployables |
| [`packages`](packages) | Futurs packages partagés |

Commencer par :

1. [`Vision produit`](docs/product/02-product-vision.md)
2. [`Stratégie de différenciation`](docs/product/05-product-differentiation.md)
3. [`Scope MVP`](docs/product/04-mvp-scope.md)
4. [`Personas et cas d’usage`](docs/product/03-personas-and-use-cases.md)

## Concepts UX

### Desktop

![Passage Produit vers Tech — desktop](docs/ux/concepts/credits-v2-handoff-desktop.png)

### Mobile

![Passage Produit vers Tech — mobile](docs/ux/concepts/credits-v2-handoff-mobile.png)

## Développement

Le socle applicatif n’est pas encore initialisé. Les conventions de travail agentique sont décrites dans [`AGENTS.md`](AGENTS.md) et le workflow spec-driven dans [`specs/README.md`](specs/README.md).

## Licence

Aucune licence open source n’est encore accordée. Le dépôt peut être public sans que son contenu soit automatiquement réutilisable. Une licence sera choisie explicitement lorsque la stratégie de distribution sera arrêtée.

