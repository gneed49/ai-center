# 0004 — Android comme client du serveur AI Center

> Statut : accepted  
> Date : 2026-08-18

## Décision

La version Android réutilise le frontend React et la crate Tauri existants. Elle
reste un client réseau du serveur Rust/Axum et ne duplique pas le domaine, la
base Supabase ou le moteur OpenAI sur l’appareil.

L’URL du serveur est configurable au build par `VITE_API_URL`. La valeur par
défaut dépend de la plateforme : `127.0.0.1` sur web/desktop et `10.0.2.2` sur
l’émulateur Android. Une installation sur téléphone réel doit viser une URL
HTTPS déployée, ajoutée explicitement à la CSP de la configuration de production.

## Raisons

- Préserver un seul modèle métier et un seul flux de migrations.
- Ne jamais distribuer les secrets OpenAI ou PostgreSQL dans une application.
- Permettre la validation locale sur émulateur sans figer l’adresse future du
  serveur de production.
- Garder la portabilité web, Linux, Android et futures plateformes Tauri.

## Conséquences

- L’app mobile nécessite une connectivité serveur pour les fonctions métier.
- Le trafic HTTP en clair n’est conservé que pour le développement local ; les
  déploiements publics doivent utiliser HTTPS.
- La disponibilité offline et la synchronisation différée restent hors MVP.
