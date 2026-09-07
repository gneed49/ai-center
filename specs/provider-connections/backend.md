# Connexions personnelles — serveur et preuves PC-T02

## Configuration

Les clés API des utilisateurs sont saisies dans la page **Connexions IA**. La
configuration `OPENAI_API_KEY` existante reste réservée au moteur serveur par
défaut ; elle n’est jamais copiée dans une connexion personnelle.

En mode `AI_CENTER_AUTH_MODE=local`, avec une écoute loopback, le serveur génère
une clé maîtresse aléatoire de 32 octets dans
`.run/provider-credentials/credentials.key`. Le répertoire doit être privé
(`0700`), le fichier doit être `0600`, régulier, sans lien symbolique ou physique.
Les répertoires ancêtres ne doivent pas être des liens symboliques. Un fichier
existant avec des permissions incorrectes est refusé, jamais réécrit.
La création locale automatique est actuellement prise en charge sur Unix.

Pour un serveur partagé (`AI_CENTER_AUTH_MODE=supabase`), fournir
`AI_CENTER_CREDENTIAL_ENCRYPTION_KEY` exclusivement au serveur via son gestionnaire
de secrets. Format : exactement 64 caractères hexadécimaux représentant 32 octets
aléatoires. Une valeur absente ou incorrecte laisse l’application accessible avec
un état de configuration explicite, mais interdit l’utilisation des connexions
à clé. Une valeur configurée incorrecte ne déclenche pas de génération locale
alternative. Aucune variable `VITE_*` ne contient un secret.

La clé maîtresse doit être sauvegardée séparément de la base chiffrée. La changer
sans rechiffrer les connexions rend leurs clés inexploitables ; aucun fallback
vers une autre connexion n’est effectué.

En local, le client officiel Claude installé est recherché dans les répertoires
absolus de `PATH` (hors répertoire du projet). Le chemin est résolu avant usage et
le runtime vérifie version, capacités et isolation avant de rendre le fournisseur
disponible. Une absence de client est affichée dans les réglages. Les options
serveur, commentées dans `.env.example`, sont :

- `AI_CENTER_SUBSCRIPTIONS_ENABLED=false` : désactiver ce transport local ;
- `AI_CENTER_CLAUDE_EXECUTABLE` : chemin absolu explicite vers le client officiel ;
- `AI_CENTER_SUBSCRIPTIONS_DIRECTORY` : répertoire absolu privé, par défaut
  `.run/provider-subscriptions`.

Le transport d’abonnement reste désactivé sur un serveur partagé ; un runtime
désactivé peut uniquement effacer les anciens secrets lors d’une suppression. La simple
construction du serveur ne lance pas le client. Voir le contrat d’abonnement
[abonnement](subscriptions.md) pour les limites fournisseur et les sessions isolées.

## Sécurité et identité

`app.provider_connections` et `app.provider_selections` sont dans le schéma privé
`app`, hors Data API Supabase. Le runtime est `NOBYPASSRLS`, les tables sont
`FORCE ROW LEVEL SECURITY`. Les politiques vérifient ensemble le workspace,
l’acteur et une adhésion acceptée owner/editor. Un viewer est refusé, y compris
pour lire les réglages. Les rôles `anon`, `authenticated` et `service_role` n’ont
aucun droit direct sur ces tables. Les droits destructifs du runtime restent
interdits sur les objets métier ; DELETE est limité aux réglages personnels.

AES-256-GCM lie chaque enveloppe à la version de format, au workspace public,
à l’acteur, à l’UUID de connexion et au fournisseur. Le nonce aléatoire fait
12 octets. Les réponses n’exposent que les quatre derniers caractères de la clé.
Le DTO ne sérialise jamais la clé. L’idempotence utilise un engagement HMAC à sens
unique dérivé de la clé maîtresse, puis le hash de requête existant. Les réponses
enregistrées ne contiennent ni clé, ni ciphertext, ni URL temporaire de login.

La sélection et la connexion sont lues sous un verrou court par acteur/workspace.
L’objet moteur est fixé pour l’opération avant création du `model_run` : extraction,
sélection du contexte, plan, couverture et Steward. Plan et couverture gardent le
même moteur. Le `selection_mode` du ContextPack reflète ce moteur effectif. Aucune
transaction SQL ne reste ouverte pendant une génération IA.

Chaque événement métier mémorise `requested_by_actor_id`, issu du contexte
serveur. Le worker peut découvrir un workspace sous un autre membre autorisé,
mais il réauthentifie l’acteur originel avant de charger son choix personnel.
La coalescence des événements utilise projet **et acteur**. Un acteur révoqué,
viewer ou absent ne peut pas être remplacé par le scanner. Les anciens événements
sans acteur explicite échouent avec une demande de nouvelle mutation de contexte.

## Migration et tests

Migration générée par Supabase CLI **2.114.0**, moteur **pg-delta**, depuis les
schémas déclaratifs :
`supabase/migrations/20260907155244_personal_provider_connections.sql`.
Elle ajoute les deux tables, leurs contraintes/politiques/droits et l’acteur des
événements. Aucune table ou donnée existante n’est supprimée. Le schéma et le
serveur doivent être livrés ensemble avant l’utilisation des connexions.

Validation au 7 septembre 2026 :

- 3 tests unitaires de chiffrement/permissions de fichier/non-sérialisation : PASS.
- Contrat HTTP et PostgreSQL réel sous `ai_center_runtime` : PASS. Il couvre
  plusieurs clés OpenAI, réponses masquées, idempotence/rejeu/changement de clé,
  viewer 403, séparation acteur/workspace et protection RLS directe, conservation
  et remplacement de clé, moteur figé, corruption sans fallback, suppression
  atomique et utilisation réelle du moteur sélectionné.
- Le même test couvre un scanner avec une clé inutilisable traitant correctement
  l’événement d’un autre acteur, puis refuse le remplacement inverse d’un acteur
  à clé inutilisable par le moteur du propriétaire.
- Posture exacte `ai_center_runtime` : PASS ; **32 pgTAP** : PASS.
- **11 tests d’intégration PostgreSQL** dans six binaires Rust : PASS (concurrence
  du contexte, idempotence, cycle de vie modèle, parcours métier API, connexions
  personnelles et invalidation ciblée). **1 test supplémentaire** de persistance
  GitHub simulée sur cette base : PASS.
- Réponses HTTP privées et erreurs : `Cache-Control: no-store` vérifié.
- `cargo clippy -p ai-center-server --all-targets --offline -- -D warnings` : PASS.
  `cargo fmt --all` et `git diff --check` : PASS.

La phase `integration` de `scripts/ci-desktop.sh` inclut désormais
`cargo test -p ai-center-server --test provider_connections -- --test-threads=1`.
Elle nécessite `DATABASE_URL` runtime et `AI_CENTER_ADMIN_DATABASE_URL` pour les
fixtures sur une **base jetable dédiée**. Les fixtures ne suppriment pas les audits
immuables ; le nettoyage se fait par destruction des seuls volumes de cette stack.

Aucun E2E, navigateur, login réel, prompt fournisseur facturé ou campagne corpus
n’a été exécuté pour cette livraison. La recette E2E appartient au propriétaire.
