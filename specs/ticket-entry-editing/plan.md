# Plan — Édition du nombre d'entrées de tickets

> Statut : frontend, TP-011 et quatre parcours métier qualifiés localement ; CI en attente
> Spec liée : [spec.md](spec.md)
> Dernière mise à jour : 2026-09-30

## Approche

Étendre l'éditeur structuré existant. La nouvelle interface reste une préparation
locale de la commande de révision actuelle : ni nouveau modèle de tâche, ni route,
ni table, ni changement de protocole de publication. Les sources des entrées
conservées et de la version restent inchangées ; un ticket ajouté manuellement
commence sans citation. Aucun rapprochement automatique entre versions.

## Réalisation minimale

1. **Contrat du formulaire.** Faire parvenir le type réel d'artefact à
   `TypedDraftEditor` via `ArtifactEditor`. Activer les commandes uniquement pour
   les deux types de tickets au format pris en charge ; ne pas déduire le droit
   d'ajouter de la seule présence d'un tableau `tickets`. Garder les formats
   historiques hors de cette conversion.
2. **État local et commandes.** Ajouter en fin, retirer et restaurer le dernier
   retrait avec sa position. Utiliser des identités de rendu locales stables,
   distinctes des indices de publication ; elles ne sont pas envoyées au serveur.
   Restaurer dans la liste actuelle, sans restaurer un ancien état complet qui
   écraserait les autres saisies. Invalider l'annulation du retrait selon la spec.
3. **Validation utile.** Réutiliser les champs et les bornes du contrat serveur.
   Ajouter des erreurs par entrée et un contrôle des tailles UTF-8/du brouillon
   avant soumission, en conservant la saisie. Normaliser uniquement les séparateurs
   et lignes vides des critères ; produire le Markdown depuis le même brouillon
   normalisé que le JSON envoyé. Le serveur reste la dernière autorité.
4. **Sources et version.** Garder `source_ids` attachés à leur entrée et ne pas
   modifier les sources globales de l'éditeur typé. Les nouvelles entrées ont
   `source_ids=[]`, avec libellé explicite de saisie manuelle. Réemployer la
   révision optimiste, les snapshots conservés et la clé idempotente existants.
   Après sauvegarde, l'URL et la sélection de tickets repartent de la nouvelle
   version selon le comportement actuel ; aucun ancien reçu n'est réassigné.
5. **Accessibilité.** Boutons non soumis (`type=button`), compte et messages
   annoncés, noms associés au ticket, restauration de focus documentée, actions
   désactivées pendant la sauvegarde. Conserver l'annulation générale de la page
   et rendre le focus à son déclencheur. Aucun dialogue de confirmation requis
   pour un retrait local immédiatement annulable.
6. **Preuves et documentation.** Exécuter les tests ci-dessous, faire relire les
   transitions/filiations et corriger les défauts. Après validation seulement,
   remplacer la limite explicite du parcours web et la note différée de T16 par
   le comportement réellement livré. Le coordinateur met à jour le suivi global.

## Tests ciblés

- Composant : 1/2/29/30 entrées ; retrait au milieu avec sources distinctes,
  annulation après modification d'une autre entrée ; nouvel ajout sans sources ;
  champs vides/UTF-8/21 critères, état occupé, focus et absence de submit implicite.
- Page : abandon puis réouverture, erreur et conflit avec saisie conservée ;
  requête contenant exactement les entrées attendues, leurs citations et le
  Markdown correspondant ; aucune validation/publication ni génération appelée.
- Réutiliser les contrôles serveur existants de `generation_contract` et
  `validation` ; ne pas les assouplir pour accepter un formulaire incomplet.
  Ajouter un test serveur seulement si une lacune du contrat est révélée.
- Modifier `ticket-real-e2e/workflow.spec.ts` pour construire les 2 puis 3 entrées
  dans l'éditeur, au lieu de l'injection directe par l'API de révision de TP-011.
  Garder la vraie API/DB locale et les connecteurs HTTP **[FICTIF]**, le contrôle
  des cinq créations distinctes, de leurs sources et des reçus. Ajouter le retrait
  dans une révision suivante et vérifier l'ancien lien version/index, ainsi que
  l'avertissement de nouvelle publication ; aucun compte réel requis.
