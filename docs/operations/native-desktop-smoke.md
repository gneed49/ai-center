# Smoke métier natif Linux

Ce smoke complète la compilation Tauri avec un parcours dans le vrai client
Linux : création de projet, session Produit, confirmation de trois connaissances,
couverture, handoff vers Tech, lecture du contexte et rechargement du handoff.
Il réutilise la [stack d'intégration jetable](isolated-integration.md), le rôle
runtime PostgreSQL et le moteur déterministe. Il ne nécessite aucun fournisseur
IA, compte utilisateur externe ou corpus privé.

## Préparer sans lancer le client

```bash
python3 scripts/native-smoke.py prepare
./scripts/ci-desktop.sh native-build
```

La première commande écrit uniquement `.run/native-smoke/tauri.config.json`.
La seconde compile le frontend avec l'API `http://127.0.0.1:4617`, puis le client
Linux dans `target/native-smoke` et enregistre son empreinte dans `build.json`.
`AI_CENTER_NATIVE_TARGET_DIR` permet de choisir un cache de compilation distinct.
La préparation et la compilation ne lancent aucune interface native.

L'overlay utilise l'identifiant `com.aicenter.client.smoke` et remplace uniquement
les origines réseau du test. Il conserve les autres directives CSP. La
configuration produit, ses bundles et les plateformes mobiles ne sont pas
modifiés. Le binaire de test ne doit pas être distribué comme release produit.

## Lancer explicitement le parcours

Prérequis Linux : dépendances de compilation Tauri, moteur de conteneurs,
client PostgreSQL, `Xvfb`, `WebKitWebDriver` et `tauri-driver` 2.0.6. La version
WebKit du pilote doit correspondre à celle de la WebView installée. Les
[instructions Tauri](https://v2.tauri.app/develop/tests/webdriver/manual-setup/)
décrivent ces dépendances ; le [guide CI](https://v2.tauri.app/develop/tests/webdriver/ci/)
explique l'affichage virtuel.

```bash
./scripts/integration-stack.sh run integration native-e2e
```

La commande prépare une stack dédiée, applique et vérifie les migrations,
compile le client de test, lance l'API locale depuis le répertoire isolé puis
exécute le parcours WebDriver. Le smoke refuse les ports 9515/9516 occupés et
un binaire qui ne correspond pas à l'empreinte du build isolé. L'API utilise
4617 ; le frontend est embarqué dans le client, sans serveur web Vite.

Chaque exécution reçoit un profil XDG et un affichage Xvfb temporaires. Le client
n'hérite ni des identifiants fournisseur, ni de la configuration PostgreSQL,
ni du profil graphique de l'opérateur. Ses processus, ceux des pilotes,
l'affichage, le profil et la stack sont nettoyés à la sortie, y compris en cas
d'échec. Un échec produit un code non nul et ne constitue pas une validation.

Le résultat agrégé et la capture se trouvent dans
`.run/native-smoke/artifacts/run.*/`. Seuls `result.json` et `screen.png` sont
publiés par le workflow ; les journaux de l'API et des pilotes restent privés.
Le résultat vérifie une origine `tauri://localhost` et la persistance du handoff
après rechargement. Il ne certifie ni l'authentification distante, ni HTTPS,
ni la campagne réelle ou les durées de dogfood.

## CI manuelle et statut des preuves

Le workflow `Manual Linux native business smoke` accepte un `workflow_dispatch`
avec l'option `run_native_smoke` activée. Il est également réutilisé par le job
Tauri de `Pre-alpha desktop certification`, après sa matrice navigateur. Ce
workflow pré-alpha conserve son seul déclencheur manuel. Le job desktop
ordinaire conserve sa compilation sur PR/push ; le parcours natif n'y est pas
ajouté. Aucun workflow distant n'a été déclenché dans cette livraison.

Au 5 septembre 2026, le code du parcours, sa CI et les tests hors UI sont
préparés. Une compilation native avec overlay a été vérifiée lors de la
préparation de l'environnement ; ce fait ne prouve pas le parcours métier.
L'exécution interactive métier est restée bloquée par la revue automatique
d'autorisation de cette session et n'a pas été relancée par un autre canal.
Le gate ACP-071 reste ouvert jusqu'à une exécution autorisée réussie et la revue
de sa capture. Aucun résultat synthétique des tests de garde ne ferme ce gate.
