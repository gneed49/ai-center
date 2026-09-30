# Invitations et identité de l’équipe — réalisation CC-T03

La société se configure dans l’application et ses membres rejoignent le même
espace. Le lot société initial sait créer la frontière de droits et révoquer
un membre ; ce complément rend l’arrivée d’un collègue praticable sans saisir
son identifiant technique.

## Contrat

- Le propriétaire crée une invitation à usage unique, pour un collaborateur ou
  un lecteur, valable au plus sept jours. Il copie le lien et le transmet dans
  le canal de son choix. AI Center ne prétend pas avoir envoyé un e-mail.
- Le lien porte un secret aléatoire dans son fragment, jamais dans le chemin ou
  une chaîne de requête enregistrable par un proxy. L’API ne conserve que son
  empreinte. Le secret ne rejoint ni journal, ni événement, ni réponse stockée
  dans le registre d’idempotence. Un lien perdu se remplace par une nouvelle
  invitation et l’ancienne se révoque.
- La consultation limitée d’une invitation exige ce secret ; elle révèle
  seulement l’entreprise, le rôle et l’expiration. Aucun inventaire de membres
  n’est disponible avant authentification et acceptation.
- L’acceptation exige une identité authentifiée vérifiée côté serveur. Une
  fonction privée étroite verrouille l’invitation, vérifie le secret, son délai,
  le rôle actuel du propriétaire invitant et l’usage unique, puis crée le
  membre dans la même transaction. Rejouer la réponse pour le même acteur ne
  recrée jamais un accès révoqué. Un autre acteur ne peut pas réutiliser le lien.
- Le rôle de propriétaire s’attribue ensuite par la gestion d’équipe existante.
  Les invitations ne s’appuient jamais sur les métadonnées client pour les droits.
- La personne peut indiquer son nom d’affichage ; c’est une information de
  présentation, pas une preuve d’identité ni un attribut d’autorisation.
- Le parcours invité peut demander un lien de connexion/création de compte via
  Supabase Auth selon la configuration de l’instance. L’envoi SMTP et la création
  réelle d’un compte restent des preuves distinctes des tests locaux.

## Séquence et preuves

1. Schéma `07_team_access.sql`, politiques, fonctions de prévisualisation et
   d’acceptation, révocation, journal d’audit et contraintes d’expiration.
2. Routes dédiées ; seules prévisualisation et acceptation ne présument pas
   d’une sélection de société. Aucun autre endpoint ne gagne d’exception.
3. Formulaire d’invitation, copie explicite du lien, statut et révocation ; page
   de réception avec nom de l’entreprise et rôle avant acceptation.
4. Tests de double acceptation concurrente, secret invalide, expiration,
   retrait du propriétaire, membre révoqué, séparation de deux sociétés et
   absence de secret dans les réponses persistées.
5. Recette navigateur avec comptes synthétiques et service local de messagerie.
   Une invitation envoyée à une personne réelle n’est jamais simulée ou faite
   implicitement pendant les tests.
