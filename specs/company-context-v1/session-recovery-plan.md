# CC-T11 — reprise des messages et attribution d'équipe

Statut : code et tests ajoutés ; recette PostgreSQL et navigateur à réaliser après migration.

## Problème constaté

Le message utilisateur est enregistré avant l'appel fournisseur, mais sa vue ne
renvoie ni auteur ni identité client. La clé de reprise vit dans une référence
React et disparaît au rechargement. Le serveur connaît déjà l'état durable de la
commande ; il ne le relie pas explicitement au message. Un collègue peut aussi
présenter le même identifiant et contenu, car le contrôle existant ne vérifie que
le contenu. Le libellé « Vous » désigne aujourd'hui tous les messages utilisateur.

## Livraison bornée

1. Ajouter l'auteur des nouveaux messages utilisateurs, sans attribuer un auteur
   aux anciens messages par déduction. Conserver l'identité client existante.
2. Relier le message à sa commande HTTP d'origine et conserver le contenu soumis
   exact : le hash de commande inclut ses espaces, alors que le texte affiché est
   normalisé. La relation vérifie société, projet et acteur ; aucun reçu d'un
   collègue n'est exposé par cette fonctionnalité.
3. Étendre la lecture de session avec auteur, appartenance au demandeur et une
   liste bornée de ses commandes : état, clé, contenu exact, erreur publique,
   délai et possibilité de reprise. Lire cet état ne lance aucun fournisseur.
4. L'interface propose explicitement « Reprendre » pour une commande abandonnée
   ou un échec déclaré réessayable. Elle renvoie le même identifiant client, la
   même clé et le contenu exact. Une opération active se consulte ; elle ne se
   relance pas. Aucun brouillon n'est conservé dans le stockage du navigateur.
5. Refuser avant l'appel fournisseur la reprise d'un message incomplet par un
   autre acteur ou sous une nouvelle commande. Un message terminé reste lisible
   sans nouvel appel. Une commande expirée ou un échec permanent ne reçoit pas
   artificiellement un bouton de reprise.

## Schéma et périmètre

`13_session_recovery.sql` est additif. Les messages anciens restent avec auteur
inconnu et sans commande récupérable. Les lectures de reçus utilisent le contexte
authentifié et l'acteur courant, même si les messages de la conversation sont
partagés. Les conversations archivées restent lisibles ; leur reprise respecte
le garde d'archive existant. Les DTO sont additifs et les anciennes réponses
enregistrées peuvent ne pas contenir ces nouveaux champs.

Le correctif distinct `14_outbox_capacity.sql` sépare le compteur de reports
d'admission du numéro monotone des baux. Un quota reporte à son délai annoncé et
une pause explicite conserve le travail sans épuiser ses essais. Les erreurs de
fournisseur et les véritables délais d'appel gardent leur budget d'échec borné.

## Preuves attendues

- Échec transitoire synthétique, relecture avec nouvelle instance client,
  reprise originale : un message utilisateur, une réponse finale, aucune
  génération supplémentaire lors du replay du succès.
- Travail encore actif : relecture sans appel et concurrence refusée jusqu'à
  expiration du bail ; reprise conservant son identité après abandon.
- Deux membres de la même société : noms corrects, reçu et clé uniquement pour
  l'auteur, tentative forgée du collègue refusée avant le fournisseur.
- Retrait, rôle lecteur, mauvaise société/projet, échec permanent, expiration et
  ancien message sans auteur : aucune reprise privilégiée.
- Contenu avec espaces conservé pour reproduire exactement le hash de commande.
- Reports quota répétés : aucun dead letter, échéance respectée et ancien bail
  incapable d'acquitter une nouvelle tentative portant le même identifiant worker.

Les fournisseurs des tests sont des doubles locaux. Les tests PostgreSQL passent
uniquement par la stack isolée gardée du checkout et restent coordonnés par root.

La garantie vérifiée concerne la commande locale et le replay d'un succès connu.
Si le fournisseur a accepté un appel mais n'a renvoyé aucun résultat observable,
une reprise explicite après abandon peut effectuer un nouvel appel : cette
fonctionnalité ne promet pas une exécution unique dans le service tiers.
