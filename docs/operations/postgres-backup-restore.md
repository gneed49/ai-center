# Smoke de sauvegarde et restauration PostgreSQL

Le gate PostgreSQL desktop vérifie qu'une sauvegarde du schéma applicatif
`app` et de ses données peut être restaurée sans perte. Ce contrôle vise
uniquement la stack Supabase locale isolée ; il ne constitue pas encore la
politique de sauvegarde de l'alpha hébergée.

## Contrat de sûreté

`scripts/postgres-backup-restore-smoke.sh` refuse de démarrer sauf si
`AI_CENTER_ADMIN_DATABASE_URL` désigne exactement la base `postgres` sur
`127.0.0.1` ou `localhost`, au port local explicitement autorisé par
`AI_CENTER_LOCAL_POSTGRES_PORT` (`54322` par défaut). `localhost` est réécrit
en `127.0.0.1` avant connexion. Les query strings, ports différents du port
autorisé, bases différentes et hôtes distants sont refusés.

La restauration utilise une base unique nommée
`ai_center_restore_<10 caractères alphanumériques>`. Le nom est validé avant
`createdb` et de nouveau dans le trap de nettoyage. Le trap appelle `dropdb
--force --if-exists` sur ce nom exact puis ne supprime que les fichiers du
répertoire créé par `mktemp`; aucun glob ni chemin récursif n'est utilisé.
L'URL et les données ne sont jamais imprimées.

## Preuve effectuée

Le script :

1. crée une archive custom avec `pg_dump --schema=app` ;
2. restaure l'archive avec `pg_restore --exit-on-error` dans une base créée
   depuis `template0` ;
3. compare le nombre de lignes de chaque table ;
4. compare un inventaire trié des relations, colonnes, contraintes, index,
   fonctions, triggers, ACL, policies et drapeaux RLS ;
5. exige que chaque table `app` ait RLS activée et forcée et qu'au moins une
   policy existe ;
6. compare un dump de schéma normalisé ;
7. compare un hash SHA-256 d'une projection logique déterministe des lignes
   JSONB triées et de l'état des séquences ;
8. supprime la base temporaire, y compris après erreur ou interruption.

Le hash affiché est une empreinte locale de validation, pas une sauvegarde ni
un artefact à publier. Les fichiers temporaires sont supprimés à la fin du run.

## Exécution locale

La stack doit déjà avoir été reconstruite et le rôle runtime appliqué. Les
clients PostgreSQL (`psql`, `pg_dump`, `pg_restore`, `createdb`, `dropdb`) sont
requis.

```bash
export AI_CENTER_ADMIN_DATABASE_URL='postgresql://postgres:postgres@127.0.0.1:54322/postgres'
./scripts/ci-desktop.sh backup-restore
```

Une stack jetable sur un autre port loopback peut être certifiée sans relâcher
la protection : définir le même port dans l'URL et dans
`AI_CENTER_LOCAL_POSTGRES_PORT`.

La CI appelle cette commande dans le job PostgreSQL juste après `integration`,
donc après `supabase db reset`, les tests de base et le provisionnement de
`ai_center_runtime`. Elle ne touche ni Android, ni iOS, ni une base distante.

## Limites avant alpha

Ce smoke prouve la restaurabilité logique du domaine `app` dans le même cluster
local, avec les rôles Supabase déjà présents. Avant promotion, la sauvegarde de
l'environnement privé doit aussi être restaurée dans une infrastructure de
reprise séparée et son objectif de temps de reprise doit être mesuré.
