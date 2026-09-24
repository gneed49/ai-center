# Effacement opérateur contrôlé d'une société — CC-T12

Approche approuvée pour implémentation le 21 septembre 2026. Cette session ne
supprime aucune donnée utilisateur réelle. Seules des sociétés `[FICTIF]` de
la stack d'intégration jetable servent à la recette.

## Périmètre et contrat

L'opération efface une société entière dans le domaine `app`, après inspection
de son UUID exact, du nombre de lignes par table et de son état de traitement.
Elle n'est exposée ni dans l'API, ni au rôle `ai_center_runtime`. Un script
opérateur sépare l'aperçu et l'exécution avec confirmation de cet UUID et du
reçu d'aperçu. Il ne reçoit aucune URL de base dans les arguments affichés et
n'imprime ni contenu, ni clé, ni mot de passe.

Les fonctions vivent dans `12_data_erasure.sql`, en `SECURITY INVOKER`, sans
droit public, authentifié, service-role ou runtime. Le rôle de maintenance
est vérifié de nouveau à l'exécution. Un rôle qui peut seulement positionner
les GUC applicatives ne peut pas déclencher l'effacement.

L'instance doit être en fenêtre de maintenance : automatisations de la société
en pause, projets métier archivés, aucun appel IA, publication, lease outbox
ou commande idempotente actif. L'opération prend des verrous de maintenance
bornés avant de vérifier l'inventaire et les comptes ; le timeout annule
l'ensemble si un traitement empêche de les obtenir. Ces verrous peuvent
suspendre brièvement les écritures d'autres sociétés mais n'en modifient
aucune donnée. L'absence de données clients ne dispense pas de le documenter.

## Intégrité et immutabilité

Une liste blanche versionnée décrit les tables tenant et leur ordre de
suppression selon les clés étrangères. Toute table applicative tenant inconnue
fait refuser l'opération avant le premier effacement. Les catalogues globaux
sont explicitement exclus. La cible `workspace_id` provient uniquement de la
résolution du UUID de société ; aucune clause libre ni nom de table utilisateur
n'est interpolé.

Les contraintes FK et RLS restent actives. Les triggers append-only et audit
ont une unique exception `DELETE`, valable pour les lignes du workspace exact,
dans cette transaction et sous le rôle opérateur vérifié. Une GUC forgée par
le runtime ne suffit jamais. Aucun `session_replication_role`, désactivation
de trigger, `TRUNCATE`, suppression de schéma ou désactivation globale de RLS.
Les mises à jour des versions immuables restent interdites, même en maintenance.

Les sources transverses disparaissent avant leurs cibles dans cette société.
Le cycle document/version utilise sa contrainte de tête déjà différée. Les
autres contraintes restent immédiates. Si l'ordre est incomplet ou une table
reste référencée, PostgreSQL annule toute la transaction.

Le reçu final ne contient que UUID, horodatage, inventaire et comptes. Il est
conservé hors de la société effacée, dans le dossier opérateur soumis à sa
propre rétention. Le script ne prétend pas effacer les sauvegardes, les comptes
Auth, les abonnements personnels locaux ou les objets publiés à distance.
Ces catégories demandent une action distincte et explicite.

## Preuves requises

- Runtime refusé, y compris avec GUC de maintenance forgée ; immutabilité
  ordinaire toujours effective avant et après l'opération.
- Mauvais UUID, mauvaise confirmation, aperçu périmé, société active ou
  travail actif : refus sans aucune suppression.
- Deux sociétés fictives, conversations, versions, packs, sources croisées
  dans la première, artefacts avec cycle document/version, observations et
  clés chiffrées fictives : effacement de toutes les lignes de la première.
- Empreinte logique des lignes de la seconde et catalogues globaux inchangée.
- Échec injecté ou rollback explicite : toutes les lignes de la cible restent
  présentes ; aucune suppression partielle annoncée réussie.
- Nouvelle table tenant inconnue : arrêt avant suppression.

La purge d'un seul projet reste hors de cette fonction : ses dépendances et
textes copiés dans d'autres projets exigent une politique distincte. L'archive
réversible reste disponible pour ce cas. CC-054 doit refléter ce périmètre au
lieu d'annoncer une purge arbitraire projet/personne.
