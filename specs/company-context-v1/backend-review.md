# Revue ciblée backend Company Context V1

Référence : `966ce9bbb597327e0ebbc3084836c6dedd759eca`, changements suivis et
nouveaux modules non encore commités du candidat local du 21 septembre 2026.
Revue de spécification en lecture seule pendant la recette commune. Les
constats ci-dessous ne constituent ni une recette distante ni une preuve
fournisseur réelle.

## Constats à corriger ou à qualifier

| Priorité | Constat | Preuve et conséquence | Correction attendue |
| --- | --- | --- | --- |
| P2 | Conversion concurrente des verrous lors d'une relation de graphe | `company/graph.rs::endpoint` prend `FOR SHARE` sur les deux scopes, puis `create_edge` met leurs compteurs à jour. Deux transactions peuvent détenir simultanément les verrous partagés et attendre mutuellement leur conversion en verrou d'écriture ; une commande échoue par deadlock. | Verrouiller d'abord les scopes distincts par identifiant croissant en `FOR UPDATE`, puis vérifier les endpoints. Recette avec deux relations distinctes concurrentes. |
| P3 | Une ligne de trop dans une citation de code tronquée | `steward/company_sources.sql` calcule `line_end` par nombre de retours à la ligne + 1. Un extrait de 6000 caractères qui se termine exactement par un retour à la ligne cite la ligne suivante, encore non lue, si le fichier continue. | Compter la dernière ligne seulement si l'extrait ne se termine pas par un retour à la ligne ; vérifier vide, une ligne, newline final et coupure exacte. |
| Gate ouvert | CC-054 n'est pas entièrement livré | La spécification demande export et suppression ciblée exercée sur fixtures. L'export propriétaire et l'archive réversible sont codés ; la purge/anonymisation reste une procédure à préparer et à répéter. | Garder l'acceptation de suppression ouverte ; ne pas présenter l'archive comme un effacement. Voir `docs/operations/company-data-retention.md`. |
| Preuve manquante | Jonction observation de code → Steward | Les tests de lecture GitHub et les tests Steward de métadonnées sont séparés. Le nouveau SELECT de code, son FK typé et ses citations ne sont pas encore exercés ensemble au moment de cette revue. | Fixture `[FICTIF]` avec contenu réellement présent, observation insuffisante et nouvelle version ; vérifier exactitude des sources, état inconnu et obsolescence. |

## Invariants vérifiés dans le code

- Les rôles sont relus par les politiques RLS : le rôle de la GUC doit encore
  correspondre au membre accepté. Un ancien état `owner` ne conserve pas
  d'accès après rétrogradation en base.
- Les sources de société et de projet gardent leurs identifiants de versions
  et leurs scopes explicites. Les FK historiques restent en place ; les
  provenances transverses utilisent des tables additives.
- Une révision de connaissance ou une nouvelle version validée d'artefact
  invalide ses packs dépendants ; le simple changement d'un compteur de graphe
  n'expire pas une source exacte restée valable.
- Un brouillon d'artefact ne remplace pas la dernière version validée dans
  le contexte ; une validation conserve les sources capturées au brouillon.
- Les métadonnées GitHub et reçus incomplets imposent un écart impossible à
  vérifier ; le serveur neutralise une conclusion trop forte du modèle.
- L'export sélectionne explicitement les colonnes métier et refuse un volume
  supérieur aux limites avant le chargement, dans un instantané cohérent.
- Le contrôle IA partage un verrou de quota entre processus, réserve avant
  appel, vérifie le rôle effectif pendant l'appel et distingue coûts connus et
  inconnus. Deux remarques de revue sur rôle viewer et dépassement du délai
  pendant le polling ont été corrigées par le lot automation avant la recette.

Les corrections et résultats de tests doivent être ajoutés au registre de
validation du candidat ; ce document conserve le constat initial et ses limites.

## Résolution — 22 septembre 2026

Coordination : les correctifs sont enregistrés dans `9be97b6`, dont la CI
PostgreSQL passe. `concurrent_graph_relationships_lock_scopes_in_one_order`
exerce les verrous ordonnés ; les tests de citations vérifient le newline final
et les extraits bornés. `steward_code_sources_keep_exact_commit_lines_and_mark_partial_evidence_unknown`
vérifie maintenant la jonction observation → steward et son incertitude.
L'effacement opérateur de société/projet est implémenté et exercé sur fixtures,
y compris le rollback si une relation inattendue risque d'affecter un autre
projet. Les contraintes de pause, dépendances et sauvegardes restent explicites
dans la procédure. L'exercice sur une cible hébergée relève toujours de G3.
