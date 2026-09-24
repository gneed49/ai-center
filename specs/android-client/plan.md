# Plan d’implémentation — Client Android

> Statut : validation native en attente  
> Spec liée : `spec.md`

## Étapes

1. Vérifier Java 21, Android SDK/NDK, Rust Android et les outils Gradle.
2. Ajouter la décision d’architecture mobile et les scripts reproductibles.
3. Initialiser la cible Tauri Android avec un identifiant stable.
4. Ajouter le résolveur d’URL API par plateforme et ses tests.
5. Ajuster CSP, CORS, manifeste réseau et zones système.
6. Construire APK et AAB pour les ABI Android supportées.
7. Inspecter, signer/vérifier, installer et lancer sur une cible disponible.
8. Rejouer les tests web/Rust et documenter les preuves.

## Validation

- [x] Configuration Tauri/Gradle générée
- [x] Tests unitaires URL et plateforme
- [x] Lint TypeScript/Rust
- [x] Build frontend production
- [x] APK Android
- [x] AAB Android
- [x] Contrôle manifeste/package/minSdk
- [x] Contrôle signature et secrets
- [ ] Smoke test Android ou preuve documentée d’absence de cible d’exécution
- [x] Non-régression desktop
