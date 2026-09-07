# Contrat des abonnements locaux

Contrat PC-T03 du 7 septembre 2026. Les E2E et connexions réelles sont réalisés
par le propriétaire ; nos tests emploient uniquement de faux processus.

## Routes et DTO

Toutes les routes appliquent les mêmes contrôles de propriétaire et workspace
que les connexions privées. Le scope interne contient les UUID publics
`workspace_id`, `actor_id`, `connection_id`. Un `login_id` ne donne aucun droit
supplémentaire. Les mutations utilisent le contrat Idempotency-Key commun.

- `GET /api/ai/connections/{id}/subscription/status` → état.
- `POST /api/ai/connections/{id}/subscription/login` → tentative.
- `GET /api/ai/connections/{id}/subscription/login/{login_id}` → tentative.
- `POST /api/ai/connections/{id}/subscription/login/{login_id}/cancel` → tentative annulée.
- `POST /api/ai/connections/{id}/subscription/logout` → état déconnecté après vérification.

État : `{ status, authenticated, connected, available, message }`, avec `status` dans
`connected | disconnected | unavailable | unknown` et `message` nullable.
Il ne contient aucune adresse e-mail, organisation, identifiant de compte
fournisseur, chemin privé ou sortie brute du client.

`authenticated` rapporte le champ officiel `loggedIn` : booléen si l'état a été
vérifié, `null` si le client est indisponible et ne peut pas être inspecté.
`connected` signifie que le compte authentifié est admissible pour la génération.
Un compte API ou Team peut donc être authentifié tout en ayant `connected: false`
et `available: false`. Il reste possible de le déconnecter ou de changer de compte
sans supprimer le profil. Le serveur vérifie `authenticated: false` après le
logout officiel ; une réponse encore authentifiée est une erreur. Le motif
d'inadmissibilité reste visible après une tentative de login non admissible.

Tentative : `{ login_id, status, auth_url }`, avec `status` dans
`pending | connected | failed | cancelled | expired` et `auth_url` nullable.
L’URL n’est disponible que pendant la connexion. L’UI peut interroger la
tentative en cours, doit proposer l’annulation et arrêter son polling à un
état terminal. Une réponse tardive d’une autre identité est ignorée. L’URL
n’est pas mise en cache, journalisée ni persistée dans une table de replay.
Le replay d’une mutation peut conserver uniquement l’identifiant et l’état,
puis consulter la tentative privée pour obtenir l’URL actuelle.

## Capacités annoncées

`claude_subscription` utilise le binaire officiel Claude Code. Le serveur local
doit activer cette capacité et connaître son chemin absolu. Le runtime vérifie
sa présence, sa version et ses options de sécurité sans lire le compte. Une
absence, un format inconnu ou une frontière non prouvée retourne une raison
explicite ; aucun client n’est installé automatiquement.

Le contrat épingle la version inspectée Claude Code 2.1.220. Une autre version
reste indisponible jusqu’à vérification de son contrat sans outils d’action. Linux est la seule
plateforme dont les chemins de politiques et groupes de processus sont
actuellement couverts par ce module. Les politiques administrées détectées
empêchent l’activation ; les sessions Team/Enterprise et types inconnus ne sont
pas utilisés pour générer, car elles peuvent charger une politique distante
prioritaire. Elles ne sont jamais contournées.

`codex_subscription` reste indisponible : l’authentification officielle existe,
mais la désactivation de tous les outils d’action n’est pas prouvée pour le
client inspecté. L’UI ne propose pas de bouton de connexion fonctionnel pour
ce transport.

## Connexion et confidentialité

Le runtime lance `claude --safe-mode auth login`. Il ne réimplémente pas OAuth
et n’impose pas une méthode de connexion au binaire. La sortie peut fournir
une URL HTTPS officielle sur `claude.com/cai/oauth/authorize`,
`claude.ai/oauth/authorize`, `platform.claude.com/oauth/authorize` ou le domaine
historique `console.anthropic.com/oauth/authorize`. Toute autre destination,
fragment, identifiant d’URL ou token est refusé. Le paramètre `code=true` du
flux officiel n’est pas un code OAuth et est accepté.

Le navigateur est ouvert par l’utilisateur depuis ce lien. Le callback est
géré par Claude Code. Si l’utilisateur doit coller un code de secours, il
termine cette étape dans le CLI officiel : AI Center ne propose aucun champ
pour un code, cookie ou token de session. Login : maximum 180 secondes.
Statut et logout : commandes officielles bornées à 10 secondes.

