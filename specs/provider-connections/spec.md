# Connexions IA personnelles — spécification

## Demande et priorité

Le 7 septembre 2026, le propriétaire demande de configurer plusieurs clés de
fournisseur dans l'application (OpenAI, Anthropic, Kimi, DeepSeek et autres), et
d'utiliser des connexions par abonnement lorsque le fournisseur le permet.
Il demande des tests unitaires et d'intégration ; il réalisera lui-même les
E2E. Cette consigne remplace, pour cette livraison, les étapes E2E/navigateur/
smoke natif exécutées par l'agent du plan Alpha Context Proof. Les scénarios
E2E existants restent disponibles pour sa recette. Aucun résultat manuel n'est
présumé réussi.

L'abonnement est interprété comme un abonnement fournisseur existant, sous
réserve de la précision demandée au propriétaire. Aucun système de facturation
AI Center n'est introduit par cette interprétation.

## Exigences

- PC-001 : page de réglages IA accessible depuis la navigation, en français.
- PC-002 : plusieurs connexions nommées, y compris plusieurs clés pour le même fournisseur, avec modèle configurable.
- PC-003 : API OpenAI, Anthropic, Kimi/Moonshot, DeepSeek et OpenRouter ; endpoints officiels fixes, sans URL arbitraire envoyant une clé vers un autre hôte.
- PC-004 : clés transmises une seule fois au serveur puis chiffrées par AEAD, avec contexte lié à l'acteur, au workspace, à la connexion et au fournisseur. Aucune clé en clair en DB, logs, réponse, stockage navigateur, tests versionnés ou exports.
- PC-005 : catalogue, liste, création, remplacement, suppression, vérification de connexion et choix du moteur réellement utilisés par les opérations du produit.
- PC-006 : connexions privées à leur propriétaire dans le workspace courant. Les autres membres ne lisent ni métadonnées privées ni ciphertext et ne peuvent sélectionner/utiliser ces connexions.
- PC-007 : sélection persistante ; fournisseur et modèle fixés pour la durée d'une opération et enregistrés dans model_runs. Une erreur de la connexion sélectionnée ne provoque pas de fallback silencieux ni l'usage d'une autre clé.
- PC-008 : mêmes opérations métier structurées pour tous les transports : extraction, sélection, plan, couverture et Steward. Vérification des schémas et invariants existants conservée.
- PC-009 : le Steward résout la connexion du bon acteur/workspace, jamais celle d'un autre demandeur ni un secret global de substitution.
- PC-010 : requêtes bornées, redirections interdites, erreurs expurgées, réponses incomplètes/refus invalides rejetés, usage et identité du modèle conservés.
- PC-011 : test de connexion explicite par lecture du catalogue fournisseur ; distinguer l'accès au catalogue de la preuve de qualité d'une génération.
- PC-012 : abonnement via client/protocole officiel seulement. Ne pas récupérer/coller des cookies ou tokens d'une autre application ni inventer de login universel. Exposer honnêtement les capacités et prérequis du client local.
- PC-013 : aucune capacité de runner de code introduite. Pour les transports locaux par abonnement, désactiver outils, hooks, MCP et découverte du projet ; refuser un mode où ces frontières ne peuvent pas être garanties.
- PC-014 : stockage des secrets indisponible ou client officiel absent => état de configuration explicite et stable ; aucun secret serveur requis dans VITE_*.
- PC-015 : formulaires accessibles, erreurs/réessais et effacement des clés saisies après sauvegarde/sortie/changement d'identité ; aucune fuite par cache ou réponse tardive.
- PC-016 : tests unitaires, contrats HTTP/processus simulés et intégration PostgreSQL/RLS réels. Aucun E2E, navigateur automatisé, fournisseur payant ou login fournisseur réel exécuté par l'agent.

## Contrat HTTP à partager

Les routes utilisent le contexte d'authentification/workspace existant et les
UUID Idempotency-Key pour les mutations. Les opérations privées de connexion
peuvent être réservées aux owner/editor, conformément au garde existant ; un
viewer ne gagne aucun droit métier grâce à une clé personnelle.

- `GET /api/ai/settings` → `{ providers, connections, selection, storage }`.
- `POST /api/ai/connections` → connexion masquée ; corps `{ id, provider, name, model, api_key? }` ; `id` est un UUID client.
- `POST /api/ai/connections/{id}/update` → connexion masquée ; corps `{ name, model, api_key? }` ; clé absente = conserver la clé.
- `POST /api/ai/connections/{id}/delete` → `{ deleted: true }` ; suppression atomique de la sélection liée, retour explicite au mode déterministe.
- `POST /api/ai/connections/{id}/test` → `{ ok: true, models: [{ id, name }] }` ; pas de prompt métier transmis.
- `POST /api/ai/selection` → sélection ; corps `{ mode: "server_default" | "deterministic" | "connection", connection_id: UUID | null }`.

Connexion masquée : `{ id, provider, name, model, key_hint, created_at, updated_at }`.
Provider : `{ id, name, auth_method: "api_key" | "subscription", available,
unavailable_reason, help_url }`. Identifiants API : `openai`, `anthropic`, `kimi`,
`deepseek`, `openrouter`. Les abonnements ont leurs identifiants séparés.
Storage : `{ available, message }`, sans chemin secret ni empreinte de clé.
Les routes complémentaires de login/status/logout abonnement seront définies
après vérification du protocole officiel et documentées dans le même répertoire.

## Interface interne des transports

Module `integrations::providers` :

- `ProviderModel { id: String, name: String }` sérialisable.
- `StructuredResponse { output: serde_json::Value, metadata: AgentRunMetadata }`.
- trait asynchrone `StructuredTransport: Send + Sync` : `provider_name() -> &'static str`, `generate(model, operation_name, instructions, input, schema) -> AppResult<StructuredResponse>`, `list_models() -> AppResult<Vec<ProviderModel>>`.
- `api_transport(provider: &str, key: SecretString) -> AppResult<Arc<dyn StructuredTransport>>` valide le fournisseur et construit un client officiel borné.
- `agent::StructuredEngine::with_transport(Arc<dyn StructuredTransport>, model: String)` réutilise les prompts/schémas métier. Conserver les constructeurs OpenAiEngine existants pour compatibilité des tests et configuration serveur.

La gestion des connexions compose ces transports. Les clients d'abonnement
implémentent le même trait, avec faux processus uniquement pendant nos tests.

## Preuves et limites

La réussite logicielle sera documentée séparément de la recette E2E du
propriétaire et des essais avec des comptes/crédits réels. Aucun abonnement
n'est présenté comme finançant automatiquement une clé API. Le plan d'origine
reste la référence produit pour le contexte, la provenance, les preuves et la
couverture.

## Corrections de revue PC-T05

- Une lecture de corps HTTP interrompue ou expirée conserve sa classe transitoire
  et la limite de trois tentatives ; un JSON complet invalide reste permanent.
- L'échec local d'une connexion choisie conserve le message validé et son run en
  échec avec l'identité choisie. Le replay reste idempotent, sans repli implicite.
- L'état d'authentification d'un abonnement est distinct de son admissibilité ;
  un compte API ou Team reste récupérable par déconnexion/changement de compte.
- Le catalogue des fournisseurs API et leurs protocoles fixes ont une liste
  commune, limitée aux cinq fournisseurs actuellement pris en charge.

Preuves attendues : faux HTTP, faux processus, composants et PostgreSQL dédiés.
La recette manuelle et les comptes réels restent à la charge du propriétaire.