- Rejouer les régressions de publication/historique concernées, lint, typecheck
  et construction web. Contrôle clavier, accessibilité et viewport 390 px dans
  le parcours navigateur, sans construire une deuxième matrice de tests.

## Impacts, livraison et retour arrière

Principaux fichiers : `typed-draft-editor.tsx`, `artifact-editor.tsx`, helpers du
brouillon si nécessaires, `artifact-page.tsx` pour le focus, leurs tests et le
parcours TP-011. Le format `agent-artifact-v1`, les APIs, quotas, index et snapshots
serveur restent compatibles. Aucun secret, appel IA ou accès distant additionnel.

Le développement a commencé après la fin du gel T17. Livrer le client vérifié
avec le serveur actuel ; aucune migration. Un retour au client
précédent enlève les nouvelles commandes mais conserve les documents enregistrés,
versions, reçus et issues. Il ne retire aucune entrée déjà sauvegardée.

## Réalisation et preuves locales — 2026-09-30

- `TypedDraftEditor` expose les commandes pour les deux types de tickets
  correspondant au type réel du livrable. Chaque ligne conserve une identité de
  rendu locale ; retirer ou restaurer une ligne ne réattribue pas ses champs ou
  ses citations aux autres. Le focus suit l'ajout, le retrait et la restauration.
  Un enregistrement invalide l'annulation du retrait ; abandonner l'édition rend
  le focus au bouton « Nouvelle révision » et ne transmet aucune commande.
- `prepareEditedDraft` centralise les erreurs de contenu et les bornes UTF-8,
  normalise les lignes de critères avant de calculer le Markdown et conserve la
  liste globale des sources. Les ajouts commencent avec `source_ids=[]`.
  `ArtifactEditor` associe les erreurs aux champs, conserve le texte invalide et
  dirige le focus vers le premier champ signalé.
- 14 tests de composant supplémentaires couvrent les limites 1/29/30, les types
  non concernés, le retrait du milieu, la restauration sans écrasement d'une autre
  saisie, les citations, l'état occupé et les limites de contenu. Deux tests de
  page vérifient l'abandon/réouverture et le conflit avec une nouvelle version,
  en conservant le brouillon, la version de départ et la reprise idempotente.
- Suite web complète : **208 tests réussis dans 51 fichiers**. Lint et
  construction TypeScript/Vite réussis. Ces résultats sont des preuves locales
  du client ; ils ne prouvent pas une publication ni des comptes externes réels.
- `ticket-real-e2e/workflow.spec.ts` prépare désormais ses deux tickets produit
  et trois tickets techniques dans l'UI, avec contrôle du focus, des citations
  vides des ajouts, des sections et sources globales conservées, du Markdown et
  de l'ancienne version inchangée. Il prévoit ensuite le retrait de l'entrée du
  milieu en version 4, l'ouverture du lien historique version 3/index 2, puis la
  validation de la version 5 et l'avertissement T16 avant nouvelle publication.
  Cette dernière confirmation n'est pas envoyée : le scénario conserve ses
  cinq issues attendues. Mobile à 390 px et contrôle axe sont intégrés au parcours.

La recette TP-011 est ensuite passée sur API/DB locale : deux tickets produit,
trois techniques, cinq créations/reçus, retrait et lien historique conservé.
La capture mobile a été inspectée et axe ne trouve aucune violation après
correction de l'accès clavier à la région des sources. La revue indépendante
a également corrigé la conservation de critères multilignes déjà enregistrés.
[Preuves et limites](validation-2026-09-30.md).

La suite métier a révélé une perte de consigne lors du changement du type de
livrable. Les clés du formulaire séparent maintenant saisie locale et reçu de
commande ; un garde synchrone empêche aussi l'admission de l'ancien type avant
la fin de navigation, pour la génération et la création manuelle. Les sept
régressions supplémentaires portent la suite du snapshot à **216 tests web** ;
lint et construction passent. Quatorze tests de cette couture ont été relancés
par la contre-revue. La recette navigateur finale est conduite par le coordinateur.

Les quatre parcours métier passent sur ce snapshot corrigé en **47,2 s**. Les
18 fichiers de code/opérations sélectionnés dans l'index sont identiques à
ceux du snapshot qualifié ; les ajouts T18 simultanés sont exclus de ce jalon.

**Reste à qualifier avant clôture :** commit et CI du candidat exact. La
qualification de comptes réels reste séparée.
