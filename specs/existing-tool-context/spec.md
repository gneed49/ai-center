# Spécification — Contexte issu des outils existants

> Statut : conception autorisée ; aucune implémentation dans ce lot documentaire
> Responsable : coordination Company Context V1
> Dernière mise à jour : 2026-09-23
> Séquence : après le lot [Tickets distincts](../ticket-publication/spec.md)

## Intention

Une entreprise possède déjà des documents Notion et des tickets Linear. Le PM
doit pouvoir les rattacher à son projet, les interroger et préparer ses livrables
à partir de leur contenu effectivement observé. Recréer ces objets dans AI Center
ou les copier manuellement ferait perdre leur identité, leur date de lecture et
leur actualisation. Ce manque est un P1 du parcours utilisateur, même si les
premiers critères CC-025 à CC-027 ne décrivaient que les publications sortantes.

## Résultat attendu

Dans un projet, « Ajouter une source » permet de choisir une connexion autorisée
et de coller le lien d'une page Notion ou d'une issue Linear existante. Une lecture
explicite conserve une observation datée et son lien canonique. L'agent PM peut
répondre à partir de cette observation et la citer ; une spécification ou un lot
de tickets généré conserve cette source exacte. Le lead retrouve cette filiation
dans le contexte transmis. Le graphe et le steward utilisent les mêmes identités.

Exemple **[FICTIF]** : rattacher une page « Expiration des crédits » et l'issue
`PROD-42`, préexistantes et sans marqueur AI Center ; poser une question au PM,
produire une spécification sourcée, puis transmettre au lead. Après modification
du ticket dans Linear, une actualisation montre ce qui a changé et les documents
qui reposent sur l'ancienne observation. Aucune écriture distante ne découle du
rattachement, d'une réponse de l'agent ou d'une détection du steward.

## Contexte et sources

- [Company Context V1](../company-context-v1/spec.md), CC-U03/U04/U06, CC-012,
  CC-020/023/025/027/029 et CC-031/054 ; [registre](../company-context-v1/tickets.md).
- [Référence avant duplication](../../docs/decisions/0005-context-control-not-tool-replacement.md)
  et [graphe société](../../docs/decisions/0007-company-context-graph.md).
- [Connecteurs actuels](../company-context-v1/connectors-plan.md) et
  [plan technique associé](plan.md), avec preuves du code et sources officielles.
- Ce document amende une omission du contrat initial. Il ne transforme pas
  l'exclusion de synchronisation exhaustive en promesse de couverture complète.

## Périmètre

### Inclus

- Rattachement volontaire d'une page Notion ou d'une issue Linear unique au scope
  projet ou société autorisé, depuis une connexion serveur existante.
- Titre et contenu textuel de la page ; titre, description et état de l'issue.
  Les omissions du périmètre et de la lecture sont affichées.
- Observation immuable, lecture de son historique, actualisation explicite,
  retrait du contexte actif sans effacement de l'historique.
- Sources exactes utilisables par conversation, génération d'artefacts et
  ContextPack/handoff, avec droits, budgets et statut de confiance conservés.
- Filiation dans le graphe, événements du steward existant, export et effacement.
- Réemploi des observations de publications Notion/Linear déjà présentes comme
  sources lisibles par les agents, sans fabriquer de second rattachement.

### Exclu

- Synchronisation globale, recherche/listing d'un compte, import massif, crawl,
  webhooks, polling, lecture autonome déclenchée par un agent, nouveau fournisseur.
- Commentaires, pièces jointes, images, OCR, transcriptions de réunions, propriétés
  complexes, relations de bases Notion, tickets liés, sous-tâches ou contenu des
  liens incorporés. Le contenu d'un lien ne devient pas celui de la page observée.
- Copie des permissions distantes individuelles dans AI Center : la connexion
  est une autorisation serveur de société, et l'objet rattaché est partagé avec
  les personnes autorisées au scope AI Center choisi. Cette conséquence est visible.
- Toute modification/suppression de l'objet distant ou remplacement des outils.

## Modèle et invariants

1. **Source rattachée** : identité stable fournisseur + identifiant canonique
   dans un scope autorisé. Elle conserve sa connexion et sa révision, son URL,
   son état actif/retiré, son auteur et sa dernière vérification. Une même source
   peut être rattachée explicitement à deux projets ; cela ne crée aucun partage
   implicite entre ces projets ou entre sociétés.
