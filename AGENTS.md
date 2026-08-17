# Instructions pour les agents

Ces règles s’appliquent à tout agent humain ou IA intervenant sur le dépôt.

## Sources de vérité

1. `docs/product/` contient la documentation produit consolidée.
2. `specs/` contient les changements en cours de conception ou de livraison.
3. `docs/decisions/` contient les décisions durables acceptées.
4. `docs/source-material/` est un corpus historique immutable : ne pas le réécrire pour refléter les décisions actuelles.

En cas de contradiction, la spécification active doit la signaler explicitement. Elle ne doit pas modifier silencieusement une décision produit existante.

## Workflow spec-driven

Pour toute évolution substantielle :

1. créer `specs/<feature-slug>/spec.md` depuis le template ;
2. identifier les décisions, règles et documents sources concernés ;
3. définir les critères d’acceptation et les preuves attendues ;
4. ajouter `plan.md` avant l’implémentation ;
5. maintenir la spécification et le plan pendant le développement ;
6. consigner les décisions durables dans `docs/decisions/` ;
7. mettre à jour `docs/product/` lorsque le comportement produit validé change.

## Conventions de contribution

- Préserver les sources originales.
- Préférer les changements petits, traçables et testables.
- Relier les choix techniques à une exigence ou une décision.
- Ne pas introduire une abstraction uniquement pour une hypothèse future.
- Ne jamais traiter un transcript de chat comme une source de vérité implicite.
- Documenter les limites, incertitudes et éléments non couverts.
- Ne pas ajouter de secret, token ou donnée sensible au dépôt.

## Architecture future

La structure `apps/` et `packages/` est réservée au futur monorepo TypeScript. Le choix définitif des frameworks, de la base de données et de l’infrastructure doit être décidé par une spécification ou une décision d’architecture, pas par convention accidentelle lors du premier scaffold.

