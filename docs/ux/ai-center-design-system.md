# AI Center — Système visuel du MVP

> Statut : référence d’implémentation  
> Date : 2026-08-18

## Concepts actifs

- `concepts/credits-v2-command-center-desktop.png`
- `concepts/credits-v2-command-center-mobile.png`
- `concepts/credits-v2-product-session-desktop.png`

Les anciens concepts handoff restent des sources de direction. Les trois images
ci-dessus sont la référence de fidélité du MVP.

## Direction

Un plan de contrôle calme et éditorial : le projet et ses objets durables
dominent, la conversation reste secondaire. Le graphe se comprend par un rail
de workflow, les versions, les sources, la couverture et les contradictions.

## Tokens

- Fond : blanc réel `#ffffff`.
- Navigation : bleu nuit `#061425` à `#0a1c31`.
- Texte principal : `#0b1428`.
- Texte secondaire : `#697386`.
- Accent : indigo `#4338ca`, actif `#3427c7`.
- Succès : `#079455`.
- Avertissement : `#e66a00`.
- Danger : `#dc2626`.
- Bordure : `#d9dee8`.
- Surface discrète : `#f7f8fb`.
- Rayon : 6 px contrôles, 8 px panneaux ; aucun grand wrapper arrondi.
- Ombre : seulement pour surélévation interactive, très diffuse.
- Espacement : base 4 px ; rythmes principaux 8, 12, 16, 24, 32.
- Mouvement : 160–220 ms, déplacements courts, réduit si demandé par l’OS.

## Typographie

Inter ou une sans-serif métriquement compatible. Titres 700, texte 400–500,
contrôles 500–600. Desktop : titre projet 48 px, sections 20–24 px, corps
14–16 px. Mobile : titre projet 32 px, sections 20–24 px, corps 15–17 px.
Les contrôles n’héritent jamais d’une taille navigateur implicite.

## Anatomie

- Desktop : rail de navigation 200 px, contenu flexible, inbox 320 px.
- Mobile : top bar simple, rail vertical, composer puis navigation basse.
- Rail workflow : ligne indigo, icônes cerclées, une étape sélectionnée.
- Listes et tables ouvertes ; cartes seulement pour un objet sélectionnable.
- Statuts combinent icône, libellé et couleur.
- Les sources utilisent identifiant, scope, version et lien de détail.

## Composants

AppShell, ProjectHeader, ScopeTabs, WorkflowRail, WorkflowStep,
KnowledgeProposal, SourceReference, StatusLabel, CoverageMatrix,
CoverageSummary, DecisionInbox, InsightDetail, ConversationThread,
Composer, ConfirmMutationBar, DeliverableViewer et HistoryTimeline.

## Icônes

Icônes filaires cohérentes, trait 1.75–2 px : accueil, dossier, balance,
exécution, réglages, cube Produit, code Tech, document, package de contexte,
handoff, checklist, bouclier, warning et lien externe. Les icônes de boutons
sont alignées optiquement et ne portent pas de taille locale arbitraire.

## Copie autorisée au-dessus de la ligne de flottaison

`AI Center`, `Center`, `Projets`, `Décisions`, `Exécutions`, `Credits v2`,
`Produit`, `Tech`, `1 décision requise`, `2 livrables à jour`,
`ProductReadyGate`, `Prêt avec 1 avertissement`, `Feature Brief`,
`ContextPack Tech`, `Handoff`, `Technical Delivery Plan`,
`Ouvrir le handoff`, `Decision Inbox`.

## Responsive

Sous 960 px, la navigation latérale devient une navigation basse, l’inbox
devient une route ou un drawer, et la table de couverture devient une liste
structurée. Le workflow conserve sa séquence et son action principale.

## Interactions obligatoires

- création et sélection d’un projet ;
- changement de scope Produit/Tech ;
- ouverture d’une session et envoi d’un message ;
- inspection, modification, confirmation ou rejet des propositions ;
- génération du FeatureBrief et évaluation du gate ;
- ouverture du handoff et création de la session Tech ;
- inspection du plan et de la couverture ;
- acceptation, rejet ou résolution d’un insight ;
- navigation dans l’historique.
