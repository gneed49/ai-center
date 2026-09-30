# Export, archivage et demandes d'effacement

Cette procédure concerne les données d'une société AI Center. L'export et
l'archivage sont implémentés. Les outils opérateur d'effacement d'une société
entière ou d'un projet isolé ont passé une première recette sur fixtures le
22 septembre ; aucune donnée utilisateur réelle n'a été supprimée.
L'anonymisation individuelle n'est pas livrée. Une archive conserve les données et ne
satisfait pas, à elle seule, une demande d'effacement.

## Export d'une société

Un propriétaire authentifié télécharge `GET /api/company/export` dans le scope
de sa société. La réponse JSON porte le format `ai-center-company-data-v1`,
l'identifiant public de société, la date de l'instantané, le nombre de lignes,
les données et la liste des exclusions. Le navigateur doit conserver ce fichier
comme une donnée confidentielle ; la réponse interdit sa mise en cache HTTP.

L'instantané est pris dans une transaction PostgreSQL en lecture seule et en
isolation répétable. Il contient les lignes métier choisies explicitement :
projets actifs et archivés, membres, conversations, versions de connaissances,
artefacts, dépendances et reçus de sources, alertes et historique. Les identifiants
internes sont conservés pour relier les objets. `complete: true` signifie que
toutes les lignes de ce périmètre ont été exportées, pas que le fichier permet
de restaurer une instance complète.

Les clés, enveloppes chiffrées, configurations de connexion, sessions fournisseur,
invitations utilisables ou empreintes de jeton, capacités de traitement en cours
et enregistrements d'idempotence sont exclus. Les catalogues globaux et les
fichiers distants jamais lus/importés ne sont pas des données exportées. Il
n'existe pas encore de fonction d'import de cet export.

Au-delà de 25 000 lignes ou 32 MiB de données projetées, le serveur refuse
l'export avant de charger les lignes : aucun téléchargement partiel n'est
annoncé complet. L'opérateur doit alors préparer un export dédié avec le même
périmètre de colonnes, une transaction cohérente, des contrôles d'appartenance
et un transfert sécurisé. Cette procédure volumineuse reste à réaliser et à
valider sur une copie isolée avant son utilisation sur des données clients.

## Archiver et restaurer un projet

Un propriétaire envoie `POST /api/projects/{public_id}/archive`, un identifiant
d'idempotence UUID et `{"archived":true}`. Le conteneur société est exclu de
cette opération. L'archive retire le projet des listes et contextes actifs,
interdit les nouvelles productions et conserve les lectures historiques. Les
packs qui dépendent de ses sources et leurs livrables deviennent obsolètes.

La restauration utilise le même endpoint avec `{"archived":false}` et un
nouvel identifiant d'idempotence. Elle ne réactive pas les anciens packs : le
contexte doit être compilé de nouveau. L'API des projets archivés permet au
propriétaire de retrouver l'identifiant à restaurer. L'archivage ne retire pas
les objets déjà publiés dans Notion, Linear ou GitHub.

## Traiter une demande d'effacement

1. Identifier la société par son identifiant public exact et vérifier l'autorité
   du demandeur, la cible (projet, personne ou société entière) et les obligations
   de conservation applicables. Consigner cette décision hors des données à
   supprimer, sans recopier leur contenu.
2. Mettre en pause les automatisations de la société, retirer les accès devenus
   inutiles, archiver les projets concernés et arrêter les publications en
   attente. Vérifier qu'aucun worker ni appel fournisseur n'est encore actif.
3. Recenser les dépendances entrantes et sortantes : messages, propositions,
   versions, packs, artefacts, observations, sources interprojets, alertes,
   audits, réservations IA et idempotence. Recenser séparément les fournisseurs
   et destinations externes ; une action locale ne supprime pas leurs copies.
4. Définir si chaque catégorie doit être effacée, anonymisée irréversiblement ou
   conservée avec accès restreint. Une substitution d'identifiant reste une
   pseudonymisation si la personne est identifiable par le texte ou une table
   de correspondance. Un simple masquage dans l'interface n'est pas une purge.
