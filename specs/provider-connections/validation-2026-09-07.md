# Livraison locale — connexions IA personnelles

Date : 7 septembre 2026. Branche : `feat/alpha-context-proof`.
Périmètre : exigences PC-001 à PC-016 de la [spécification](spec.md).
Les E2E et les essais avec des comptes réels sont confiés au propriétaire par
sa consigne du 7 septembre ; ils ne sont pas des résultats de cette livraison.

## Version et intégration

Base : `8861eca0854b74ab43a2a94013e98e804ab41290`.
Première intégration complète : `3ce4216fb8b87d2a63622a369f1640d93b3d9aee`.
Correctif unique après les deux revues :
`5ea7ce348e274ea93044821d9ccb0e4a222fa729`.
Fusion finale : `e8b05415f53c475aa26da5dcabba31d3e4d24370`, avec un arbre
strictement identique au correctif relu et testé. Les changements suivants de
cette remise sont uniquement le présent rapport et ses liens de suivi.

Les tickets transports, backend, abonnement et interface ont été intégrés dans
la même branche. Les seuls conflits de fusion concernaient l'ajout indépendant
de dépendances dans Cargo.lock ; aucune version existante n'a été changée par
ces résolutions. Les nouvelles sources restent locales. Le point publié
antérieurement `91dc76b` ne contient pas cette évolution ; aucune publication,
fusion distante, release ou mise en production n'est certifiée ici.

## Comportement livré

- Plusieurs profils privés nommés, dont plusieurs clés d'un même fournisseur,
  pour OpenAI, Anthropic, Kimi/Moonshot, DeepSeek et OpenRouter.
- Ajout, modification, remplacement de clé, suppression, accès au catalogue et
  sélection persistante depuis **Réglages IA**.
- Clés chiffrées côté serveur, réponses masquées, isolation acteur/workspace,
  routes et requêtes sans cache ; aucun secret dans le stockage navigateur.
- Moteur commun pour conversation, sélection du contexte, plan, couverture et
  Steward, avec identité fixée pendant l'opération et sans fallback silencieux.
- Connexion par abonnement Claude via le client officiel compatible et son
  profil privé ; distinction entre compte authentifié et compte admissible.
  Changement de compte et déconnexion restent proposés à un compte API ou
  d'organisation refusé pour la génération.
- Échecs réseau transitoires pendant le corps HTTP correctement classés ;
  message, identité fournisseur/modèle et échec conservés quand une connexion
  sélectionnée ne peut être déchiffrée, sans doublon au replay.

Le [contrat backend](backend.md) décrit le stockage, les routes et l'outbox ;
le [contrat abonnement](subscriptions.md) précise les limites du client local.

## Tests sur les sources corrigées

| Contrôle | Résultat et portée |
| --- | --- |
| Unités Rust et contrats HTTP/processus simulés | 125 tests réussis ; un test exigeant PostgreSQL reste ignoré dans cette commande. |
| Frontend Vitest/jsdom | 49 tests réussis ; formulaires, changement d'identité, erreurs, connexion et déconnexion simulées. |
| PostgreSQL/RLS réel sur base jetable | 32 assertions pgTAP et 11 intégrations Rust dans six binaires réussies. |
| Compilation/lint Rust | Clippy tous targets avec avertissements refusés, formatage et contrôle du diff réussis. |
| Qualité frontend | Lint, TypeScript, build et formatage des fichiers modifiés réussis. |

Les intégrations PostgreSQL comprennent la confidentialité des profils, les
droits du rôle runtime NOBYPASSRLS, les mutations HTTP idempotentes, la sélection
du moteur et la persistance métier. Les cas de clé maîtresse absente et de
ciphertext corrompu vérifient la conservation du message, le replay sans
doublon et la reprise explicite après correction de la configuration.
La stack de test dédiée a été arrêtée et ses seuls volumes supprimés.
Le service Podman temporaire utilisé pour ces tests a aussi été arrêté et son
socket supprimé ; aucune base utilisateur n'a été modifiée.
Les cinq worktrees de cette implémentation ont été retirés après vérification
de leur propreté et de l'intégration de leurs commits. Le dépôt principal et
les rapports de travail hors dépôt sont conservés.

## Vérifications complémentaires et périmètre temporel

Sur la première intégration `3ce4216`, l'installation npm, le lint et le build
web, le check serveur et le check Tauri Linux ont réussi. Le check Tauri est
une compilation ; aucun client natif n'a été lancé. Le bundle principal
conserve un avertissement de taille supérieure à 500 kB minifié.

Les 27 unités du harness d'évaluation ont réussi sur `7cb917a` ; les 17 unités
des gardes de cible d'intégration ont réussi sur `3ce4216`. Leurs sources n'ont
pas été modifiées par la correction. Le secret scan a réussi une dernière fois
sur la fusion finale `e8b0541`, avec la documentation de remise en cours.
L'audit des dépendances npm de production a réussi sur `3ce4216` : zéro avis
signalé à la date du contrôle, avec un verrou npm inchangé par la correction.
Aucun nouvel audit Rust de sécurité n'est revendiqué.

