# Développement local unifié

Le lanceur `./dev` démarre l'environnement Linux complet depuis n'importe quel
répertoire courant : Supabase/PostgreSQL, le serveur Rust et l'application
Tauri, qui démarre elle-même le frontend Vite.

## Démarrage

```bash
./dev
```

`./dev` et `./dev linux` sont équivalents. Le moteur agentique déterministe est
utilisé par défaut afin que le démarrage fonctionne sans clé ni coût API.

Pour utiliser OpenAI avec la clé conservée dans `.env.local` :

```bash
./dev linux --agent openai
```

Le premier lancement installe les dépendances npm si nécessaire et peut prendre
plusieurs minutes pendant le téléchargement des images Supabase. Les lancements
suivants réutilisent les dépendances et la base existantes.

Si Docker est installé mais arrêté, le lanceur tente `systemctl start docker`
sans interaction lorsque les droits `sudo` le permettent. Sinon, il indique la
commande exacte à exécuter manuellement.

## Commandes

| Commande | Effet |
| --- | --- |
| `./dev` | Démarre l'environnement Linux complet |
| `./dev linux --verbose` | Démarre avec les journaux Tauri détaillés |
| `./dev doctor` | Vérifie les prérequis sans rien modifier |
| `./dev doctor --json` | Même diagnostic, lisible par un agent |
| `./dev status` | Affiche l'état de la base, de l'API et de l'app |
| `./dev status --json` | Retourne un objet JSON stable |
| `./dev logs` | Suit les journaux serveur et application |
| `./dev logs server` | Suit uniquement le serveur Rust |
| `./dev stop` | Arrête l'environnement et conserve la base |

Les alias npm `debug`, `debug:linux`, `dev:doctor`, `dev:status` et `dev:stop`
exposent la même interface. Les commandes historiques restent disponibles.

## Cycle de vie

- `Ctrl+C` ou la fermeture de Tauri arrête l'application et le serveur lancés
  pendant la session ; Supabase reste actif pour accélérer la reprise.
- `./dev stop` arrête également Supabase. La CLI conserve ses volumes et donc
  les données locales.
- Aucun lancement normal n'exécute `supabase db reset` ou `supabase stop
  --no-backup`.
- `.env.local` existant n'est jamais remplacé. S'il manque, une copie privée de
  `.env.example` est créée.

Les PID, empreintes et journaux sont stockés sous `.run/ai-center/`, qui n'est
pas versionné. En cas d'échec :

```bash
./dev status
./dev logs
./dev stop
```

## Prérequis Linux

- Node.js 24+ et npm ;
- Rust 1.91+ avec Cargo ;
- Docker accessible par l'utilisateur ;
- GTK 3, WebKitGTK 4.1, `pkg-config`, `curl` et `setsid`.

`./dev doctor` identifie précisément un prérequis absent.

## Android

Cette interface ne sélectionne pas encore de téléphone. Les commandes Android
existantes (`npm run dev:android`, `build:android:debug` et `verify:android`)
restent inchangées. La connexion USB sera ajoutée dans une itération dédiée.
