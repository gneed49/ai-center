# Flow de développement unifié

> Statut : active
>
> Responsable : AI Center

## Objectif

Permettre à un humain ou à un agent de démarrer l'environnement local complet
avec une commande stable, sans écraser la configuration locale ni réinitialiser
les données.

## Périmètre

- Supabase local et PostgreSQL ;
- serveur Rust/Axum ;
- frontend Vite lancé par l'application Tauri Linux ;
- diagnostic, statut, journaux et arrêt contrôlé ;
- mode agent déterministe par défaut et mode OpenAI explicite.

La connexion à un téléphone Android est hors de cette itération. Les commandes
Android existantes restent disponibles.

## Interface

```text
./dev [linux] [--agent deterministic|openai] [--verbose]
./dev doctor [--json]
./dev status [--json]
./dev logs [server|app]
./dev stop
```

Sans sous-commande, `./dev` équivaut à `./dev linux`.

## Règles

1. Le script se repositionne à la racine du dépôt.
2. Les dépendances npm sont installées seulement lorsque le lockfile a changé.
3. `.env.local` n'est jamais remplacé. S'il manque, il est créé depuis
   `.env.example` avec le moteur déterministe sélectionné par le processus.
4. `supabase start` est réutilisable et aucune commande destructive de base de
   données n'est exécutée implicitement.
5. Le serveur n'est déclaré prêt qu'après succès de `GET /api/health`.
6. Les PID et journaux sont écrits sous `.run/ai-center/`, ignoré par Git.
7. L'arrêt ne cible que les processus dont le script a enregistré le PID.
8. Les commandes de diagnostic ne demandent aucune interaction et disposent
   d'une sortie JSON exploitable par un agent.

## Critères d'acceptation

- **DEV-01** : `./dev` et `./dev linux` lancent Supabase, l'API et Tauri Linux.
- **DEV-02** : un second lancement réutilise les services sains ou échoue avec
  une explication claire, sans créer de doublon.
- **DEV-03** : un environnement sans `.env.local` démarre en mode déterministe.
- **DEV-04** : `./dev doctor`, `status`, `logs` et `stop` sont documentés.
- **DEV-05** : `./dev stop` conserve les données Supabase.
- **DEV-06** : les commandes npm et Android existantes restent inchangées.
- **DEV-07** : lint, tests web/Rust et contrôles shell passent.

## Preuves attendues

- vérification syntaxique des scripts Bash ;
- sortie réussie de `./dev doctor --json` ;
- tests et lint existants ;
- démarrage de Supabase et réponse de `/api/health` ;
- statut Git propre après commit.
