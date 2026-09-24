# Validation intégrée — 21 septembre 2026

> Candidate locale en cours sur `feat/company-context-v1`, baseline `966ce9b`.
> Aucun commit de cette V1, push, déploiement ni fournisseur réel qualifié à ce point.
> Ce registre sera fermé uniquement après recette du candidat final, y compris
> le complément de portabilité des données par projet.

## Résultats exécutés

| Périmètre                            | Résultat observé                                                                                                                 | Limite                                                                           |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Qualité commune initiale             | 151 unités Rust, 3 contrats synthétiques, 142 tests web, 73 tests Python ; format, lint, Clippy workspace, build web/API passent | Photo avant corrections finales et complément projet                             |
| Frontend final des parcours d'équipe | 149 tests web /39 fichiers, lint/build passent sous Vitest 4.1.11                                                                | Recette API simulée et unités, distinctes de PostgreSQL                          |
| Chromium bureau/compact              | 94 scénarios fonctionnels puis deux parcours visuels de six surfaces passent                                                     | Fixtures `[FICTIF]`, pas de compte tiers                                         |
| Firefox                              | 48 scénarios passent, dont accessibilité/débordement des nouvelles surfaces                                                      | Navigateur installé dans le cache utilisateur ; aucun paquet système             |
| Export projet UI                     | 3 recettes de téléchargement passent sur Chromium bureau/compact et Firefox                                                      | API simulée ; backend en qualification                                           |
| Migration                            | Baseline → 11 migrations ; données et propriétaires legacy préservés, 32 pgTAP et vérification runtime/RLS passent               | Précède complément opérateur projet                                              |
| Métier PostgreSQL                    | MVP, 2 projets synthétiques, concurrence packs, idempotence, lifecycle3, invalidation, artefacts2, invitations4, outils6 passent | Moteur déterministe/HTTP local                                                   |
| Reprise et quotas                    | Reprise conversation2, identité message, automatisation5, reports outbox2, suivi GitHub1 passent                                 | Deux tests anciens restants corrigés ci-dessous, recette suivante requise        |
| Société/contexte                     | 14/15 passent : permissions, graphe concurrent, règles inter-projets, code sourcé, retrieval grand corpus, archives et renommage | Purge société échouait sur nom SQL ambigu, corrigé sans nouvelle preuve de purge |
| Restauration                         | 56 tables, 133 politiques, schéma/données/grants/RLS restaurés dans une base temporaire                                          | Pas infrastructure de production, clés restaurées non encore exercées sur cible  |
| Auth                                 | Magic-link/OTP réel Supabase local et frontière de rôle/entreprise déjà exercés                                                  | Nouvelle recette privée/retrait en cours ; aucun SMTP externe                    |
| OCI                                  | API et web construits localement via Podman                                                                                      | Rebuild final/configuration cible et publication encore requis                   |

Les comptes détaillés ne s'additionnent pas en un nombre de validations du
produit : certaines suites ont plusieurs photos. La suite PostgreSQL complète
a volontairement collecté tous les résultats et terminé en échec sur deux tests,
sans masquer leurs échecs par les autres contrôles verts.

## Corrections issues des recettes et revues

- Invitation ancienne empêchée de réadmettre un membre retiré ; une nouvelle
  invitation explicite reste possible.
- Création d'entreprise privée : seuls les comptes autorisés ou propriétaires
  existants peuvent créer un workspace consommant les ressources serveur.
- Arrêt IA concurrent, sans bloquer le transport pendant la vérification DB ;
  abandon de la ressource avant settlement. Deadline classée comme échec réel,
  distincte d'un report de quota.
- Publication en vol recontrôlant droits d'écriture et génération du réglage ;
  rétrogradation et pause/reprise rapides ne sont plus ignorées.
- Message lié à son auteur, à son identité et à sa commande d'origine. Ancien test
  attendant un contournement d'échec terminal par nouvelle clé remplacé par refus
  explicite puis nouvel envoi distinct ; nouveau passage DB requis.
- Deux verrous de scopes acquis dans un ordre stable pour les relations du graphe.
- Version du pack décidée par les sources serveur, pas le seul compteur du graphe.
- Route du test graphe corrigée pour suivre le vrai lien ; erreur de test distinguée
  d'un défaut de rendu. Historiques archivés conservés et mutations refusées.
- Lecture de code : fichiers secrets connus refusés avant réseau ; motifs de
  credentials exclus avant stockage. Cas base64 avec `=` corrigé. Filtre conservateur,
  aucune promesse d'absence exhaustive de secret dans un corpus arbitraire.
- Journaux d'erreur DB limités au SQLSTATE : aucun diagnostic contenant une ligne
  métier complète. Messages internes non journalisés avec leur texte brut.
- Script d'effacement : cible hôte/port/base fixée, paramètres libpq hérités pouvant
  rediriger la connexion neutralisés, certificat distant vérifié, confirmations et
  refus couverts par quatre tests sans accès à une base.

[Revue indépendante](independent-reliability-review.md). Le traitement contrôlé
des opérations abandonnées avant effacement reste ouvert dans cette revue.

## Dépendances et sécurité

Audit npm après corrections : aucune vulnérabilité signalée. Mises à jour
ciblées : Hono 4.13.8, js-yaml 4.3.2 et Vitest 4.1.11. Les correctifs js-yaml et
Vitest sont documentés par leurs mainteneurs :
[js-yaml](https://github.com/nodeca/js-yaml/security/advisories/GHSA-2883-xcg3-v3hh),
[Vitest](https://github.com/vitest-dev/vitest/security/advisories/GHSA-82fw-gwwq-j7x9).

Audit Rust : rustls porté à 0.23.45 pour
[RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html),
chacha20 0.10.2 remplace une version retirée. Le nouvel audit termine sans
vulnérabilité bloquante mais conserve sept avertissements : six dépendances
non maintenues et glib 0.18.5 signalée `unsound`. Les chemins inverses de
l'arbre de dépendances les relient au client Tauri Linux. Ils ne sont pas
masqués et devront être traités avant qualification de ce client natif ; la
candidate courante cible l'API Rust et le navigateur.

Aucun audit automatique ni absence de constat ne constitue une certification
exhaustive. Les migrations, les droits et les scénarios métier restent qualifiés
par leurs preuves séparées.

## Dépendances externes ouvertes

- Compte/modèle et budget IA explicitement autorisés.
- Espaces de qualification Notion, Linear et GitHub désignés par le propriétaire.
- Hébergement, domaine, Auth/SMTP, secrets, surveillance et restauration sur la cible.
- Pilotage humain dans la durée prévue par CC-G4.

Aucune clé n'est demandée dans le chat ou consignée dans ce rapport. Les comptes
et services réels n'ont pas été simulés pour fermer ces étapes.
