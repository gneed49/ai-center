# Revue de continuité du contexte — 23 septembre 2026

Référence auditée : `f5c5a28489af18d80106173006c6434d40d59124`.
Ses huit contrôles CI distincts passent, dont images, PostgreSQL et navigateur.
La revue indépendante a néanmoins identifié trois défauts de portée fonctionnelle
que ces tests ne couvraient pas. **G1 reste ouvert.** Les preuves du 22 septembre
restent vraies dans leur périmètre ; elles ne prouvent pas ces trois continuités.

| Priorité | Défaut confirmé | Conséquence | Preuve exigée du correctif |
| --- | --- | --- | --- |
| P1 | Le modèle ne reçoit pas les messages précédents de la conversation | Une question de suivi perd le brainstorm non confirmé | Second tour avec premier échange exact, capture immuable, session étrangère exclue, bornes et omissions |
| P1 | Artefacts publiables manuels et livrables générés séparés | Le relais vers Notion/Linear/GitHub demande de reconstruire le résultat | Génération de brouillons typés, conversion explicite des livrables existants, sources exactes conservées, validation avant publication |
| P1 | Les premières sources/paires du steward sont reprises sans progression durable | Une contradiction entreprise/inter-projets peut rester indéfiniment hors fenêtre ; appels répétés | Plus de 128 sources et 48 paires, ancre nouvelle, voisin ancien inter-projets examiné, déduplication avant IA, progression bornée et visibilité des omissions |

Les corrections sont attribuées à trois lots autonomes avec plans et tests.
La coordination conserve l'intégration des migrations, permissions, inventaires
d'effacement et de l'état publié. Aucune qualification fournisseur, destination
cliente ou déploiement n'est assimilée aux fixtures locales.

## Recette Auth complémentaire

Le parcours existant séparait UI simulée, Auth HTTP réel local et invitations
PostgreSQL. Une nouvelle recette assemble deux identités Auth éphémères dans
le navigateur, création de société, invitation, projets partagés et retrait.
Sa revue indépendante a demandé : transport local sans proxy/redirection,
diagnostics privés, attente de la mutation effective et nettoyage des groupes
de processus. Les correctifs sont appliqués ; la recette finale reste à passer
après stabilisation du serveur modifié par les lots ci-dessus.

## Revue indépendante des correctifs : steward et artefacts

Lecture du code, des contrats et des tests sur le checkout partagé, puis
vérifications locales ciblées sans fournisseur ni base de données lancés par le
relecteur. Les constats ci-dessous portent sur les correctifs en cours après la
référence auditée en tête de document. **G1 reste ouvert** : le responsable de
l’intégration conserve la recette PostgreSQL, navigateur, CI et images.

