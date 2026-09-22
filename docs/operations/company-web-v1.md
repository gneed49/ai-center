# AI Center — exploitation de la V1 web entreprise

## Portée et preuves

Ce dossier couvre une instance privée pour une entreprise, son équipe et ses
premiers clients invités. Une instance peut héberger plusieurs sociétés isolées.
Au sein d’une société, les projets sont partagés avec tous ses membres selon
leur rôle. Le produit ne promet pas de projets privés par équipe.

Le registre de livraison est dans
[les tickets Company Context](../../specs/company-context-v1/tickets.md).
Une image construite, un test local ou un connecteur simulé ne prouve ni un
hébergement public, ni l’accès à un compte tiers, ni la délivrabilité SMTP.

## Ensemble à déployer

- API Rust et frontend React servis sur la même origine ; PostgreSQL privé sous
  le rôle `ai_center_runtime`, jamais un rôle administrateur dans l’API.
- Auth Supabase ; callbacks HTTPS autorisés explicitement sur `/auth/callback`.
- Proxy TLS géré par l’opérateur devant le service web HTTP interne.
- Clé de chiffrement serveur persistante et sauvegardée séparément pour les
  connexions IA personnelles et les outils de travail.
- Logs d’exploitation privés ; ne pas activer la journalisation des corps HTTP,
  des headers Authorization, des URL d’invitation complètes ou des secrets.

Les images se construisent depuis un commit qualifié avec
[les Dockerfiles](../../deploy/oci/server.Dockerfile). Le frontend exige la
configuration Auth publique au build. `VITE_SUPABASE_ANON_KEY` est une clé
publique ; aucun secret privé ne doit porter le préfixe `VITE_`.

[Compose](../../deploy/compose.yaml) attend `AI_CENTER_API_IMAGE`,
`AI_CENTER_WEB_IMAGE` et `AI_CENTER_RUNTIME_ENV_FILE` (chemin absolu d’un fichier
privé hors dépôt). Il expose seulement `127.0.0.1:8080` par défaut. Ne jamais
publier le port PostgreSQL ou celui de l’API sur Internet.

Le fichier runtime privé contient au minimum `DATABASE_URL`, `SUPABASE_URL`,
`AI_CENTER_CORS_ORIGINS` (origine HTTPS exacte),
`AI_CENTER_CREDENTIAL_ENCRYPTION_KEY` et le choix explicite
`AI_CENTER_AGENT_MODE`. `AI_CENTER_AUTH_MODE=supabase` est imposé par Compose.
Pour un moteur serveur OpenAI, ajouter `OPENAI_MODEL` et `OPENAI_API_KEY`.
Les connexions personnelles passent par l’écran de réglages. Les abonnements
CLI locaux ne constituent pas une identité adaptée au serveur partagé.

Avec Podman, construire au format Docker (`podman build --format docker`) pour
conserver les HEALTHCHECK utilisés par Compose ; le format OCI de Podman les
ignore. Aucun conteneur de cet exemple n’achète ou ne provisionne d’hébergement.

## Ordre d’ouverture

1. Identifier la cible, son domaine, les responsables, la région et le périmètre
   de données autorisé ; enregistrer les références hors secrets.
2. Sauvegarder la base et la clé de chiffrement séparément ; restaurer la
   sauvegarde dans une infrastructure de reprise avant toute ouverture.
3. Appliquer les migrations versionnées avec le rôle de migration. Ne jamais
   transposer `db reset` de la stack de test à cette cible.
4. Vérifier/appliquer les privilèges runtime avec
   [le contrat dédié](postgres-runtime-role.md). La réparation des droits rejoue
   aussi `99_runtime_extensions.sql` après le socle.
5. Démarrer le serveur et le frontend issus du même candidat. Le serveur refuse
   le démarrage si le contrat minimal Company Context manque dans la base.
6. Configurer SMTP, expéditeur, callbacks et limites Auth. Créer la première
   identité propriétaire par l’administration Auth autorisée, ajouter son UUID
   à `AI_CENTER_COMPANY_CREATORS` côté serveur, puis utiliser le parcours de
   création d’entreprise. Un compte simplement inscrit auprès d’Auth ne peut
   pas créer une société et consommer le moteur serveur. Les propriétaires
   déjà acceptés peuvent créer d’autres sociétés, dans la limite de dix par compte. Les membres suivants utilisent les
   invitations à usage unique de l’écran Entreprise.
7. Qualifier connexion, création société/projet, invitation, retrait d’accès,
   conversation, validation d’artefact, graphe et export avec des comptes dédiés.
8. Relier les outils sur des destinations de qualification. Vérifier une
   publication, sa lecture, une erreur de droits et une réponse ambiguë.
9. Activer le modèle et un budget explicites, vérifier les coûts connus et les
   coûts inconnus ; effectuer le workflow métier avec les vraies clés.
10. Ouvrir aux personnes prévues puis observer le pilote défini par CC-T15.

## Invitations et connexions

