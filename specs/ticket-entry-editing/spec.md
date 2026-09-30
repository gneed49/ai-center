# Spécification — Ajouter et retirer des entrées de tickets

> Statut : qualifié localement sur API/DB, navigateur et revue ; CI du commit en attente
> Responsable : coordination Company Context V1
> Dernière mise à jour : 2026-09-30

## Intention et résultat attendu

Le PM et le lead peuvent déjà modifier les tickets d'un livrable structuré et
publier chaque entrée séparément. Ils doivent aussi pouvoir compléter ou réduire
ce lot depuis le même éditeur, sans modifier du JSON ni passer par l'API.

Exemple **[FICTIF]** : partir d'un brouillon d'un ticket produit, ajouter un second
ticket, remplir ses critères et enregistrer une nouvelle version. Après relecture
et validation, publier les deux entrées dans Linear. Retirer ensuite une entrée
prépare un autre brouillon ; l'ancienne version et ses issues restent intactes.

## Contexte et sources

- [Tickets distincts](../ticket-publication/spec.md), notamment TP-008/009/011/012,
  et [plan](../ticket-publication/plan.md), amélioration UX explicitement différée.
- [Parcours web](../../docs/product/07-company-context-web.md), limite actuelle
  de l'éditeur ; [ADR 0005](../../docs/decisions/0005-context-control-not-tool-replacement.md)
  et [ADR 0007](../../docs/decisions/0007-company-context-graph.md).
- Interfaces existantes : `TypedDraftEditor`, `ArtifactEditor`, `ArtifactPage` ;
  `artifacts/generation_contract.rs`, `validation.rs` et `save_draft` côté serveur.
- Aucun changement du modèle de ticket ou du contrat de publication. L'identité
  publiée reste **version immuable + index dans cette version**.

## Périmètre

**Inclus** : ajouter en fin de liste et retirer une entrée de `product_tickets`
ou `technical_tickets` au format `agent-artifact-v1`, dans une révision autorisée ;
conserver la saisie, les sources et les garanties d'enregistrement existantes.

**Exclus** : tableau de tâches, réordonnancement, duplication automatique, statut
d'avancement, nouveau générateur, rapprochement entre versions, sélection de
nouvelles sources, modification ou suppression d'une issue distante. Aucun bouton
d'ajout de ticket dans un kickoff, une spécification ou un plan technique.

## Parcours et invariants

1. **Ajouter** : « Ajouter un ticket » ajoute une entrée vide à la fin, place le
   focus dans son titre et annonce son numéro. Le nouveau ticket est une saisie
   humaine avec `source_ids=[]` ; il ne copie pas les citations d'un voisin et
   n'est pas présenté comme produit par l'agent ou fondé sur des sources citées.
   Les sources du document restent consultables, sans être attribuées à ce ticket.
2. **Compléter** : titre, description et critères d'acceptation restent les mêmes
   champs métier. Une entrée incomplète reste modifiable localement mais empêche
   l'enregistrement ; l'erreur indique le champ et le ticket concernés. Aucun
   texte fictif de remplacement n'est enregistré automatiquement.
3. **Retirer** : « Retirer le ticket N » ne modifie que la révision en cours.
   Une action locale « Annuler le retrait » restaure la dernière entrée retirée,
   à sa position avec tout son texte et ses citations, jusqu'au prochain ajout,
   retrait, enregistrement ou abandon de cette édition. Les modifications des
   autres entrées intervenues entre retrait et annulation restent conservées.
   Un libellé explique que le retrait ne supprime aucune issue externe.
4. **Bornes** : conserver **1 à 30 entrées**. L'ajout est désactivé à 30 avec une
   explication ; le retrait de la dernière entrée est désactivé. Ne pas limiter
   l'éditeur à 25 : cette limite concerne les publications simultanées, pas la
   taille du document. Les règles serveur ci-dessous restent canoniques.
5. **Sources** : les `source_ids` des entrées conservées et des sections ne
   changent pas. Retirer une entrée ne redistribue pas ses citations aux suivantes.
   La liste des sources de la version demeure conservée, y compris les captures
   d'origine ; elle n'est pas réduite silencieusement selon les entrées restantes.
   Réutiliser les contrôles d'accès et de validité des sources du serveur.
6. **Enregistrer** : même commande `save_draft`, même `expected_version_id`, même
   reprise idempotente. Le serveur crée un nouveau brouillon ; le Markdown est
   régénéré depuis les champs structurés et doit correspondre exactement. Ni
   validation ni publication automatique. Une erreur ou un conflit conserve la
   saisie ; ne pas appliquer les modifications à une nouvelle version sans
   résolution explicite du conflit.
