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
