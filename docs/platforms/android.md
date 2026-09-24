# Client Android

Le client Android AI Center réutilise le frontend React et le cœur Tauri v2 du
desktop. Le serveur Rust/Axum, PostgreSQL et les secrets OpenAI restent côté
serveur.

## Prérequis

- Node.js et Rust selon le `README.md` ;
- JDK 17 ou 21 ;
- Android SDK Platform 36, Build Tools et Platform Tools ;
- Android NDK ;
- cibles Rust `aarch64-linux-android` et `x86_64-linux-android`.

Le script `scripts/android.sh` détecte par défaut le SDK dans
`$HOME/Android/Sdk`, le JDK dans `$HOME/Android/jdk-21` et la version la plus
récente du NDK installée. Les variables standard Android peuvent remplacer ces
valeurs.

## Initialiser et construire

```bash
npm run android:init
npm run build:android:debug
npm run build:android
npm run verify:android
```

`build:android:debug` produit des APK signés avec la clé Android de debug :

- `apk/arm64/debug/app-arm64-debug.apk` pour un téléphone Android courant ;
- `apk/x86_64/debug/app-x86_64-debug.apk` pour un émulateur x86_64.

`build:android` produit un APK release universel non signé et un AAB. Une clé de
publication dédiée est obligatoire avant toute distribution Google Play.

## Relier le serveur

L’émulateur officiel expose la machine hôte sous `10.0.2.2`. Sans configuration
supplémentaire, le frontend Android utilise donc
`http://10.0.2.2:4317`, tandis que le web et le desktop utilisent
`http://127.0.0.1:4317`.

Pour tester un téléphone connecté en USB contre le serveur local, rediriger le
port puis construire le client avec l’URL de boucle locale :

```bash
adb reverse tcp:4317 tcp:4317
VITE_API_URL=http://127.0.0.1:4317 npm run build:android:debug
adb install -r apps/desktop/src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk
```

Pour une distribution réelle :

1. déployer le serveur derrière HTTPS ;
2. ajouter précisément cette origine au `connect-src` de la CSP Tauri ;
3. construire avec `VITE_API_URL=https://api.example.com` ;
4. signer l’AAB avec une clé de publication protégée, jamais stockée dans le dépôt.

Le trafic HTTP en clair est autorisé uniquement dans le variant debug. Le
variant release le refuse.

## Contrôles avant livraison

```bash
npm run lint
npm test
npm run build
npm run build:desktop
npm run verify:android
```

Sur une cible Android, vérifier ensuite :

1. ouverture de l’application sans écran blanc ni crash ;
2. absence d’erreur WebView dans `adb logcat` ;
3. ouverture du menu et navigation vers un projet ;
4. connexion au serveur et chargement des projets ;
5. absence de recouvrement par les barres système en haut et en bas.

La matrice de preuves courante est tenue dans
`specs/android-client/validation.md`.
