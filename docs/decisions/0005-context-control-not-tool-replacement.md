# 0005 — Piloter le contexte sans remplacer les outils de production

> Statut : acceptée
>
> Date : 2026-08-24

## Contexte

Les équipes utilisent déjà des systèmes durables pour produire et coordonner
leur travail : GitHub ou GitLab pour le code, Linear ou Jira pour les tickets,
Notion ou Confluence pour la documentation, Figma pour le design, des bases de
données pour les données et des agents ou CLI spécialisés pour l’exécution.

Le problème prioritaire n’est pas l’absence d’un outil supplémentaire. Il est la
fragmentation du contexte entre humains, agents, équipes, projets et systèmes :
décisions perdues, reformulations, contradictions invisibles, provenance
incertaine et absence de lien entre l’intention et les preuves.

Si AI Center reproduit ces outils ou devient un nouvel IDE, il entre en
concurrence avec des produits spécialisés, impose une migration irréaliste et
dilue sa proposition de valeur.

## Options considérées

1. Construire une suite intégrée remplaçant tickets, documentation, design et
   code.
2. Construire principalement un cockpit et un runner d’agents de code.
3. Construire un plan de contrôle contextuel au-dessus des systèmes existants,
   relié à eux par des références et des adapters limités.

## Décision

Adopter l’option 3.

AI Center possède le contexte partagé et gouverné : connaissances structurées,
provenance, décisions, relations, `ContextPack`, handoffs, contrats, gates,
insights, couverture et audit. Les systèmes externes conservent leurs objets
canoniques et leurs fonctions de production.

La frontière produit est bidirectionnelle :

```mermaid
flowchart LR
    sources["Outils et sources existants"] -->|"références et changements"| center["AI Center\npilotage du contexte"]
    center -->|"ContextPack et contrat"| specialist["Agent ou outil spécialisé"]
    specialist -->|"état, artefacts et preuves"| center
    center -->|"liens, impacts et contrôles"| people["Équipes Produit et Tech"]
    people -->|"décisions et validations"| center
```

## Règles qui en découlent

1. **Intégration avant remplacement** : connecter ou référencer un système est
   le choix par défaut.
2. **Référence avant duplication** : un ticket, document, design, commit ou
   artefact reste canonique dans son outil d’origine.
3. **Production externe** : AI Center ne fournit pas de runner, IDE, terminal ou
   éditeur complet dans son cœur.
4. **Adapters limités** : chaque adapter expose des capacités, des permissions,
   une provenance et un journal de synchronisation explicites.
5. **Portabilité du contexte** : le `ContextPack` et le contrat ne dépendent pas
   d’un fournisseur unique.
6. **Retour de preuve obligatoire** : un handoff externe n’est terminé que si
   son état ou son résultat est rattaché au graphe.
7. **Fonctions natives optionnelles** : une fonction interne de tâche ou de
   documentation pourra exister plus tard pour les équipes sans outil, sans
   devenir une dépendance du cœur.
8. **Mobile différé** : Android et iOS ne participent pas à la consolidation du
   MVP et ne reçoivent aucun investissement de test dans cette phase.

## Conséquences sur le modèle

- introduire `ExternalReference` avec fournisseur, type, identifiant, URL,
  état observé, provenance et date de synchronisation ;
- permettre à `Task`, `Execution`, `Artifact` et `Evidence` de référencer des
  objets externes ;
- distinguer le contrat générique `ToolAdapter` des adapters de modèle utilisés
  par les agents internes de cadrage et d’analyse ;
- représenter l’export d’un `ContextPack` et l’import d’une preuve comme des
  événements auditables ;
- ne pas inférer qu’une entité `Execution` signifie une exécution hébergée par
  AI Center.

## Conséquences produit

- l’alpha doit prouver un cycle réel **contexte → outil externe → preuve →
  contrôle**, et non la capacité d’AI Center à produire du code ;
- le choix du premier connecteur dépend de la valeur démontrée et non du nombre
  d’actions automatisées ;
- les écrans ouvrent les objets dans leur outil source et rendent visibles leur
  provenance, leur fraîcheur et leur état de synchronisation ;
- les métriques prioritaires portent sur la reconstruction de contexte évitée,
  les contradictions utiles, la couverture et la traçabilité multi-outils.

## Révision

Réviser cette décision uniquement si des usages réels démontrent qu’une fonction
native réduit fortement la friction sans imposer une migration, ou si une
contrainte de sécurité exige d’internaliser un traitement. Toute révision doit
préserver la portabilité du contexte, la provenance et la possibilité de
continuer à utiliser les systèmes externes.