2. **Observation exacte** : identifiant immuable, numéro de version locale,
   identité et portée, date de lecture serveur, date distante si disponible,
   titre, projection textuelle, empreinte, couverture et raisons d'omission.
   Le numéro local n'est jamais présenté comme une révision native Notion/Linear.
   Les dates absentes restent absentes ; elles ne sont pas estimées.
3. **Confiance** : `observed_external`, distinct de `confirmed`. Le fournisseur
   est canonique pour son objet, pas pour les règles de toute l'entreprise. Une
   citation prouve ce qui a été lu, pas sa vérité ni sa conformité au code.
4. **Fraîcheur** : « observé le… », « vérifié le… » et « modifié dans l'outil le… »
   sont séparés. Aucun contenu n'est déclaré à jour en temps réel entre deux
   lectures. Une observation identique réutilise la version et enregistre un
   reçu de vérification ; elle ne relance pas inutilement le steward.
5. **Historique** : un changement de projection, de couverture ou de disponibilité
   crée une nouvelle observation. Une réponse identique à une ancienne version
   après un changement crée également une nouvelle version : seul le dernier
   état est dédupliqué. Les anciennes citations ne sont jamais réécrites.
6. **Sources sélectionnables** : observations actives lisibles, complètes ou
   partielles avec limites explicites. Une observation indisponible, retirée ou
   dont la connexion est révoquée n'est pas réintroduite comme contexte courant.
   Son historique reste consultable sous les droits AI Center existants jusqu'à
   l'effacement gouverné ; un membre révoqué n'y conserve aucun accès.
7. **Séparation des contrats** : importer une source ne crée aucun artefact ou job
   de publication fictif. Les lecteurs et marqueurs stricts de publication ne
   sont pas assouplis. Aucun contenu entrant ne commande une écriture sortante.

## Parcours et comportements

### Rattacher et comprendre la couverture

Le propriétaire active explicitement la capacité de lecture des objets existants
sur une connexion. La capacité est désactivée pour les anciennes connexions à la
migration. Un owner/editor choisit le scope et confirme le partage du contenu
avec ses lecteurs. Le formulaire accepte un lien officiel ou un identifiant
documenté ; il refuse une URL arbitraire avant tout accès réseau. Le serveur
utilise uniquement les APIs officielles, sans suivre le lien collé ni ses redirections.

La source apparaît avec titre, fournisseur, lien canonique, date, texte observé
et couverture. Aucun marqueur AI Center n'est requis. Une page dont le parent
est une autre page, le workspace ou une source de données peut être lue sans
importer son parent ; le titre est identifié par son type, pas par un nom de
propriété imposé. Une issue est identifiée par son UUID canonique après résolution.

La limite initiale est une projection textuelle de 64 Kio par objet et une lecture
de 45 secondes au total, au plus trois requêtes Notion ou une requête Linear.
La limite réseau par réponse reste 256 Kio. Aucun parcours paginé n'est suivi :
les éléments non récupérés, blocs inconnus et contenu tronqué sont signalés,
jamais considérés comme absents. Un dépassement empêchant un décodage fiable
donne une lecture indisponible, pas une observation artificiellement complète.

### Questionner et produire

Le contexte de l'agent contient un bloc de données observées avec source exacte,
scope, URL, dates, couverture et extrait. Il ne place pas le texte distant dans
les instructions système, les règles obligatoires ou une autorisation d'outil.
Une instruction malveillante incluse dans le document demeure du contenu cité.
Le modèle ne peut ni importer une URL supplémentaire ni publier à cause de ce texte.

La sélection utilise les mêmes portées autorisées que les autres sources : scope
local, règles société et projets explicitement liés. Une session globale n'agrège
que ce que son acteur peut lire. Les exclusions par limites et les extraits sont
quantifiés. Une source partielle permet de citer son passage observé ; elle ne
permet pas de conclure que rien n'existe dans les parties non lues.

Les sources d'un artefact et d'un ContextPack conservent l'observation exacte et
son empreinte ; le lien mène au snapshot historique, même après actualisation.
La filiation remonte du ticket publié à la version d'artefact, puis à cette
observation. Valider l'artefact n'élève pas la source externe au statut de règle.

### Actualiser, retirer et reprendre

