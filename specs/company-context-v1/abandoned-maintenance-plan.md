# Solder les traitements abandonnés avant maintenance

Gate d'opérabilité ouvert : la purge refuse actuellement tout `model_run`
encore `running` et toute commande `processing`, même après expiration de son
bail. Une panne suivie du retrait de l'auteur ou de l'archivage empêche parfois
la reprise applicative. Ne pas contourner cette garde en ignorant les statuts.

Plan accepté par la coordination le 21 septembre ; implémentation et recette en cours le 22 septembre : procédure opérateur privée distincte, sans droit
runtime, en `SECURITY INVOKER`, avec aperçu et reçu, UUID exact de société et
confirmation explicite que tous les processus serveur/workers sont arrêtés.
La pause persistante doit être continue depuis au moins 12 minutes : plus que
la durée maximale d'appel de 600 secondes, le bail maximal de 11 minutes et
une marge. Le contrôle est effectué sur `workspace_automation_controls`
`enabled=false` et `updated_at`, de nouveau sous verrous de maintenance bornés.

Seuls les traitements antérieurs à cette pause sont éligibles. Aucun bail
encore valide, aucune activité postérieure à la pause ni nouvelle commande ne
peut être soldé. Un délai expiré ne prouve pas qu'un fournisseur distant a
annulé sa tâche ; le reçu doit conserver « résultat distant inconnu ».

Dans une transaction unique, marquer les réservations expirées `cancelled`,
les appels abandonnés `cancelled` avec une classe d'erreur opérateur explicite,
les commandes expirées `failed` avec une réponse terminale non réessayable et
une nouvelle génération qui invalide tout ancien détenteur de bail. Les
événements `processing` dont le bail est expiré et les publications bloquantes
demandent la même inspection : l'opérateur les clôture explicitement, sans
incrément artificiel d'essais ni relance distante. Une publication dont le
résultat externe est inconnu ne doit jamais être automatiquement recréée.

Les versions métier, messages et sources immuables restent intactes. Un audit
minimal attribué à la maintenance indique les comptes et raisons ; aucun
contenu, secret ni affirmation de succès fournisseur n'est ajouté. La purge
conserve ensuite ses gardes habituelles et exige un nouvel aperçu.

Recette requise : pause trop récente, bail encore actif, activité post-pause,
mauvais UUID, absence de confirmation d'arrêt et runtime refusés ; anciens
traitements fictifs seulement soldés ; autre société inchangée ; ancien lease
inutilisable ; aucun appel réseau ; rollback sans clôture partielle. Le rôle
opérateur ne dispense pas de ces conditions. Le lot est une migration additive distincte ; schema12 conserve ses refus de travail actif. Les preuves sur fixtures sont requises avant acceptation de ce complément.
