# Validation — Flow de développement unifié

> Date : 2026-08-21
>
> Résultat : implémenté ; smoke test Supabase bloqué par le daemon Docker arrêté

## Résultats

| Contrôle | Résultat |
| --- | --- |
| Syntaxe Bash des quatre scripts | Réussi |
| `./dev help` | Réussi |
| `./dev doctor --json` | Réussi, signale correctement Docker arrêté |
| `./dev status --json` | Réussi |
| CLI Supabase et Tauri locales | Détectées |
| Lint TypeScript et Rust | Réussi sans avertissement |
| Tests frontend | 5/5 réussis |
| Tests unitaires Rust | 3/3 réussis |
| Build web et workspace Rust | Réussi |
| Format Rust et contrôle des diffs | Réussi |
| Test d'intégration PostgreSQL | Bloqué : Docker local inactif |
| Démarrage Tauri par l'orchestrateur | En attente du démarrage Docker/Supabase |

## Blocage environnemental

Docker 29.7.2 est installé, mais `docker.service` est désactivé et inactif. La
politique de la machine exige un mot de passe administrateur pour le démarrer ;
aucun secret n'a été demandé ou manipulé pendant la validation.

Le lanceur tente d'abord un démarrage non interactif puis retourne une erreur
actionnable :

```text
sudo systemctl start docker
```

Une fois Docker actif, exécuter les preuves runtime finales :

```bash
./dev
./dev status --json
npm test
./dev stop
```

`./dev stop` utilise `supabase stop` sans `--no-backup`, conformément au contrat
de conservation des données locales.