L'actualisation relit exclusivement l'objet rattaché. Elle conserve l'historique,
compare la projection normalisée et affiche `inchangé`, `modifié`, `partiel` ou
`indisponible`. Un 403/404 ne prouve pas la suppression : le message indique une
absence ou un accès impossible, sans inventer la cause. Un timeout/429/5xx est
un échec de vérification temporaire, distinct d'un changement du document.

Un changement utile invalide les packs et signale les artefacts dépendants ;
leur texte et leurs citations restent immuables. La génération en cours ne peut
commettre un résultat affirmant un contexte courant si la source est devenue
inaccessible ou a changé. Une lecture inchangée ne périme pas les dépendances.

Retirer une source arrête son inclusion et rend ses dépendances à réexaminer,
sans supprimer le document distant. Rotation/révocation de connexion ou changement
de droits pendant un appel empêchent d'enregistrer le contenu avec une autorité
périmée. Une reconnexion exige une nouvelle lecture autorisée avant réutilisation.
Une réponse HTTP perdue se récupère par reçu local ; le GET ne relance ni outil
ni modèle. Une répétition de la même commande ne crée pas deux observations.

## Critères d'acceptation

| ID | Comportement attendu | Preuve ciblée |
| --- | --- | --- |
| ETC-001 | Rattacher une page Notion et une issue Linear préexistantes, sans marqueur ni artefact préalable ; titre, contenu et identité corrects. | Fixtures HTTP puis PostgreSQL ; pas de mutation distante. |
| ETC-002 | Page Notion parent page/workspace/data source et titre renommé ; Linear UUID ou URL avec identifiant lisible résolu vers le même UUID. | Contrats de lecture distincts ; parsers de publication inchangés. |
| ETC-003 | URL arbitraire, hôte ressemblant, redirection, mauvais UUID, identité distante incohérente ou projection invalide sont refusés. | Tests HTTP et validation avant réseau. |
| ETC-004 | Bornes taille/durée/appels, pagination non suivie, blocs inconnus et parties exclues produisent une couverture honnête. | Fixtures complètes/partielles/hors taille ; compteur d'appels. |
| ETC-005 | Déduplication d'identité, commande perdue/rejouée et actualisations concurrentes ne dupliquent pas la source et n'inversent pas ses versions. | Tests PostgreSQL concurrents et reçu après rechargement. |
| ETC-006 | Droits société/projet/acteur/connexion vérifiés avant et après lecture ; lecteur interdit en mutation ; révocation en vol ne persiste pas le texte. | Tests de RLS et barrières HTTP ; zéro contenu après révocation. |
| ETC-007 | Le PM répond en citant une observation exacte, le livrable généré conserve cette source, le lead la retrouve dans son pack. | Scénario synthétique complet agents → artefact → handoff. |
| ETC-008 | Contenu malveillant, source partielle et source indisponible ne deviennent ni instructions, ni règles confirmées, ni preuve d'absence. | Tests de projection, validation des citations et évaluations synthétiques d'injection. |
| ETC-009 | Changement/indisponibilité/retrait invalident les dépendances incluses et déclenchent le steward borné ; inchangé ne déclenche aucun nouvel appel IA. | Tests de fraîcheur, filiation, frontier et compteur fournisseur nul pour inchangé. |
| ETC-010 | Une observation de publication Notion/Linear existante est également interrogeable et citable, avec sa filiation sortante préservée. | Fixture après publication puis modification distante relue. |
| ETC-011 | UI affiche source/date/couverture/historique et reprise par reçu ; aucun polling distant ni envoi automatique lors d'un montage. | Tests composants et parcours navigateur [FICTIF]. |
| ETC-012 | Exports incluent références/observations/provenance ; effacement et inventaires, rôles runtime, maintenance et quotas couvrent les nouvelles données. | Recette isolée, vérificateurs de schéma et non-régression publications/GitHub. |

## Limites et validation

Le code n'est pas livré par ces documents. Les contrôles locaux avec fixtures
n'attestent pas une connexion réelle. Une qualification ultérieure lit seulement
des objets de test explicitement autorisés, après disponibilité des connexions.
CC-G1 reste ouvert jusqu'aux preuves logicielles ; G2/G3/G4 restent distincts.

Les sources officielles consultées et les décisions d'implémentation sont dans
le [plan](plan.md). La mise à jour du produit consolidé et une décision durable
sur le modèle sont prévues lors de la réalisation, après la revue du contrat.
