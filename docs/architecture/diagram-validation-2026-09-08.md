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
