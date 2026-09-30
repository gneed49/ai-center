# Validation — deux projets fictifs

Date : 8 septembre 2026. Base du lot : `85b5517`.
Tests livrés dans `b5999f0`, intégrés sans changement à `5e0e98d`.
Demande : [spécification et amendement](spec.md).

## Résultat reproduit

**Vérifié localement :** Médiathèque Partagée et Atelier Réparable parcourent
les services réels d'AI Center avec persistance PostgreSQL. Le moteur
`DeterministicEngine` est le double de fournisseur existant. Aucun code produit
n'a dû être corrigé pour satisfaire ces cas.

Les deux cas sont versionnés dans les
[fixtures JSON](../../apps/server/tests/fixtures/synthetic-projects.json).
Ils ne sont pas insérés dans le seed de développement. Le test crée ses
projets avec des noms `[FICTIF]` uniques dans une base jetable.

| Cas                  | Résultat vérifié                                                                                                                                                                                                                                                                      |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Médiathèque Partagée | La conservation de l'historique des prêts sans expiration passe de l'intention confirmée au brief, au contexte et au plan technique. Une purge à 30 jours crée une contradiction bloquante ; sa résolution crée une nouvelle version et permet un nouveau handoff et un nouveau plan. |
| Atelier Réparable    | La suppression des photos de diagnostic à 30 jours reste présente dans son propre contexte et son plan. Son snapshot complet et son ContextPack restent identiques et courants pendant la contradiction et la révision de la médiathèque.                                             |

Le [test d'intégration](../../apps/server/tests/synthetic_projects.rs) exécute
les deux projets ensemble dans le même workspace. Il contrôle également :

- le gate bloqué avant confirmation, puis passé après confirmation ;
- l'exclusion du brouillon non confirmé et d'une ancienne note technique ;
- les versions et la provenance rapportées au snapshot du projet, la sélection
  obligatoire/facultative et la concordance de l'export JSON et de son hash ;
- le refus d'un pack ou d'une session appartenant à l'autre projet ;
- les sources des plans limitées à leur propre contexte ;
- le refus du contexte périmé, l'ancien contenu immuable, les versions avant et
  après résolution dans l'audit, puis la recompilation ;
- la couverture toujours manquante en l'absence de preuve externe.

Les [contrats hors base](../../apps/server/tests/synthetic_context_contract.rs)
vérifient sur les deux fixtures la sélection d'un historique facultatif,
la priorité des obligations quand le budget est limité et le refus d'un
identifiant de source étranger à la liste de candidats.

## Exécutions

| Contrôle                                                                        | Résultat courant                                      | Journal local                                        |
| ------------------------------------------------------------------------------- | ----------------------------------------------------- | ---------------------------------------------------- |
| `cargo test -p ai-center-server --lib`                                          | 125 réussis, 1 ignoré                                 | `/tmp/acp-synthetic-server-unit.log`                 |
| `cargo test -p ai-center-server --test synthetic_context_contract`              | 3 contrats réussis, chacun couvrant les deux fixtures | Rapport d'implémentation SYN-T01                     |
| `cargo test -p ai-center-server --test synthetic_projects -- --test-threads=1`  | 1 intégration réussie réunissant les 2 projets        | `/tmp/acp-synthetic-worktree-projects-corrected.log` |
| pgTAP sur la stack exclusive                                                    | 32 réussis                                            | `/tmp/acp-synthetic-pgtap.log`                       |
| `python3 -m unittest discover -s scripts/tests -p 'test_alpha*.py'`             | 41 réussis                                            | `/tmp/acp-synthetic-alpha-unit.log`                  |
| `python3 -m unittest discover -s scripts/tests -p 'test_integration_target.py'` | 17 réussis                                            | `/tmp/acp-synthetic-target-unit.log`                 |
| Identité du rôle de connexion                                                   | `ai_center_runtime`, superuser=false, bypassrls=false | `/tmp/acp-synthetic-runtime-check.log`               |

Le test Rust ignoré est la preuve PostgreSQL de persistance GitHub, normalement
collectée séparément en phase `integration` ; elle n'a pas été réexécutée dans
ce lot. Ces nombres ne comptent pas les deux fixtures comme deux tests Rust.

Le premier essai PostgreSQL a refusé une session Tech ouverte directement.
Le montage du test a été corrigé pour la créer par un handoff après compilation,
conformément au contrat du produit ; le second essai a réussi. Cet échec initial
ne constitue pas une régression produit corrigée. Les premiers tests Python
nécessitant HTTP loopback ont rencontré le refus de socket du sandbox ; leur
exécution autorisée avec sockets locaux a ensuite réussi.

## Environnement et reproduction

Stack exclusive `ai-center-ci-75ce22554434`, PostgreSQL sur le port 55322,
migrations du checkout courant et rôle runtime sans contournement RLS. Les
gardes existantes ont vérifié l'identité avant les opérations SQL. La base de
développement et le checkout canonique n'ont pas été utilisés.

L'hôte ne fournit pas les clients PostgreSQL 17 nécessaires. Des wrappers
privés utilisent ceux du conteneur exclusif ; ils refusent un endpoint autre
que la base loopback attendue. Les identifiants runtime sont générés pour ce
run et ne figurent ni dans les fixtures ni dans le code du helper privé.
La sélection du workdir et le cycle local suivent les options de la
[CLI Supabase](https://supabase.com/docs/reference/cli/supabase-start).

Les commandes des nouveaux tests sont collectées par `quality` et
`integration` dans [ci-desktop.sh](../../scripts/ci-desktop.sh). Sur une machine
disposant des prérequis du [runbook](../../docs/operations/isolated-integration.md),
la reproduction standard sans E2E est :

```bash
cargo test -p ai-center-server --test synthetic_context_contract
./scripts/integration-stack.sh run integration
```

La commande de reproduction `integration` exécute aussi les autres suites
PostgreSQL du dépôt. Dans cette passe, le coordinateur a exécuté les contrôles
listés ci-dessus via `/tmp/run-acp-synthetic.py`, sur la stack gardée déjà
démarrée, depuis le worktree d'implémentation pour le nouveau binaire. Le
rapport ne revendique pas une exécution de toutes les autres suites DB.

## Revue et portée

Les [revues indépendantes](review-2026-09-08.md) portent sur les onze fichiers
du diff `85b5517...5e0e98d` : Standards — zéro violation et zéro jugement
facultatif ; Spec — zéro constat. Aucun correctif de revue n'est nécessaire.
Clippy strict des deux nouveaux binaires réussit, ainsi que le format Rust,
la syntaxe Bash, le contrôle des liens Markdown (162 cibles dans 76 fichiers
au point de consolidation) et Prettier sur les fichiers concernés.

La stack et ses seuls volumes ont été supprimés : le journal
`/tmp/acp-synthetic-stop.log` confirme le nettoyage ciblé, puis les inventaires
Podman filtrés par cette identité sont vides. Le fichier privé de connexion
runtime a été supprimé. Le service Podman temporaire s'est arrêté avec code 0.
Le worktree d'implémentation propre a été retiré après vérification de son
intégration ; sa branche reste conservée. SYN-T01 et SYN-T02 sont terminés.

Ces résultats prouvent les contrats logiciels testés avec deux domaines
fictifs. Ils ne mesurent pas la compréhension d'un modèle réel, un avantage
A/B, un compte fournisseur, une recette E2E ou une exploitation de l'alpha.
Le choix de deux projets réels n'est plus attendu pour ce lot. Aucun navigateur,
client natif, appel fournisseur réel, déploiement ou push n'a été lancé.
