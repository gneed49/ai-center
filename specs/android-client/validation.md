# Validation — Client Android

> Date : 2026-08-18  
> Statut : preuves de build complètes, smoke test natif en attente

## Matrice d’acceptation

| Critère | État       | Preuve                                                                              |
| ------- | ---------- | ----------------------------------------------------------------------------------- |
| AND-01  | réussi     | projet Gradle généré sous `apps/desktop/src-tauri/gen/android`                      |
| AND-02  | réussi     | manifeste release : `com.aicenter.client`, minSdk 24, targetSdk 36                  |
| AND-03  | réussi     | APK debug arm64 et x86_64 signés en APK Signature Scheme v2                         |
| AND-04  | réussi     | AAB universel contenant les bibliothèques arm64-v8a et x86_64                       |
| AND-05  | en attente | le bundle web est validé, mais l’émulateur hôte quitte avant le boot Android        |
| AND-06  | réussi     | `VITE_API_URL` prioritaire et normalisé par test unitaire                           |
| AND-07  | réussi     | tests unitaires `10.0.2.2` sur Android et `127.0.0.1` ailleurs                      |
| AND-08  | réussi     | routes Centre et projet validées à 390 × 844, sans overlay ni erreur console        |
| AND-09  | en attente | styles `env(safe-area-inset-*)` présents ; preuve sur appareil encore nécessaire    |
| AND-10  | réussi     | CORS limité aux origines web/Tauri explicites ; CSP limitée aux hôtes locaux connus |
| AND-11  | réussi     | audit des sources et artefacts décompressés sans secret serveur                     |
| AND-12  | réussi     | tests web/Rust/PostgreSQL, lint, build web et build desktop réussis                 |

## Artefacts

| Artefact                             |             Taille | SHA-256                                                            |
| ------------------------------------ | -----------------: | ------------------------------------------------------------------ |
| `app-arm64-debug.apk`                | 240 896 205 octets | `d0c5bc7e07805591620fd3e885eb2f371763f0bd0100ba568c1722f6643ec6db` |
| `app-x86_64-debug.apk`               | 241 683 242 octets | `6dd0bc30c749ec937c645aa81353504bd1c9ef20deaecdc93f12af2832bad4b4` |
| `app-universal-release-unsigned.apk` |  19 877 779 octets | `8fcd71ce4bb73d073429f6a2415072a1a52385d1d6bb863fb8f29e4e68d0b4ca` |
| `app-universal-release.aab`          |   9 175 179 octets | `c445f52a03db435ce39deb30dfc0148bf9bd4bb95b13563000cdbdac80fa648b` |

> L’empreinte AAB est vérifiée automatiquement par `npm run verify:android` ;
> l’APK/AAB release restent non signés conformément au périmètre MVP.

## Manifeste et sécurité

- permission réseau : `android.permission.INTERNET` ;
- package debug : `com.aicenter.client.debug` ;
- package release : `com.aicenter.client` ;
- trafic HTTP en clair : autorisé en debug, refusé en release ;
- ABI release : `arm64-v8a`, `x86_64` ;
- viewport bord-à-bord : `viewport-fit=cover` et variables CSS
  `safe-area-inset-*` ;
- aucune occurrence de `OPENAI_API_KEY`, `DATABASE_URL` ou clé `sk-proj` dans
  les artefacts décompressés.

## Limite d’exécution locale

L’Android Emulator 37.1.11 puis la version stable 36.3.10 ont été essayés avec
KVM, sans accélération, SwiftShader, rendu désactivé, Vulkan désactivé et
plusieurs tailles de fenêtre. Le processus quitte avec le code 139 pendant le
boot du système, avant installation de l’APK. `adb` voit brièvement la cible,
puis elle disparaît. Cette limite concerne l’environnement hôte ; elle ne
constitue pas une preuve de fonctionnement ou de panne de l’application.

La clôture des critères AND-05 et AND-09 exige donc un téléphone Android avec
débogage USB ou un émulateur fonctionnel sur une autre machine.