| Priorité | Défaut et scénario concret | Références | État du correctif et preuve attendue |
| --- | --- | --- | --- |
| P1 | Une ancienne analyse pouvait terminer après expiration/reprise du bail : lecture du jeton sans verrou, puis clôture sans vérifier qu’une ligne avait été modifiée. L’ordre projets → progression pouvait aussi s’opposer à progression → projets lors d’une autre tentative. | [progress.rs](../../apps/server/src/steward/progress.rs), `lock_current` et `finish` ; [steward.rs](../../apps/server/src/steward.rs), `prepare_run` et `persist_completed_run`. | **Implémenté** : verrou progression avant projets dans les deux phases ; clôture conditionnée au jeton et à son expiration, exactement une ligne requise ; rejet transactionnel des résultats anciens et clôture de leur model run. Test préparé `expired_worker_cannot_commit_or_release_a_new_workers_lease` : le travailleur ancien ne peut écrire ni évaluation, ni reçu, ni continuation, ni libérer le nouveau bail. Recette DB à intégrer. |
| P1 | Une continuation en attente créée par A pouvait empêcher B de programmer sa propre reprise après révocation de A. La déduplication portait sur toute la société ; la continuation de A échouait ensuite, laissant des sources en attente sans événement autorisé. | [progress.rs](../../apps/server/src/steward/progress.rs), insertion de `steward.continue` dans `finish`. | **Implémenté** : déduplication par acteur d’origine ; les contrôles d’autorisation restent appliqués à l’exécution. Test préparé `revoked_actors_pending_continuation_does_not_strand_authorized_progress` avec A révoqué et B autorisé. Recette DB à intégrer. |
| P2 | Deux fichiers distincts d’un même corpus GitHub partageaient l’identité du corpus comme parent. Le filtre anti-versions du même objet les excluait donc, même si un README et une configuration se contredisaient. | [company_sources.sql](../../apps/server/src/steward/company_sources.sql), projection `github_code_file_observation` ; [company.rs](../../apps/server/src/steward/company.rs), sélection des voisins. | **Implémenté** : identité de l’observation de fichier comme parent ; le catalogue ne retient déjà que la dernière observation par dépôt/chemin. Test préparé `distinct_files_of_the_same_verified_corpus_are_comparable`. Recette DB à intégrer. |
| P2 | Après conversion committée mais réponse perdue, recharger la page perdait la clé conservée uniquement en `useRef`. Un nouveau clic créait alors un deuxième brouillon. | [deliverable-conversion.tsx](../../apps/web/src/components/artifacts/deliverable-conversion.tsx), gestion de la clé ; [deliverable-conversion.test.tsx](../../apps/web/src/components/artifacts/deliverable-conversion.test.tsx). | **Transmis au lot artefacts, correction en cours** au moment de cette revue. La reprise doit conserver l’identité par acteur/société/projet/livrable et retrouver le reçu. Le test initial vérifie seulement deux clics dans un même montage : ajouter perte de réponse, démontage/rechargement et absence de seconde création. |
| P2 | Après génération à partir de M1, ajout de M2 dans la conversation puis simple correction du titre, l’enregistrement recapturait la session à M2. La provenance de source divergeait alors de `generation.conversation`, restée à M1, sans action explicite de changement de source. | [artifacts/mod.rs](../../apps/server/src/artifacts/mod.rs), `append` et `save_draft` ; [sources.rs](../../apps/server/src/artifacts/sources.rs), source `session`. | **Correctif implémenté et relu** : les sources sélectionnées sont résolues, mais leur instantané antérieur est conservé quand leur identité reste inchangée ; seules les nouvelles sources sont capturées à nouveau. Test de génération → ajout de message → édition à compléter/valider par le lot artefacts, puis recette DB. |

Les trois nouveaux scénarios steward figurent dans
[steward_progress.rs](../../apps/server/tests/company_context/steward_progress.rs),
avec les scénarios de parcours au-delà de 128 sources et 48 paires, du plafond
d’appels et du bail actif/périmé : six tests DB préparés au total. Le parcours
reste explicitement borné par voisinage lexical, avec omissions visibles ; il
ne certifie pas l’absence de contradictions dans toute la société.

Une seconde lecture indépendante confirme dans le code l’ordre des verrous,
la clôture conditionnelle avec rollback, la déduplication par acteur et l’identité
des fichiers : les trois constats steward sont corrigés au niveau logiciel.
La clôture d’un model run lorsque l’acteur perd ses droits pendant l’appel relève
du correctif transversal de fiabilité repris par l’intégration ; elle doit être
testée distinctement du remplacement d’un bail entre deux travailleurs autorisés.

Preuves locales obtenues par le relecteur :

- Clippy serveur ciblé (`--lib --test company_context`, avertissements refusés) :
  réussi après correction de l’ordre des verrous ; les nouveaux tests DB
  compilent, ce qui ne prouve pas leur exécution sur PostgreSQL.
- Tests unitaires du steward : **9 réussis, 0 échec**.
- Tests UI ciblés de génération et conversion : **4 réussis, 0 échec**, dans deux
  fichiers, avant les deux corrections P2 artefacts. Ces tests couvraient la
  génération typée, la synchronisation du Markdown, le rejeu dans le même
  montage et la restauration d’une génération avec lecture seule du reçu ; ils
  ne couvraient pas encore les deux scénarios P2 ci-dessus.
- Aucun nouveau P1 confirmé dans la revue ciblée des artefacts : citations
  autorisées, versions de connaissances/artefacts exactes, résultat et reçu
  enregistrés atomiquement, reçu limité à l’acteur/projet, séparation des
  identités navigateur et garde d’expiration sous verrou examinés dans le code.

Ces preuves n’établissent ni recette DB finale, ni fournisseur réel, ni
publication externe. La clôture des constats exige les résultats d’intégration
et ne ferme pas automatiquement G1.