Les liens locaux ont été vérifiés après la rédaction de cette remise :
46 cibles valides dans 11 documents. Le diff documentaire ne contient pas
d'erreur d'espacement.

Le test lib de persistance GitHub nécessitant une base a réussi séparément
lors de la livraison backend, après ses dernières retouches `no-store` et
`forget(scope)`. Il ne faut pas le compter parmi les 11 intégrations PostgreSQL
du correctif ni le présenter comme rejoué sur ce correctif.

## Revue Standards

La première revue a trouvé deux violations P2 de l'ADR 0003 : délai du corps
HTTP traité comme permanent et échec de résolution avant conservation du
message. Elle a aussi proposé un P3 facultatif de maintenabilité pour les
listes de fournisseurs répétées.

Le correctif conserve les classes timeout/connexion et les reprises bornées,
enregistre l'intention et le model run même si la clé sélectionnée est illisible,
et partage un registre limité aux cinq fournisseurs effectivement livrés.

La seconde revue, en lecture seule sur `5ea7ce3`, clôture les deux P2 et le P3.
Aucun nouveau constat Standards n'est retenu dans le delta. Les scénarios de
tests ont été relus, sans prétendre à une seconde exécution indépendante.

## Revue Spec

La première revue a trouvé un P2 sur PC-012/PC-015 : un compte authentifié mais
non admissible bloquait les actions de récupération, et la vérification de
logout confondait inéligibilité et fin d'authentification.

Le contrat expose désormais `authenticated: boolean | null`, distinct de
`connected`. L'interface garde les actions de récupération et le motif de
refus ; le serveur n'annonce une déconnexion que si le client confirme
`authenticated: false`. Les cas API/Team et le faux logout sont testés.

La seconde revue, en lecture seule sur `5ea7ce3`, clôture ce P2 et ne retient
aucun nouvel écart Spec dans le delta. Les deux axes restent distincts : leur
clôture atteste la correction des constats de code, pas la réussite d'une
recette réelle.

## Commandes de reproduction

Avec les dépendances installées, pour les contrôles sans base :

```bash
cargo test -p ai-center-server --lib --offline
cargo clippy -p ai-center-server --all-targets --offline -- -D warnings
cargo fmt --all --check
npm run test -w @ai-center/web
npm run lint -w @ai-center/web
npm run build:web
python3 -m unittest discover -s scripts/tests -p 'test_alpha_*.py'
python3 -m unittest discover -s scripts/tests -p 'test_integration_target.py'
git diff --check
```

La validation PostgreSQL emploie une stack jetable préparée par
`scripts/integration-stack.sh prepare` et sa garde de cible. Elle vérifie la
posture runtime avec `scripts/sql/verify-runtime-db-role.sql`, exécute pgTAP
sur la base fraîche puis les binaires Rust ci-dessous, avec les URL runtime
et administrateur réservées au sous-processus de tests :

```bash
cargo test -p ai-center-server --test mvp_flow --test context_pack_concurrency --test idempotency_atomicity --test model_run_lifecycle --test targeted_invalidation --test provider_connections --offline -- --test-threads=1
```

Ne pas exécuter ces fixtures contre une base utilisateur. La stack exclusive
de correction `ai-center-ci-f286efe42ee3` a été supprimée par la garde.
Les journaux temporaires de cette passe sont
`/tmp/acp-review-fix-rust-unit-final.log`, `/tmp/acp-review-fix-web-test.log`,
`/tmp/acp-review-fix-db.log`, `/tmp/acp-review-fix-clippy-final.log`,
`/tmp/acp-review-fix-web-lint.log`, `/tmp/acp-review-fix-web-build.log` et
`/tmp/acp-review-fix-db-teardown.log`. Ils sont locaux et non versionnés ; aucun
journal de démarrage contenant les accès des fixtures n'est livré dans Git.

## Recette et limites

La [recette manuelle](manual-verification.md) couvre les parcours confiés au
propriétaire. Aucun E2E, navigateur automatisé, ouverture native du lien de
connexion, login réel ou appel à un fournisseur IA réel n'a été exécuté.
Un accès réussi au catalogue n'est pas une preuve de génération structurée
ni de qualité pour chaque modèle disponible.

L'abonnement désigne une formule fournisseur existante ; aucun abonnement
commercial AI Center n'est ajouté. Le support Claude est limité au client
officiel Linux 2.1.220 inspecté, aux comptes personnels Pro/Max et à l'absence
de politique administrée incompatible. Les autres versions sont refusées
jusqu'à vérification. ChatGPT/Codex reste indisponible, car la désactivation
de tous ses outils d'action n'est pas garantie par le client inspecté.

Pour lancer le projet, suivre le [guide de démarrage](../../docs/development.md).
Une base locale existante doit recevoir les migrations en attente avant le
nouveau serveur ; le guide fournit la séquence sans reset des données.
Les gates opérationnels du plan Alpha Context Proof (campagne réelle,
intégrations réelles et période d'usage) restent distincts de cette livraison
logicielle et ne sont pas présumés accomplis.
