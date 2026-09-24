# Qualification locale des images — 22 septembre 2026

Cette qualification concerne les images Linux amd64 locales reconstruites après
le jalon `9be97b6`. Elles ne sont ni publiées dans un registre ni déployées chez
un hébergeur. Leurs labels indiquent explicitement un état de travail.

## Analyse des paquets

Grype 0.119.0, archive officielle vérifiée par SHA-256
`3fa2dc4b924621ab65404cf08d0b8438d896d80ab949c9d5a4ca283c36004c9b`.
Base de vulnérabilités v6.1.9 du 22 septembre 2026, contrôlée valide par l'outil.
Les scans finaux sont non filtrés ; aucune règle d'ignorance ajoutée.

| Image locale | Total des correspondances | Critiques | Élevées | Moyennes | Autres | Avec correctif disponible |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| API avant correction, Debian 12.11 + curl | 365 | 23 | 105 | 116 | 121 | Présents, dont glibc/GnuTLS/perl |
| API corrigée, Distroless Debian 13 cc | 20 | 0 | 3 | 9 | 8 | 0 |
| Web Nginx 1.30.5 avant retrait des modules inutilisés | 9 | 0 | 3 | 6 | 0 | 0 |
| Web sans modules image/XSLT/GeoIP/NJS ni curl | 4 | 0 | 1 | 3 | 0 | 0 |

Ces nombres comptent des couples paquet/vulnérabilité, pas des failles uniques
ni des chemins exploitables démontrés. « Aucun correctif disponible » n'est pas
une absence de risque. Les rapports JSON complets sont locaux sous
`.run/oci-scan/`, avec les paquets, versions, correspondances et métadonnées DB.
La [documentation du scanner](https://github.com/anchore/grype) décrit sa portée.

## Images et intégrité

- API : `3c083c5de3b481919a3f94fc397e8ad9cc4ea1c48554bf6768cbcf2c6d5b31ec`.
- Web : `0437d27ae15db65d582a80fd1c90c7e9d1c502d0fdd4777bcd542b825c8d0639`.
- Base API : `gcr.io/distroless/cc-debian13:nonroot`, digest
  `sha256:1ca671d851d5cd39326d095b949a1d4e6ec836f59c5022b21efcd2773bd3a6d5`.
  Signature Cosign vérifiée avec l'identité
  `keyless@distroless.iam.gserviceaccount.com` et issuer Google, suivant les
  [instructions officielles Distroless](https://github.com/GoogleContainerTools/distroless#how-do-i-verify-distroless-images).
  Le binaire Cosign 3.1.3 a lui-même été comparé au digest de son asset officiel.
- Base web Nginx 1.30.5 figée par digest dans le Dockerfile. Les modules retirés
  ne sont pas chargés par la configuration de l'application, qui sert les
  fichiers statiques et relaie les requêtes API.

## Signalements élevés restants

| Signalement | Analyse sur les binaires/configurations inspectés | Limite |
| --- | --- | --- |
| CVE-2026-85091, zlib, API et web | L'avis décrit `gzwrite`/`gzprintf`/`gzvprintf` après blocage d'écriture. L'API ne lie pas libz ; aucun de ces symboles n'est importé par les deux exécutables. La configuration Nginx n'active pas de traitement de fichiers gzip via ces fonctions. | Analyse de chemin, pas preuve d'absence universelle ; la bibliothèque reste inventoriée. [Avis Debian](https://security-tracker.debian.org/tracker/CVE-2026-85091). |
| CVE-2026-19499, glibc, API | Concerne le formatage monétaire `strfmon`/`strfmon_l` ; aucun import de ces fonctions dans l'exécutable Rust. Debian classe le sujet comme mineur sans DSA pour trixie. | Réévaluer si une dépendance introduit ce chemin. [Avis Debian](https://security-tracker.debian.org/tracker/CVE-2026-19499). |
| CVE-2026-5435, glibc, API | Concerne des fonctions obsolètes d'impression DNS ; aucun import `ns_printrrf`, `ns_printrr` ou `fp_nquery` dans l'API. Debian classe le sujet comme mineur sans DSA. | Ne certifie pas toute la libc ; refaire l'analyse après changement de binaire. [Avis Debian](https://security-tracker.debian.org/tracker/CVE-2026-5435). |

Les imports dynamiques ont été relevés avec `readelf` sur les exécutables
extraits des images exactes, conservés uniquement sous `.run/`. Il s'agit
d'une analyse locale de non-atteignabilité des chemins décrits, pas d'une
suppression des alertes ni d'un audit offensif. Les autres correspondances
restent dans le rapport complet ; aucune promesse « zéro CVE ».

## Recettes d'exécution

`scripts/oci-web-smoke.py` passe sur l'image web corrigée : HTML, asset, liens
directs, en-têtes, cache, santé, 404 et API absente explicitement en erreur.

`scripts/integration-stack.sh run tls-smoke oci-api-smoke auth-smoke real-e2e`
passe avec la base/Auth jetables de ce checkout :

1. SQLx se connecte réellement en TLS avec la CA de test approuvée ; autorité
   inconnue et certificat pour un autre nom sont refusés. Le serveur confirme
   `pg_stat_ssl.ssl=true`. La configuration TLS temporaire est restaurée.
2. L'image API utilise le rôle runtime avec RLS, atteint la santé DB, refuse
   les requêtes sans identité et avec bearer invalide, passe son healthcheck
   intégré et s'arrête sur SIGTERM avec code 0. Elle tourne UID 10001 avec
   système de fichiers en lecture seule et sans capacités Linux.
3. Auth Supabase local et les deux parcours navigateur réels passent après
   cette correction. Fournisseur déterministe, aucun appel tiers facturé.

SQLx Rustls est maintenant inclus. La politique de démarrage refuse une base
non loopback sans `sslmode=verify-full`. Les scripts TLS utilisent exclusivement
le superutilisateur **de la stack jetable gardée** pour poser puis retirer les
certificats fictifs ; cette opération n'est jamais un runbook de base de production.

Avant ouverture : reconstruire/étiqueter depuis le commit retenu, refaire le
scan avec sa DB à jour, examiner tout nouveau signalement et vérifier le
transport/authentification/sauvegardes sur l'hébergement réel.
