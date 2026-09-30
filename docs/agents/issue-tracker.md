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

Après le refus initial du contrôle automatique, le propriétaire a explicitement
autorisé la publication publique du diff proposé le 5 septembre. Le push normal
de `8a6e0da` à `91dc76b` a réussi : les 31 commits de reprise sont publiés sur
`feat/alpha-context-proof`. L'autorisation couvre aussi la mise à jour de la
PR n°1 et la vérification de sa CI. La PR doit être marquée prête pour la revue
du socle logiciel après réussite des contrôles sur son HEAD courant ; cela ne
constitue ni une fusion vers `main`, ni une release, ni un déploiement. L'issue
n°2 reste le suivi de cette consolidation.
