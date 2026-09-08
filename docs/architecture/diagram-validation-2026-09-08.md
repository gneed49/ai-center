# Validation documentaire du schéma Alpha — 8 septembre 2026

## Périmètre et références

Cette note clôt la correction documentaire
[ACP-T14](../../specs/alpha-context-proof/tickets.md#acp-t14--vue-figjam-alpha-et-traçabilité-documentaire)
de la phase 0 du [plan Alpha Context Proof](../../specs/alpha-context-proof/plan.md).
La baseline locale est `1a24cfdfcb8fedf109bce7d82e3d2b2e1bf4837b`, sur
`feat/alpha-context-proof`. La source reste le
[flow contextuel Mermaid](context-control-flow.md), avec ses
[invariants](context-control-flow.md#invariants).

Le support FigJam représente le contrat du flow. La validation de ce dessin
ne prouve pas l'exécution des parcours représentés et ne change pas les gates
de recette, campagne réelle, usage propriétaire ou promotion.

## Écart corrigé

**`Vérifié` le 8 septembre par lecture de la page FigJam `0:1` avant ajout :**
le board historique `gn5z1F1tnQpQ23580fQ5QC`, lié depuis README, contient
« Plan simulé » et ne représente pas la chaîne GitHub → ExternalReference → preuve → couverture
du flow Alpha versionné. Le présenter sans distinction comme complément du
schéma courant crée une ambiguïté.

La vue Alpha est ajoutée à côté du dessin historique dans ce même board.
Le dessin antérieur reste conservé comme historique ; les sources Mermaid
et les exports SVG/PNG existants restent inchangés.

## Preuve de la vue Alpha

**`Vérifié` le 8 septembre par relecture `get_figjam` et inspection de la capture
du board :** la [section Alpha Context Proof `7:213`](https://www.figma.com/board/gn5z1F1tnQpQ23580fQ5QC?node-id=7-213),
titrée par le nœud `7:214`, regroupe les sections `6:165` (AI Center) et `6:172`
(production externe). Ses 14 nœuds et 18 connecteurs ont été relus après ce
regroupement et sont alignés sur le flow de référence.

- Le chemin principal relie intention, connaissances, gate, compilation,
  ContextPack et handoff explicite.
- La production est située dans l'outil externe ; GitHub conserve repository,
  PR, commit et checks. Le retour vers AI Center passe par la lecture de
  métadonnées, ExternalReference, la preuve candidate, la validation humaine
  puis la couverture.
- Le steward mène à une décision humaine et à la révision ou à l'invalidation
  ciblée. Le schéma représente le contrôle du contexte, sans runner AI Center.
- Les anciens nœuds `1:*` sont conservés. L'inspection de la capture confirme
  que le dessin historique et la nouvelle vue sont côte à côte sans recouvrement.
- La capture finale de la vue Alpha, de 2048 × 539 pixels pour une section
  de 3800 × 940, a été inspectée : contenu lisible, sans recouvrement ni texte
  tronqué. Les URLs temporaires de capture ne constituent pas des références
  durables ; le lien direct de section ci-dessus est conservé.

## Contrôle des exports existants

**`Vérifié` le 8 septembre :** l'inspection visuelle du
[PNG local](assets/alpha-context-control-flow.png) confirme la boucle Alpha,
GitHub en lecture seule, ExternalReference, validation humaine, couverture et
recompilation. Les mêmes repères textuels sont présents dans le
[SVG](assets/alpha-context-control-flow.svg). Ce contrôle des exports locaux
est distinct de la lecture API et du rendu FigJam. Leur régénération n'est
pas nécessaire pour corriger le lien historique.

## Contrôles du dépôt

**`Vérifié` le 8 septembre :** Prettier valide les six fichiers du lot,
`scripts/check-markdown-links.py` valide leurs 69 cibles locales et
`git diff --check` réussit. La comparaison à `1a24cfd` confirme la conservation
des trois sources Mermaid, des exports et du corpus historique. L'auto-relecture
des conventions du dépôt et de l'acceptation ACP-T14 ne laisse aucun constat
ouvert ; cette vérification documentaire n'exécute pas l'application.

## Portée de la livraison

Seuls les liens, le ticket, son plan, cette note et le suivi du projet changent
dans le dépôt. Aucune source Mermaid, aucun export, code, schéma SQL ou fichier
du corpus historique n'est modifié. Aucun test applicatif, E2E, smoke natif ou
appel fournisseur réel n'est exécuté pour ce lot documentaire.

La mise à jour FigJam est effective dans le board partagé. Les changements du
dépôt sont une livraison locale ; ils n'impliquent aucun push, modification de
PR, lancement de CI, déploiement ou promotion de l'alpha.

## Standards

**HEAD relu :** `af548beaffc0f4546c8c07bbdb8e564d50dc0276`.

**Diff figé :** `git diff 1a24cfdfcb8fedf109bce7d82e3d2b2e1bf4837b...HEAD`.
Commits : `0d690bb` et `af548be`. Six fichiers Markdown ; arbre de travail propre au contrôle initial.

**Violations documentées : zéro.**

- `README.md` et `docs/architecture/README.md` désignent directement la vue Alpha, distinguent le dessin historique et maintiennent Mermaid comme référence. Le delta respecte la préservation des sources et la traçabilité exigées par `AGENTS.md:10,28-30`.
- `specs/alpha-context-proof/plan.md:172-195` et `tickets.md:321-349` relient ce lot au plan, aux exigences et à une acceptation bornée. Cela respecte le maintien du plan et la définition des preuves (`AGENTS.md:19-22`). Aucune décision produit nouvelle nécessitant une modification de `docs/product/` n'est introduite.
- `docs/architecture/diagram-validation-2026-09-08.md` et `PROJECT_STATUS.md` datent la validation, localisent le support et la note, distinguent validation visuelle et exécution fonctionnelle, puis précisent la livraison Git locale et les gates ouvertes. Cela respecte `AGENTS.md:33`, le vocabulaire de preuve de `spec.md:35-46` et la distinction des preuves datées de `docs/agents/issue-tracker.md`.

**Jugements facultatifs : zéro.** Aucun des douze smells de la baseline n'appelle de correction sur ce delta documentaire ; les renvois répétés assurent la navigation entre statut, ticket et preuve.

**Limites :** inspection indépendante des documents locaux et du diff uniquement. Le fichier de preuves FigJam transmis par le coordinateur a été lu comme preuve rapportée : je n'ai ni interrogé FigJam ni inspecté moi-même sa capture. Les résultats de formatage et de liens déjà rapportés n'ont pas été rejoués. Aucun test applicatif, E2E, réseau ou changement dans le dépôt.

**Bilan Standards : 0 constat bloquant, 0 constat facultatif.**

## Spec

**HEAD examiné :** `af548beaffc0f4546c8c07bbdb8e564d50dc0276`.
**Comparaison :** `git diff 1a24cfdfcb8fedf109bce7d82e3d2b2e1bf4837b...HEAD`.
Commits : `0d690bb` et `af548be`. Le delta contient six fichiers Markdown.

**Résultat : zéro constat.** Aucune demande manquante ou partielle, aucun
élargissement injustifié et aucune mise en œuvre erronée établis dans ce delta.

- La phase 0 demande « Conserver Mermaid comme source versionnée ; produire
  en complément un diagramme FigJam propre et un export SVG/PNG »
  (plan directeur fourni, phase 0, lignes 53–57). Les trois sources
  Mermaid et les exports existants sont conservés ; le complément apporte
  la vue Alpha et ses références documentaires.
- ACP-T14 exige une « vue Alpha distincte, alignée sur le flow versionné »,
  la relecture des nœuds et connexions, des liens directs, la préservation de
  l'historique et une note durable (`specs/alpha-context-proof/tickets.md:331-337`).
  Le relevé de structure fourni par le coordinateur
  contient les 14 nœuds et les 18 arêtes orientées de
  `docs/architecture/context-control-flow.md:5-43`, sans arête manquante ou
  supplémentaire. Les libellés préservent les branches oui/non, l'export
  JSON/Markdown, la production externe, la lecture autorisée, la validation
  humaine, la révision réelle et l'invalidation ciblée. README et l'index
  d'architecture ciblent tous deux la section `7:213` ; la note datée est reliée
  au statut du projet.
- La portée prescrit « aucune preuve d'exécution des parcours représentés »
  et aucune fermeture de gate (`specs/alpha-context-proof/tickets.md:339-341`).
  Les ajouts respectent cette limite et précisent la livraison Git locale.

**Limite de revue :** comparaison indépendante du diff et du relevé fourni,
sans inspection distante indépendante. L'apparence du board et la conservation
effective de ses anciens nœuds reposent sur ce relevé et la note de validation.
Aucun test applicatif, E2E, essai natif, appel fournisseur ou accès réseau.

**Bilan des deux axes : Standards 0 constat ; Spec 0 constat. Aucun problème de sévérité à classer.**

## Intégration et nettoyage

Le commit documentaire `0d690bbec1a74c6a4a9666109c1eff7ccae5de4d` est intégré
par `af548beaffc0f4546c8c07bbdb8e564d50dc0276`, avec un arbre strictement
identique. Les deux revues ci-dessus portent sur cette fusion ; elles n'ont
pas demandé de correction. Leur consignation ici est documentaire uniquement.

Le worktree `ai-center-alpha-diagram-docs` a été retiré sans forcer après
vérification de sa propreté et de l'intégration de son commit. Sa branche est
conservée et seul le checkout principal reste présent. Aucun environnement de
test ou service supplémentaire n'a été démarré pour ACP-T14.