Les liens d’invitation expirent en 1 à 7 jours. Le secret reste dans le fragment
`#token=…` du navigateur et n’est pas inclus dans le callback du magic link.
L’utilisateur garde l’onglet d’invitation ouvert pendant la connexion, puis
accepte explicitement. Le serveur ne stocke que l’empreinte du jeton. Retirer
un membre et révoquer une invitation non utilisée sont deux opérations distinctes.

Un propriétaire configure les connexions Notion, Linear et GitHub. Choisir les
permissions minimales pour les destinations voulues. Une destination enregistrée
ne prouve pas que le compte connecté y a accès. La publication est une action
explicite sur une version validée, avec son identité de traitement conservée.
Une réponse perdue peut signifier que l’objet externe a été créé : l’état
« à vérifier » impose une réconciliation et ne relance pas aveuglément la création.

## Limites et suspension

Le panneau Entreprise affiche les opérations de génération de la dernière heure,
les traitements réservés et les estimations de coût disponibles. Le coût absent
reste inconnu. Par défaut, une société peut demander 120 opérations par heure,
avec quatre appels simultanés et une durée maximale de 120 secondes chacun.
Chaque membre dispose aussi d’un plafond de 60 opérations par heure et deux
appels simultanés dans cette société ; les deux niveaux s’appliquent ensemble.
L’opérateur ajuste `AI_CENTER_AI_CALLS_PER_HOUR` (1–10000),
`AI_CENTER_AI_CONCURRENT_CALLS` (1–32) et
`AI_CENTER_AI_CALL_TIMEOUT_SECONDS` (10–600). Les tentatives internes bornées du
fournisseur ne sont pas des opérations utilisateur supplémentaires ; ce quota
n’est pas une limite monétaire. Définir aussi le budget du compte fournisseur.
Les plafonds individuels sont réglables avec `AI_CENTER_AI_ACTOR_CALLS_PER_HOUR`
(1–10000) et `AI_CENTER_AI_ACTOR_CONCURRENT_CALLS` (1–32).

Un propriétaire peut suspendre les générations et les publications. La reprise
conserve les compteurs et n’annule pas une écriture déjà acceptée chez un tiers.
Les contrôles sont persistants et couvrent aussi les connexions IA personnelles.

## Disponibilité, incidents et reprise

- `/healthz` contrôle uniquement nginx ; `/api/health` contrôle l’API et la base.
  Surveiller les deux depuis la cible HTTPS avec une alerte vers un responsable.
- Surveiller taux de 5xx/429, latence, traitements en attente, traitements à
  vérifier, refus Auth et échecs de sauvegarde. Les seuils et destinataires
  restent propres à la cible et doivent être testés réellement.
- Les commandes portent un identifiant stable. Après une réponse incertaine,
  utiliser la reprise fournie par l’interface ; ne pas créer un nouvel identifiant
  dans le seul but de contourner un conflit.
- Un arrêt du serveur interrompt les appels locaux. Il ne peut pas annuler une
  écriture déjà acceptée par un outil externe ni effacer une consommation IA.
- Un retour arrière utilise une image connue compatible avec le schéma actuel.
  Ces migrations additives ne justifient pas une restauration destructive de la
  base. Si une restauration est nécessaire, isoler le trafic et qualifier la
  reprise dans un environnement séparé avec les secrets correspondants.

## Sauvegarde, données et rétention

Le [smoke de restauration](postgres-backup-restore.md) vérifie données, ACL,
RLS, fonctions et séquences dans une base jetable. En production, définir
fréquence, chiffrement, accès, rétention, objectifs de perte acceptable et délai
de reprise avec le responsable de l’entreprise. Le journal de test ne fournit
pas de sauvegarde de production.

Un export métier contient des conversations et des données personnelles : le
conserver dans un emplacement privé, même lorsque les secrets techniques en sont
exclus. Archiver un projet conserve ses preuves et son historique. Ce n’est
pas une suppression. Une demande d’effacement doit traiter les sources croisées,
les historiques et les sauvegardes selon la procédure de données ; elle ne doit
pas être simulée par un bouton qui masque seulement le projet.

La perte de la clé de chiffrement rend les connexions sauvegardées illisibles.
Une rotation exige réencryption contrôlée ou reconnexion des outils ; remplacer
simplement la variable d’environnement n’est pas une rotation fonctionnelle.

## Dossier de qualification externe restant

Conserver pour le candidat exact : version des images et migrations, cible HTTPS,
preuve de restauration séparée, tests SMTP/Auth, identités pseudonymisées des
participants, sources et destinations réellement autorisées, budget IA mesuré,
alerte effectivement reçue, procédure de suspension et observations de pilote.
Aucune de ces preuves ne doit contenir les valeurs des secrets.


### Transport PostgreSQL

Le binaire inclut Rustls pour PostgreSQL. Une base distante doit utiliser
`sslmode=verify-full` dans `DATABASE_URL`, avec certificat et nom DNS vérifiés.
Si le fournisseur demande sa propre autorité, monter son certificat CA en
lecture seule et préciser `sslrootcert` dans la configuration privée. Les
connexions loopback des stacks locales peuvent rester sans TLS. Une erreur TLS
n'est jamais contournée en passant une base distante à `require` ou `disable`.
