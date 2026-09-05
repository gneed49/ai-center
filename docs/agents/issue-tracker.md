# Suivi des spécifications et tickets

Le dépôt distant est [gneed49/ai-center](https://github.com/gneed49/ai-center).
La consolidation Alpha Context Proof est portée par la [PR #1](https://github.com/gneed49/ai-center/pull/1)
vers `main`.

Pour cette reprise, lire [la spécification](../../specs/alpha-context-proof/spec.md),
[le graphe des tickets](../../specs/alpha-context-proof/tickets.md) et
[le registre des validations](../../specs/alpha-context-proof/validation.md).
Les identifiants ACP-T sont des tickets locaux ; ils ne désignent pas des numéros
d'issues GitHub. L'[issue #2](https://github.com/gneed49/ai-center/issues/2)
suit leur consolidation ; sa fermeture par la PR concerne le socle logiciel.
Les gates opérationnels et durées du plan restent suivis dans le registre de
validation, indépendamment de cette fermeture.

Pour une revue de la consolidation, figer `origin/main` et utiliser
`git diff origin/main...HEAD`. Les preuves antérieures sont datées ; vérifier
les résultats sur le commit relu avant de les présenter comme courants.

## Publication de la reprise

Le push du 5 septembre a été refusé par la revue automatique d'approbation :
le code et la documentation seraient publiés sur un dépôt public sans accord
explicite sur cette divulgation précise. Les commits de reprise restent locaux ;
la PR n°1 distante reste à `8a6e0da`. L'issue n°2, qui renvoie seulement à des
documents déjà publics, a été créée avec succès. Ne pas retenter le push ou
utiliser un autre canal de publication avant l'accord explicite du propriétaire
sur le diff proposé. La PR ne doit pas être marquée prête pour ces nouveaux
changements tant qu'ils ne sont pas publiés et que leur CI n'est pas vérifiée.
