# Équipe et invitations web — validation du 21 septembre 2026

Complément CC-T03, selon `team-access-plan.md`, réalisé dans le mandat
autonome de la spécification Company Context V1.

## Livraison

- L'équipe affiche les noms de présentation servis par `/api/team/members`.
  Chaque rôle, y compris lecteur, peut modifier son propre nom ; les droits
  continuent de dépendre exclusivement des autorisations serveur.
- Les propriétaires créent une invitation collaborateur ou lecteur valable
  un, trois ou sept jours. Le lien se copie explicitement, sans envoi d'e-mail
  implicite. Les invitations et leurs statuts se consultent et se révoquent.
- Un secret aléatoire de 256 bits est créé dans le navigateur ; seule son
  empreinte SHA-256 rejoint la création persistée côté API. Le lien complet
  reste dans la mémoire de l'onglet créateur. Il ne peut pas se relire depuis
  la liste et doit être remplacé s'il a été perdu.
- `/join/:invitationId` est accessible avant les gates connexion/société.
  Le secret reçu reste dans le fragment jusqu'à navigation, afin de survivre
  au remontage de l'application lors d'une connexion dans un autre onglet.
  Aucune copie en localStorage/sessionStorage, clé de cache ou redirect OTP.
- Aperçu limité de l'entreprise/rôle/expiration ; connexion ou création du
  compte par lien Supabase, puis acceptation explicite avec nom affiché.
  Le lien de connexion retourne uniquement à `/auth/callback`. L'écran demande
  de garder l'onglet de l'invitation ouvert puis d'y revenir.
- Un compte déjà membre peut ouvrir l'entreprise sans consommer l'invitation.
  Preview et accept envoient le secret uniquement en body POST sans clé
  d'idempotence ; HTTPS exigé, sauf boucle locale utilisée en développement.
  Une reprise d'acceptation ambiguë reste une consommation atomique serveur.

## Validation locale

Suite web collective : **101 tests réussis dans 25 fichiers** ; lint web
sans avertissement, TypeScript et build Vite réussis.

Les onze tests nouveaux couvrent génération/empreinte/fragment, absence de
secret dans URL HTTP et registre de commande, aperçu avant gate de connexion,
redirect OTP sans secret, acceptation sans société sélectionnée, accès déjà
existant, lien incomplet, retry ambigu, création/copie/révocation par
propriétaire, nom d'un lecteur et protection du dernier propriétaire.
Les comptes et données de test sont exclusivement synthétiques `[FICTIF]`.

La documentation officielle `signInWithOtp`, `onAuthStateChange` et le
changelog Supabase ont été consultés avant l'intégration. Aucun changement de
schéma ni appel réel à Supabase Auth n'a été fait par ce lot frontend.
La réception réelle d'e-mail, le fonctionnement SMTP et le lien entre deux
onglets dans un navigateur réel restent des preuves distinctes du lot de
recette intégré. Aucun e-mail à une personne réelle n'a été envoyé.

Références :
- https://supabase.com/docs/reference/javascript/auth-signinwithotp
- https://supabase.com/docs/reference/javascript/auth-onauthstatechange
- https://supabase.com/changelog