5. Préparer une intervention de maintenance pour ces identifiants exacts. Les
   reçus append-only et les clés étrangères cycliques empêchent une suppression
   naïve en cascade. Ne pas désactiver globalement RLS, les triggers ou les
   contraintes, ni utiliser `session_replication_role` pour les contourner.
6. Tester l'intervention sur une copie isolée avec fixtures `[FICTIF]` : compte
   attendu avant/après, absence de références orphelines, absence de fuite entre
   sociétés, préservation des autres sociétés et vérification des copies de
   contenu dérivées. Une procédure qui ne passe pas ces contrôles ne doit pas
   être exécutée en production.
7. Faire approuver la cible, le périmètre, la durée de conservation résiduelle,
   la fenêtre de maintenance et le rapport de répétition. L'effacement est
   irréversible ; aucune autorisation d'archivage ne vaut autorisation de purge.
8. Après une éventuelle exécution autorisée, conserver un reçu minimal de la
   décision et des contrôles. Suivre aussi l'expiration des sauvegardes et les
   demandes chez les services externes. Informer précisément des données
   supprimées et de celles qui restent conservées, avec leur échéance.

## Sauvegardes et preuve de reprise

L'export métier ne remplace pas une sauvegarde d'exploitation. Le
[smoke de restauration PostgreSQL](postgres-backup-restore.md) vérifie la
restauration du schéma applicatif dans la stack locale isolée. Il ne prouve
ni la restauration d'une infrastructure hébergée, ni une purge de ses
sauvegardes. La politique de rétention, les droits d'accès et le traitement
des effacements lors d'une restauration doivent être fixés pour l'environnement
réel avant l'arrivée de données clients.

## Outil opérateur société entière

`scripts/company-erasure.py` appelle uniquement les fonctions privées de
maintenance. Il exige une connexion directe en rôle `postgres`, fournie par
`AI_CENTER_OPERATOR_DATABASE_URL` via le gestionnaire de secrets opérateur.
Ne pas la saisir dans une commande enregistrée ni la publier dans les logs.
`AI_CENTER_ERASURE_ALLOWED_HOST`, `AI_CENTER_ERASURE_ALLOWED_PORT` et
`AI_CENTER_ERASURE_ALLOWED_DATABASE` fixent
explicitement la cible autorisée. Une connexion distante exige TLS avec
vérification complète et certificat CA configuré pour le client PostgreSQL.

L'aperçu utilise `python3 scripts/company-erasure.py preview UUID_SOCIETE`.
Il retourne seulement le format, le UUID, les comptes par table et un reçu
d'aperçu. Après la décision autorisée et la préparation décrites plus haut,
l'opérateur positionne `AI_CENTER_ALLOW_COMPANY_ERASURE=yes` et lance
`python3 scripts/company-erasure.py execute UUID_SOCIETE --confirm UUID_SOCIETE --receipt RECU`.
Ces identifiants sont des valeurs précises issues de l'aperçu, jamais une
recherche par nom, un joker ou une société choisie automatiquement.

Le serveur refuse si le reçu ne correspond plus aux comptes actuels, si la
société n'est pas en pause, si un projet métier reste actif, si un traitement
reste en cours ou si l'inventaire des tables a changé. Une fenêtre de maintenance
est nécessaire : les verrous bornés empêchent les écritures concurrentes et
peuvent suspendre brièvement celles d'autres sociétés. Les lecteurs restent
disponibles. L'effacement est transactionnel, limité à 100 000 lignes et soumis
à un délai client de 30 secondes. Un refus ou timeout ne produit pas de reçu
de succès ; vérifier l'état par un nouvel aperçu avant toute nouvelle tentative.

Les triggers immuables gardent leurs protections hors de cette opération.
L'exception de suppression exige simultanément le rôle et la session opérateur,
le workspace exact et la transaction exacte ; une GUC forgée par le runtime
ne l'active pas. Les contrôles FK, RLS et triggers restent installés. La purge
supprime aussi les credentials chiffrés du workspace. Elle laisse les catalogues
globaux, les comptes Auth, copies externes, sauvegardes et abonnements personnels
locaux à leurs procédures distinctes.

