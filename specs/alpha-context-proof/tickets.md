# Tickets de reprise — Alpha Context Proof

Le plan directeur fourni par le propriétaire le 4 septembre 2026 est comparé
à la branche existante `feat/alpha-context-proof`, baseline `8a6e0da`.
La PR de consolidation reste [#1](https://github.com/gneed49/ai-center/pull/1).
Le périmètre et les critères de preuve sont ceux de [spec.md](spec.md) et
[validation.md](validation.md). L'instruction d'implémenter ce plan autorise
la reprise du périmètre existant ; les gates de promotion restent distinctes.

Les tickets ci-dessous forment un graphe. Un ticket dépendant commence après
intégration de ses prérequis. Les branches de travail sont fusionnées dans la
branche de consolidation après leurs validations ciblées.

| Ticket  | Objet                                                  | Dépendances   | Statut  |
| ------- | ------------------------------------------------------ | ------------- | ------- |
| ACP-T01 | Intégration locale isolée                              | Aucune        | Prêt    |
| ACP-T02 | Invalidation ciblée des versions sources               | Aucune        | Prêt    |
| ACP-T03 | Références GitHub repository, PR, commit et checks     | Aucune        | Prêt    |
| ACP-T04 | Certification des parcours desktop manquants           | T01, T02, T03 | À faire |
| ACP-T05 | Revue du protocole d'évaluation et gates de promotion  | Aucune        | À faire |
| ACP-T06 | Revue Standards/Spec, corrections et preuves courantes | T01–T05       | À faire |

## ACP-T01 — Intégration locale isolée

**Exigences :** phase 1, ACP-070, ACP-076, ACP-077.

Le script d'intégration courant lance un reset dans le projet Supabase de
développement `AICenter`. Construire un environnement de test avec identité,
configuration, ports et cycle de vie propres. Toute opération destructive
doit vérifier sa cible avant exécution. Adapter CI et documentation.

**Acceptation :** stack jetable complète, migration baseline→alpha, RLS,
tests PostgreSQL, auth et parcours réel ; teardown de la seule stack de test.
Un test de garde démontre qu'une cible de développement est refusée avant
reset. Aucun secret réel ni base de développement nécessaire.

## ACP-T02 — Invalidation ciblée

**Exigences :** ACP-017, ACP-041, ACP-044, ACP-045, ACP-046.

`invalidate_dependent_projections` ignore actuellement les versions modifiées.
Propager l'obsolescence uniquement aux packs, livrables, preuves et couvertures
qui en dépendent. Conserver le fallback global seulement quand la provenance
est insuffisante, avec cause explicite et audit. Respecter les contraintes de
handoff sur la graph version courante et l'immutabilité historique.

**Acceptation :** graphe à deux branches, révision d'une branche, projections
de l'autre conservées ; résolution atomique, fallback tracé, refus de génération
stale et recompilation vérifiés sur PostgreSQL.

## ACP-T03 — Périmètre GitHub et rate limits

**Exigences :** phase 7, ACP-050 à ACP-056.

Étendre le chemin d'import actuellement limité aux URLs de PR aux repositories
et commits. Les checks restent observés sur le SHA pertinent. Préserver
allowlist, URLs canoniques, API officielle, absence de redirects, historique,
validation humaine et compatibilité des références PR existantes. Classifier
une limite GitHub HTTP 403 avec ses headers comme transitoire : elle ne doit
pas être traitée comme perte d'accès durable.

**Acceptation :** contrats faux HTTP pour chaque type, URLs invalides/SSRF,
ETag, changement de SHA, 403 rate limit, perte d'accès, retries et aucune écriture
GitHub ; parcours d'import cohérent dans l'interface.

## ACP-T04 — Certification desktop

**Exigences :** ACP-020, ACP-034, ACP-070, ACP-071 ; phases 6 et 8.

Compléter les lacunes constatées dans les scénarios existants : offline et
reconnexion avec saisie préservée, réponse perdue et replay, résolution en conflit,
parcours complet plan/preuve/couverture/historique, erreurs par route et
ressources hors scope. Vérifier les comportements dans le navigateur avant
figer les tests. Les preuves simulées et full-stack restent identifiées.

**Acceptation :** suites significatives sur Chromium 1440×900 et 1024×768,
Firefox desktop ; zéro erreur inattendue, duplication, fuite ou violation axe
critical/serious. La preuve native Tauri est distincte du build Linux.

## ACP-T05 — Évaluation et promotion

**Exigences :** ACP-072 à ACP-076 ; phases 9 à 11.

Vérifier les contrôles exécutables du harness : budget ferme, calibration,
sources autorisées, corpus, aveugle, répétitions, annotations et seuils. Corriger
les écarts reproductibles sans inventer de résultats ou de consentements.
Conserver un registre explicite des prérequis externes et durées d'exploitation.

**Acceptation technique :** tests offline des refus et calculs, manifests et
commandes opérationnelles utilisables. **Acceptation live :** corpus autorisé,
fournisseurs configurés, évaluations humaines et périodes d'usage réalisées.
L'acceptation technique ne ferme pas les gates G9 à G11.

## ACP-T06 — Intégration et revue

**Exigences :** ACP-006 et toutes les exigences touchées.

Revue en deux axes Standards/Spec de `origin/main...HEAD`, corrections sur
worktree dédié, validations adaptées et documentation datée. Mettre à jour la
PR après vérification du contenu destiné au dépôt public. Nettoyer les worktrees
d'implémentation après fusion. La release reste conditionnée aux gates du plan.

**Acceptation :** écarts techniques corrigés, résultats localisables, statut
exact des gates restant ouverts, branche unique et PR relisible.

## Écarts additionnels confirmés pendant la lecture du code

Ces tickets complètent le graphe initial. T04 et T06 attendent également leur
intégration. T07 attend T02 ; T08 attend T03 et T07 afin de limiter les conflits
sur les contrats et l'orchestration métier.

### ACP-T07 — Évaluation structurée de couverture

**Exigences :** phase 4, ACP-015, ACP-018, ACP-035 à ACP-039, ACP-056.

Introduire une opération de modèle distincte de la génération du plan pour
évaluer le mapping des exigences et les lacunes, exclusivement depuis le pack
et le plan produits. Valider les UUID, l'appartenance au pack, l'exhaustivité et
l'unicité des exigences. Persister un model run propre avec versions du contrat,
usage et statut. Garder les appels réseau hors transaction, le contrôle optimiste
et la commande idempotente. Une appréciation du modèle ne valide jamais une
preuve : la couverture effective dépend des preuves humaines courantes.

**Acceptation :** faux provider validant les cinq opérations structurées,
rejet des sources inventées/dupliquées/omises, échec explicite sans livrable
partiel, traçabilité distincte du run de plan et du run de couverture ; aucun
contenu spécifique à Credits v2 dans le moteur réel.

### ACP-T08 — Traçabilité du travail externe

**Exigences :** phase 7, ACP-020, ACP-021, ACP-050, ACP-054 à ACP-056.

Relier explicitement les références et preuves au ContextPack transmis en
réutilisant les enveloppes existantes `tasks`, `executions`, `execution_events`
et `artifacts`, ainsi que les arêtes `tracked_by`. Les états décrivent le travail
observé dans l'outil externe ; aucun runner ni action GitHub en écriture.
Conserver les imports historiques en lecture et fournir le lien explicite dans
le parcours de preuve. Vérifier toutes les relations au workspace et au projet.

**Acceptation :** export/handoff → référence externe → enveloppe et artefact →
preuve candidate → validation humaine ; reload et retry conservent la même
chaîne sans duplication. Les refreshes ajoutent des événements d'observation
et préservent l'historique en cas de changement de SHA ou de perte d'accès.
