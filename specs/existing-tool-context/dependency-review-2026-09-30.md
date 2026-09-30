# Correctif des dépendances après qualification T17

Date : 2026-09-30. Base fonctionnelle : `520a6a0`.

Les deux audits distants du candidat T17 ont échoué sur `npm audit` ; aucun
résultat d'audit Rust ne peut être déduit de ces jobs arrêtés à cette étape.
Les autres contrôles terminés à la lecture du 30 septembre (qualité, Chromium,
Tauri, images API/web et politique de packaging) avaient réussi. Les deux jobs
PostgreSQL étaient encore en cours à cette lecture.

## Changement ciblé

Seul le verrou npm change, dans les plages de versions déjà autorisées :

| Dépendance transitive | Avant | Après |
| --- | --- | --- |
| brace-expansion | 5.0.9 | 5.0.12 |
| fast-uri | 3.1.7 | 3.1.8 |
| ip-address | 10.5.0 | 10.7.2 |
| undici | 7.29.0 | 7.30.0 |

Les alertes touchent notamment les traitements d'entrées malformées et Undici.
Références primaires : [brace-expansion](https://github.com/advisories/GHSA-q2hr-2g5m-vwhr),
[fast-uri](https://github.com/advisories/GHSA-hrr3-gc8f-f4qj),
[ip-address](https://github.com/advisories/GHSA-rpw4-54j3-4h4q),
[Undici](https://github.com/advisories/GHSA-w293-vg96-wgc3).
L'audit initial listait plusieurs avis pour certaines dépendances ; cette liste
de liens n'en constitue pas un inventaire exhaustif.

Aucun manifeste, version majeure imposée ou seuil d'audit n'a été modifié.

## Preuves locales

- Audit npm après installation : zéro vulnérabilité connue, y compris
  `npm audit --omit=dev` (rapport JSON privé `.run/t17-npm-audit-after.json`).
- Sur une copie des fichiers suivis de `520a6a0`, avec le nouveau verrou et les
  dépendances installées : **192 tests web dans 50 fichiers**, lint et build web
  réussis (`.run/t17-security-baseline-checks.log`). La copie exclut les travaux
  T18/T19 en cours dans le checkout partagé.
- Le binaire local `cargo-audit` est absent. Le verrou Rust n'est pas modifié ;
  son audit doit être confirmé par la CI du correctif.

La CI doit être relue sur le nouveau commit avant de fermer la qualification
distante. Un audit sans alerte connue ne certifie ni une absence exhaustive de
vulnérabilités ni un déploiement en production.
