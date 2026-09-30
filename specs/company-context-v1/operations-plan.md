# CC-T12 — Préparation de l’exploitation web

Le code web et serveur est livré avec un schéma applicatif précis. Le serveur
refuse de démarrer si les tables et fonctions requises de cette version manquent.
La sonde existante `/api/health` vérifie PostgreSQL ; une indisponibilité n’est
jamais transformée en succès par le proxy.

Le packaging Compose proposé assemble les images API et web sans ouvrir
PostgreSQL ou le port API sur l’hôte. Il exige un fichier de secrets hors dépôt,
une configuration Auth Supabase et un proxy TLS opérateur. Le navigateur ne
reçoit que la configuration publique Auth injectée au build.

Preuves : rôle runtime et migrations réelles sur stack jetable, construction
OCI si le moteur local le permet, validation de configuration et smoke HTTP.
Les sondes sont bornées et ne révèlent aucun contenu utilisateur. Sauvegardes et
restauration restent exercées par le script isolé existant ; qualification
hébergée et alerte réelle exigent une cible identifiée.

Une rotation de la clé de chiffrement sans réencryption détruirait l’accès aux
connexions sauvegardées : elle doit suivre une procédure de remplacement,
réencryption contrôlée et restauration testée. Les valeurs ne sont ni des
arguments de build, ni des constantes, ni des éléments du dossier de preuve.


## Correction de qualification des images — 22 septembre

Le scan non filtré des images locales antérieures signale 365 correspondances
sur l'API (dont 23 critiques et 105 élevées) et neuf sur le web (dont trois
élevées). Ces correspondances ne constituent pas une preuve d'exploitation ;
les correctifs disponibles doivent cependant être appliqués avant qualification.
Le runtime API Debian 12.11 contient notamment curl et son arbre de dépendances
pour un simple healthcheck, et plusieurs paquets de la base restent anciens.

Plan correctif autorisé dans CC-055/056 : runtime Distroless Debian 13 cc signé,
figé par digest, contrôle de santé intégré au binaire, exécution non-root et
arrêt SIGTERM vérifié. Conserver une vraie vérification HTTP + disponibilité DB,
ne pas remplacer le healthcheck par un simple test de processus. Refaire build,
scan non filtré et démarrage réel de l'image. Aucun résultat « zéro CVE » promis.

La revue du build API a aussi trouvé SQLx compilé sans TLS. Ajouter Rustls pour
PostgreSQL et refuser une connexion distante qui ne vérifie pas le certificat
et le nom du serveur. La connexion loopback des recettes reste autorisée. Les
messages d'erreur ne reproduisent jamais la chaîne de connexion. Ajouter tests
de configuration et preuve de négociation TLS locale, puis qualification de la
base distante quand elle sera choisie.

## Version PostgreSQL de la recette — 30 septembre

Complément CC-055/056 : la recette isolée doit attester PostgreSQL 17.11, image
officielle `supabase/postgres:17.11.0.002`, sans changer de major ni de moteur.
La CLI reste figée à 2.114.0 : les versions publiées 2.118.0 et
2.119.0-beta.17 sélectionnent encore PostgreSQL 17.6 par défaut. Le fichier
`supabase/.temp/postgres-version` est une couture explicite du code officiel
2.114.0, lue par le démarrage, le reset et le résolveur des bases temporaires.

Plan : `prepare` produit ce seul pin dans le workdir CI, sans importer de
métadonnées liées au développement. Le marqueur engage version et digest ;
les gardes refusent absence/altération/symlink et overrides incompatibles. La
transition depuis l'ancien marqueur exact est permise à la préparation et son
arrêt reste possible ; elle ne doit pas élargir l'identité ou le nettoyage.
Après démarrage et reset, puis avant les phases SQL, vérifier conteneur, label,
image réellement utilisée, digest officiel par architecture et version serveur
170011. Ne conserver que ces métadonnées techniques, sans URL ou configuration
privée. Une erreur arrête les phases mais n'empêche pas l'arrêt ciblé.

Acceptation : tests hors réseau de préparation/reprise, pin altéré ou manquant,
liens symboliques, overrides, conteneur étranger, ancienne image, digest erroné,
architecture/version inattendue et commande refusée sans fuite de sortie.
La qualification avec conteneurs est séparée : migrations historiques, RLS,
Auth, TLS et restauration doivent passer sur l'image attestée. Ni ces tests ni
ce pin ne qualifient la version d'une cible Supabase hébergée inconnue.

Sources : [release PostgreSQL](https://github.com/supabase/postgres/releases/tag/v17.11.0.002-cli),
[pin CLI 2.114.0](https://github.com/supabase/cli/blob/v2.114.0/apps/cli/src/legacy/shared/legacy-db-image.ts#L95),
[bootstrap](https://github.com/supabase/cli/blob/v2.114.0/apps/cli/src/legacy/shared/db-bootstrap/bootstrap-config.ts#L284).

Qualification locale exécutée : 30 tests de gardes et sept de migrations
réussis ; image officielle attestée, 23 migrations du socle, 32 assertions
pgTAP, 94 scénarios PostgreSQL, TLS et restauration de 60 tables/143 politiques,
Auth API et navigateur à deux comptes réussis. [Rapport de la recette isolée](../ticket-entry-editing/validation-2026-09-30.md).
Les régressions UI découvertes pendant cette recette sont suivies dans ce rapport ;
elles ne sont pas assimilées à des échecs PostgreSQL. CI du commit et cible
hébergée restent des preuves distinctes.
