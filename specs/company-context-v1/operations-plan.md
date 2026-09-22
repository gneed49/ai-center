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