7. **Historique** : après un retrait, les numéros de la nouvelle version peuvent
   changer. Les anciens reçus et liens continuent d'ouvrir leur ancienne version
   et leur ancien index. Une publication de la nouvelle version suit l'aperçu et
   la confirmation de créations supplémentaires T16, même si un titre est identique.
8. **Annuler l'édition** : abandonner ajout, retrait et modifications locales,
   sans POST. Réouvrir l'éditeur retrouve la version enregistrée. Pendant un
   enregistrement, ajout, retrait, annulation et soumission sont désactivés.
   Ce lot ne promet pas une sauvegarde des modifications après fermeture d'onglet.

### Limites de contenu existantes

| Élément | Contrat serveur à conserver |
| --- | --- |
| Titre d'entrée | Non vide, au plus 200 octets UTF-8. |
| Description | Non vide, au plus 8 000 octets UTF-8. |
| Critères | 1 à 20, chacun non vide et au plus 2 000 octets UTF-8. |
| Sources | Au plus 100 UUID uniques par section/entrée, tous autorisés ; au plus 98 citations distinctes dans le brouillon. |
| Ensemble | Brouillon sérialisé ≤ 100 000 octets ; structure ≤ 131 072 octets ; Markdown ≤ 262 144 octets ; au plus 100 sources de version. |

Les limites JavaScript en caractères ne remplacent pas les bornes UTF-8. Les
critères restent une liste « un par ligne » ; les lignes vides de mise en forme
peuvent être ignorées à la soumission, mais au moins un critère réel est requis.
Les sections, résumé et questions ouvertes conservent leurs limites existantes.

### Accessibilité et focus

Actions au clavier avec noms distincts ; annonce discrète du nombre d'entrées et
du retrait/restauration. Après retrait, focus sur le titre suivant, ou précédent
si le dernier est retiré ; après restauration, sur le titre restauré. Aucune
réutilisation de champ ne doit mélanger texte/citations de deux entrées. Annuler
l'édition rend le focus à son déclencheur. Les champs et actions restent lisibles
à 390 px sans défilement horizontal ; les erreurs sont associées aux champs.

## Critères d'acceptation et preuves

| ID | Résultat attendu | Preuve utile |
| --- | --- | --- |
| TEE-001 | Ajouter à 1 et 29 entrées, refuser 31 ; retirer à 2, conserver au moins 1. Pas d'action sur les autres types. | Tests de composants avec interaction réelle et bornes. |
| TEE-002 | Retirer l'entrée du milieu conserve exactement texte/citations des autres ; annuler restaure la bonne entrée sans écraser leurs modifications. | Fixture de trois entrées distinctes et sources distinctes. |
| TEE-003 | Nouvel ajout sans citations inventées ; sections, sources de version et entrée historique inchangées après sauvegarde. | Test de la requête d'édition et réponse API locale réelle. |
| TEE-004 | Saisie invalide, limites UTF-8/critères et erreur serveur expliquées sans perte du brouillon ; nouvelle version concurrente non écrasée. | Tests ciblés formulaire/page ; validations serveur existantes réutilisées. |
| TEE-005 | Enregistrement crée un brouillon conforme et du Markdown identique ; aucun appel IA, de validation ou de publication implicite. | Test de page puis parcours API locale. |
| TEE-006 | L'ancienne issue retrouve version/index exacts après retrait ; republier la nouvelle version exige l'avertissement T16. | Régression navigateur sur historique et aperçu. |
| TEE-007 | PM 1 → 2 tickets produit et lead 1 → 3 tickets techniques, ajouts faits dans l'UI puis publication explicite. | Adapter le parcours TP-011 **[FICTIF]** : supprimer sa préparation multi-ticket par API. |
| TEE-008 | Clavier, focus, annulation d'édition/retrait, état occupé et mobile fonctionnent ; annulation n'envoie rien. | Tests composants et navigateur du même parcours, contrôle d'accessibilité. |

## Limites et validation

L'éditeur propose l'ajout en fin, le retrait et sa restauration locale, avec des
identités de champs stables et des contrôles de contenu avant enregistrement.
Les tests de composants et de page couvrent les bornes, les sources conservées,
les erreurs, le conflit de version, l'annulation et le focus. Le parcours TP-011
utilise désormais l'éditeur pour ses entrées supplémentaires ; son exécution
contre l'API et la base locales reste à confirmer avant clôture de ce lot.

Aucune migration ni modification serveur. Les fixtures sont **[FICTIF]** ; les
simulations et l'API locale ne qualifient pas les comptes externes ni la production.
[Plan, état de réalisation et preuves](plan.md).