Conserver le reçu final minimal hors des données supprimées, dans un dossier
opérateur avec accès et rétention définis. La recette automatisée est
`company_context::erasure` et ne peut viser que la stack PostgreSQL jetable
sur `127.0.0.1:55322`. Son résultat doit être consigné avant de déclarer ce
chemin qualifié ; un test local n'autorise pas l'effacement d'une cible réelle.

## Export et effacement d’un seul projet

Le propriétaire peut télécharger l’export depuis le projet, y compris après
archivage. `GET /api/projects/{id}/export` produit un instantané cohérent sans
cache, limité à 25 000 lignes et 32 Mio. Il distingue l’historique du projet
des versions exactes directement citées provenant d’autres projets. Les
conversations voisines ne sont pas parcourues. Les comptes, credentials,
invitations, capacités de reprise et configurations privées restent exclus.
Un dépassement refuse le fichier entier ; aucun export incomplet n’est annoncé
comme complet. Le conteneur de contexte société utilise l’export société.

L’effacement isolé s’utilise uniquement après archivage du projet, pause de
l’automatisation et résolution des traitements en cours. Les mêmes variables
de connexion et de cible que ci-dessus s’appliquent. L’aperçu est
`python3 scripts/project-erasure.py preview UUID_SOCIETE UUID_PROJET`.
L’exécution nécessite `AI_CENTER_ALLOW_PROJECT_ERASURE=yes`, puis
`python3 scripts/project-erasure.py execute UUID_SOCIETE UUID_PROJET --confirm UUID_PROJET --receipt RECU`.

Le reçu expose les dépendances qui empêchent l’opération : liens de graphe,
sources exactes, références structurées et clés étrangères dans les deux sens.
Le système refuse un projet encore lié à des données conservées ; il ne
réécrit pas leurs historiques pour forcer la suppression. Une dérive de
l’inventaire des tables, colonnes ou clés étrangères impose une revue.
Les mentions libres copiées dans du texte ne sont pas détectables exhaustivement.

La transaction prend des verrous dans un ordre fixe et compare aussi le
contenu de toutes les lignes applicatives conservées. Une modification
indirecte d’un autre projet ou catalogue entraîne un rollback. Ce contrôle
est limité à 100 000 lignes et 64 Mio conservés dans toute l’instance ; les
instances plus grandes nécessitent une procédure opérateur revue. Aucun
droit d’effacement n’est accordé à l’API. Les comptes Auth, objets distants
et sauvegardes restent hors de cette opération.

## Traitements abandonnés après une panne

Un délai expiré ne prouve pas l’annulation d’un appel distant. Pour débloquer
une maintenance, arrêter **toutes** les instances API et tous les workers,
maintenir la société en pause continue depuis au moins douze minutes, puis
utiliser `python3 scripts/close-abandoned-work.py preview UUID_SOCIETE`.
Les commandes en cours, appels, réservations, événements en traitement et
publications doivent être antérieurs à cette pause. Un bail actif ou une
activité ultérieure empêche toute clôture.

Après examen de l’aperçu, positionner
`AI_CENTER_ALLOW_ABANDONED_MAINTENANCE=yes`, puis exécuter
`python3 scripts/close-abandoned-work.py execute UUID_SOCIETE --confirm UUID_SOCIETE --receipt RECU --operator-actor UUID_OPERATEUR --workers-stopped`.
L’identifiant opérateur est une référence fournie par l’opérateur et consignée
comme telle ; il ne simule pas une authentification de cette personne.

La clôture est atomique, auditée et ne fait aucun appel réseau. Les capacités
de traitement sont invalidées ; le résultat distant reste explicitement
inconnu. Une ancienne commande clôturée reste terminale même après son
expiration. Les publications déjà parties demandent une réconciliation avec
l’outil distant avant toute décision de republication. Les versions métier
ne sont pas modifiées. Un nouvel aperçu d’effacement est requis après cette
maintenance ; l’ancien reçu n’est plus utilisable.
