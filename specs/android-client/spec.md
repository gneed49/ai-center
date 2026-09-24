# Spécification — Client Android AI Center

> Statut : active  
> Responsable : AI Center  
> Dernière mise à jour : 2026-08-18

## Intention

Distribuer le même plan de contrôle contextuel sur Android à partir du frontend
React et du cœur Tauri v2 existants. L’application Android reste un client du
serveur Rust déployable : elle n’embarque ni PostgreSQL, ni clé OpenAI, ni règle
métier divergente.

## Contexte et décisions

- `docs/product/04-mvp-scope.md` définit les parcours P0 à préserver.
- `docs/decisions/0001-cross-platform-control-plane.md` impose un frontend
  partagé et une enveloppe native légère.
- `docs/decisions/0003-server-side-agent-engine.md` garde les secrets et le
  moteur agentique sur le serveur.
- `docs/decisions/0004-android-client.md` précise l’adressage réseau mobile.

## Périmètre

### Inclus

- Projet Android généré par Tauri v2 et Gradle.
- Identifiant Android stable `com.aicenter.client`.
- Compatibilité Android API 24 ou supérieure.
- APK debug signé par ABI pour installation directe, APK release universel et
  AAB pour une future publication.
- ABI de distribution `arm64-v8a` pour les appareils et `x86_64` pour les
  émulateurs ; les cibles Rust 32 bits restent disponibles hors build standard.
- Navigation et dix surfaces P0 utilisables sur écran de téléphone.
- Respect des zones système et du viewport tactile.
- URL d’API configurable au build par `VITE_API_URL`.
- Valeur par défaut Android `http://10.0.2.2:4317` pour l’émulateur officiel.
- Autorisation réseau Android et trafic HTTP local explicite pour le développement.

### Exclu

- Publication Google Play, fiche Store et signature de production définitive.
- Notifications push, biométrie, partage natif et fonctionnement hors ligne.
- Serveur Rust ou Supabase embarqué sur le téléphone.
- Synchronisation en arrière-plan.

## Exigences de sécurité

- Aucun secret serveur dans l’APK ou le bundle web.
- L’URL de production doit utiliser HTTPS ; HTTP reste autorisé pour le serveur
  local ou l’émulateur de développement.
- La CSP n’autorise que les adresses locales explicitement déclarées. L’origine
  HTTPS de production sera ajoutée lorsqu’elle sera connue.

## Critères d’acceptation

- [x] AND-01 — `tauri android init` génère un projet Android reproductible.
- [x] AND-02 — Le package Android est `com.aicenter.client` et cible API 24+.
- [x] AND-03 — Des APK debug signés et installables sont générés pour arm64 et x86_64.
- [x] AND-04 — Un AAB de distribution est généré.
- [ ] AND-05 — Le frontend mobile démarre dans la WebView sans erreur critique.
- [x] AND-06 — L’URL API peut être définie au build sans modifier le code.
- [x] AND-07 — L’émulateur utilise par défaut `10.0.2.2` et le web/desktop conserve `127.0.0.1`.
- [x] AND-08 — Les routes principales restent utilisables à 390 × 844.
- [ ] AND-09 — Les zones système Android ne recouvrent pas les commandes.
- [x] AND-10 — Le serveur accepte l’origine Tauri Android sans ouvrir un CORS global.
- [x] AND-11 — Aucun secret n’est présent dans l’APK, l’AAB ou les sources générées.
- [x] AND-12 — Les tests, lints et builds web/desktop existants restent verts.

Les critères AND-05 et AND-09 nécessitent encore une exécution sur une cible
Android. L’émulateur Linux disponible quitte avec le code 139 avant le boot du
système, y compris avec deux versions du moteur et plusieurs modes graphiques.
Les preuves statiques et responsive sont détaillées dans `validation.md`.

## Preuves attendues

- Tests unitaires du résolveur d’URL par plateforme.
- Test responsive des réglages et des parcours principaux.
- Sortie Gradle/Tauri, contrôle `aapt` du manifeste et vérification de signature.
- Inventaire des APK/AAB avec tailles et empreintes SHA-256.
- Audit de secrets sur les artefacts décompressés.