Le runtime ne lit jamais les fichiers de credentials. Le client officiel les
gère sous `CLAUDE_CONFIG_DIR`, distinct pour les trois UUID du scope, avec
répertoires 0700. `HOME` et XDG sont isolés uniquement dans l’enfant. Aucun
changement de l’environnement global ni réemploi d’une session d’une autre
application. L’environnement hérité est supprimé : aucune clé API, token,
endpoint de substitution ou configuration cloud ne sélectionne une autre
facturation à l’insu de l’utilisateur.

## Génération et validation

Le transport commun `StructuredTransport` transmet instructions et contexte
uniquement sur stdin. Les arguments contiennent uniquement le modèle, les
options fixes et le schéma métier statique sans donnée utilisateur. Claude
Code n’expose pas d’option documentée de schéma par fichier ; aucun faux flag
n’est introduit.

Options : `--safe-mode -p --tools "" --disallowedTools "mcp__*"`, configuration
MCP vide et stricte, skills et Chrome désactivés, aucune source de paramètres
utilisateur/projet, `disableAllHooks: true`, `dontAsk`, aucune persistance de
conversation, `--output-format json --json-schema <schéma statique>` et prompt
système fixe. Aucun outil d’action n’est offert. Le mécanisme interne
`StructuredOutput` du client valide le JSON et termine le tour ; il ne lit
pas de fichier et n’exécute pas de commande. Il ne faut pas le supprimer avec
une interdiction globale `*` qui empêcherait le format structuré.

Les entrées et stdout sont limités à 2 Mio ; les sorties d’authentification et
stderr à 64 Kio ; une génération dure au maximum 120 secondes. Quatre processus
maximum et une opération par connexion bornent la concurrence. Une future
annulée tue le groupe du processus, ses descendants et libère sa place. Logout
annule une tentative en cours et la génération avant de déconnecter. La
suppression d’une connexion retire son répertoire UUID après annulation,
même sans client installé ; un UUID supprimé ne peut pas être réactivé par
une ancienne opération dans le runtime. Les
erreurs publiques sont classifiées sans leur texte privé.

Seule une enveloppe `result/success` valide contenant `structured_output` est
acceptée. Les refus, troncatures, outils inattendus, erreurs et changement
silencieux de modèle sont rejetés. Le schéma et les invariants métier sont
également validés par `StructuredEngine`. Le modèle servi et l’usage sont
conservés ; une estimation CLI de coût n’est pas présentée comme une facture
de l’abonnement.

`list_models` vérifie le statut et retourne les alias officiels du CLI. Cette
liste est un aide à la configuration, pas une interrogation d’un catalogue
de compte ni une preuve d’éligibilité ou de qualité de génération. Le modèle
peut être saisi manuellement.

## Sources officielles

- [Claude Code dans un produit, méthodes d’authentification](https://code.claude.com/docs/en/legal-and-compliance).
- [CLI : flags, statut et déconnexion](https://code.claude.com/docs/en/cli-reference).
- [Exécution programmatique et JSON structuré](https://code.claude.com/docs/en/headless).
- [Stockage des sessions et priorité de facturation](https://code.claude.com/docs/en/authentication).
- [Politiques administrées et comptes d’organisation](https://code.claude.com/docs/en/admin-setup).
- [Support : changement de crédits SDK suspendu](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan).
- [Codex app-server](https://learn.chatgpt.com/docs/app-server).

Ces sources décrivent le protocole ; aucune connexion réelle ni génération
fournisseur n’a été exécutée pendant notre implémentation.

## Preuve de validation PC-T03

18 tests unitaires et d’intégration de processus passent avec une CLI Python
factice : isolation des scopes et de l’environnement, stdin privé, version
future refusée, URLs officielles, comptes personnels, enveloppes invalides,
limites de sortie, échéances, annulation des descendants, logout et suppression
sans client. Les régressions PC-T05 couvrent aussi le login API/Team, sa
récupération vers un abonnement personnel, et un logout qui laisse le compte
authentifié. `cargo clippy -p ai-center-server --all-targets --offline -- -D warnings`
passe. Le module a été compilé contre le contrat PC-T01 et les dépendances
`tokio/process/io-util/fs` et `nix/signal/process`, fournies à l’intégration par
PC-T02. Aucune preuve fournisseur réel ou E2E n’est revendiquée.
