# AI Center — client web

Interface React/TypeScript partagée par le navigateur et l’enveloppe Tauri.
Elle consomme exclusivement l’API Rust : aucun secret ni accès direct à la base
n’est inclus dans le bundle.

Depuis la racine du dépôt :

```bash
npm run dev:web
npm run lint
npm test
npm run build:web
```

La variable publique optionnelle `VITE_API_URL` permet de changer l’adresse de
l’API, qui vaut `http://127.0.0.1:4317` par défaut.
