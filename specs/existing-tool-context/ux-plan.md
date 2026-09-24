# Parcours UX — Sources des outils existants (T17)

> 2026-09-24 — conception seulement, sans modification du produit. La recette T16
> reste prioritaire. Ce plan suit la [spécification](spec.md), le [contrat API](api-contract.md)
> et la [revue du design](design-review.md). Les routes d'interface ci-dessous sont
> proposées ; les routes API et leurs discriminants restent ceux du contrat.

## 1. Résultat utilisateur et limites visibles

Un PM ajoute au projet une page Notion ou un ticket Linear déjà utilisé par son
équipe. Il voit ce qui a été lu, questionne son agent puis retrouve la même source
dans sa spécification et dans le contexte transmis au lead. Une modification
distante devient visible après une vérification demandée par une personne.

Le vocabulaire principal est « Sources », « Contenu lu », « Vérifier dans l'outil »,
« Lecture du… », « Retirer du contexte » et « Version utilisée ». Ne pas afficher
les termes ingestion, head, snapshot, rebind ou empreinte dans les actions métier.
« Source observée » ne signifie ni règle validée, ni preuve de travail réalisé.
Une source n'est jamais annoncée synchronisée en temps réel.

Le projet organise le contexte ; il n'ajoute pas une permission privée inexistante.
L'ajout partage le contenu enregistré avec les membres autorisés de la société
dans AI Center, même si leurs accès individuels à Notion ou Linear diffèrent.
L'espace société sert aux sources transverses ; il ne rassemble pas implicitement
les sources de tous les projets. Les parcours de contexte gardent leurs limites.

## 2. Entrées et écrans

| Écran proposé | Contenu et actions principales | Lecture locale |
| --- | --- | --- |
| `/projects/{projectId}/sources` | « Sources du projet », nom du projet, Ajouter une source, liste et filtres | `GET /api/projects/{id}/tool-sources` |
| `/sources` | « Sources de la société », nom de société, explication du contexte transverse ; même composant et projet société résolu par l'API existante | Même GET avec le vrai UUID du scope société |
| `/sources/{referenceId}` | Source rattachée, observation courante, couverture, vérification, historique, actions selon rôle | `GET /api/tool-sources/{id}` puis GET détail de l'observation exacte |
| `/source-observations/{observationId}` | Lecture historique exacte avec date/version, fraîcheur actuelle séparée, retour vers source | `GET /api/tool-source-observations/{id}` |
| `/publication-observations/{observationId}` | Même lecteur, origine « Créée depuis un livrable », lien vers publication et version source | `GET /api/publication-observations/{id}` |

Ajouter « Sources du projet » à la navigation du projet et « Sources » à l'espace
société. Les liens de citations et de graphe vont directement à une observation,
jamais à la page courante d'une référence. Valider UUID, type et portée retournés.
Un lien historique inconnu ou inaccessible produit une erreur explicite, sans
retour silencieux à une source actuelle ou à un autre projet.

La liste propose les filtres Tous les outils / Linear / Notion et Actives /
Retirées / Toutes. Afficher les compteurs serveur avec leurs libellés exacts,
une pagination par curseur et un état vide distinct d'une panne. Les 25 lignes
visibles ne représentent pas la totalité des sources. Ne pas afficher un filtre
ou une recherche serveur non couvert par le contrat.

Chaque ligne/carte montre titre, outil, portée, état de lecture, date de contenu
et dernière vérification. Les badges ont un texte, pas seulement une couleur.
Le statut métier d'un ticket Linear reste distinct de l'état de sa lecture.
Sur mobile, les cartes passent sur une colonne, les actions restent nommées et
les textes/URLs longs ne provoquent pas de débordement.

La liste T17 contient les références rattachées, comme le prévoit son endpoint.
Les sources issues de publications apparaissent d'abord dans leurs publications,
citations et dans le graphe ; ne pas fabriquer un second rattachement pour les
faire entrer dans cette liste. Une future liste unifiée nécessiterait un contrat
explicite avant d'annoncer un compteur regroupant toutes ces familles.

## 3. Connexion autorisée et partage explicite

Le propriétaire gère la capacité « Autoriser la lecture de documents et tickets
existants » dans Connexions. Elle est désactivée initialement, y compris sur les
anciennes connexions. Expliquer : « Les membres pouvant ajouter des sources
pourront partager les documents accessibles à cette connexion dans AI Center. »
L'activation ne lit rien ; une destination de publication ne vaut pas cette
autorisation. Un changement affiche son impact sur l'utilisation des sources.
La sauvegarde actuelle d'une connexion désactivée peut la réactiver : ne pas
soumettre un simple changement de capacité dans cet état. Passer par le flux
owner qui annonce explicitement la réactivation avant confirmation.

Le formulaire d'ajout de source n'a **aucun champ de clé**. Il choisit une
connexion serveur déjà autorisée ; nom d'équipe et outil sont suffisants.
Sans connexion utilisable, un propriétaire dispose du lien « Gérer les connexions » ;
un éditeur voit « Un propriétaire doit autoriser cette lecture dans Connexions ».
Ne pas envoyer automatiquement une demande au propriétaire ni à un outil tiers.

| Rôle actuel | Consultation | Ajout / vérification / retrait | Connexion / réactivation / changement de connexion |
| --- | --- | --- | --- |
| Lecteur | Oui, sous les droits courants | Non | Non |
| Éditeur | Oui | Oui, projet actif et connexion autorisée | Non |
| Propriétaire | Oui | Oui | Oui ; lecture obligatoire avant réactivation/changement |

L'UI prévient les actions indisponibles mais le serveur reste l'autorité.
Après archive du projet ou perte du rôle, les anciennes actions ne restent pas
actives grâce au cache. Un refus d'accès enlève le contenu affiché ; l'historique
n'est pas un moyen de conserver l'accès après révocation de la société.

Parcours « Ajouter une source » :

1. Afficher la société et le projet choisis, ou « Contexte général de la société ».
2. Choisir Linear ou Notion et la connexion autorisée. Ne proposer aucun nouveau
   connecteur, explorateur de compte, recherche distante ou import de dossier.
3. Saisir « Lien de la page ou du ticket » ; les identifiants officiellement admis
   restent une alternative dans l'aide. Refuser les liens non pris en charge sans
   les ouvrir. Exemples de documentation identifiés **[FICTIF]**.
4. Confirmer avec une case non précochée : « Je partage le contenu lu avec les
   membres autorisés de [Société] dans AI Center, dans le contexte [Projet]. »
   Changer de portée ou de connexion remet cette confirmation à zéro.
5. Cliquer « Lire et ajouter la source ». Pendant la lecture, conserver saisie et
   portée figées, afficher un état d'attente et le suivi de la demande, sans
   fausse progression en pourcentage ni promesse de couverture entière.

Le résumé précise que le document reste dans son outil et que la lecture ne
modifie rien à distance. `created` ouvre le contenu observé ; `existing` affiche
« Cette source est déjà rattachée » et la date réelle de sa lecture, sans prétendre
qu'elle vient d'être vérifiée. Une source retirée demande une réactivation owner ;
une autre connexion demande un changement explicite, jamais un second objet.
Une première lecture impossible n'affiche ni source vide ni succès artificiel.

## 4. Lire ce qui a effectivement été reçu

Le détail privilégie le titre et le contenu métier. Au-dessus du corps, afficher :

- **Contenu lu le…** : `observed_at`, date serveur de la capture affichée ;
- **Dernière vérification le…** : `freshness.last_checked_at` et son résultat,
  distincts de la capture ;
- **Modifié dans l'outil le…** : `remote_updated_at`, seulement si fourni ;
- **Lecture enregistrée n°…** : version locale, jamais « version Notion/Linear » ;
- **Origine** : outil, portée et « Ouvrir dans Notion/Linear » avec URL canonique sûre.

Les dates sont présentées en français avec heure/fuseau accessibles. Le détail
historique combine le snapshot immuable et la fraîcheur recalculée par le GET.
Il ne remplace pas une date manquante par aujourd'hui. UUID, hashes et révision de
connexion restent sous « Détails techniques » replié par défaut.

Le bloc « Ce que cette lecture couvre » est toujours accessible près du corps.
Une couverture complète signifie complète **pour le texte annoncé**, jamais pour
l'ensemble du compte ou tous les éléments d'une page. Traduire les omissions
connues : commentaires non lus, pièces jointes non lues, objets liés non lus,
propriétés non lues, contenus intégrés non lus, transcriptions non lues, blocs
non reconnus, texte coupé par l'outil, limite de texte atteinte, date distante
indisponible. Un code futur inconnu devient « Une limite de lecture est signalée »
avec son code dans les détails ; il ne disparaît pas du rendu.

| État serveur | Présentation | Action possible |
| --- | --- | --- |
| available + complete | « Texte lu dans le périmètre annoncé » et exclusions | Utiliser la source, vérifier explicitement |
| available + partial | « Lecture partielle » ; raisons visibles avant le corps | Consulter/citer les passages lus, ne pas conclure sur les parties absentes |
| unavailable + none | « Source absente ou inaccessible dans l'outil » ; pas d'ancien texte présenté comme courant | Ouvrir une lecture historique ou demander une nouvelle vérification autorisée |
| Dernière vérification failed | « La dernière vérification n'a pas abouti » ; ancien contenu et date conservés | Reprise selon le reçu/délai, sans affirmer un changement distant |
| historical | « Vous consultez la version utilisée à cette date » | Lien distinct « Voir la lecture actuelle » |
| detached | « Retirée du contexte actif » | Historique consultable ; owner peut réactiver explicitement |
| Connexion désactivée/capacité retirée/non réattestée | « Cette source ne peut plus alimenter le contexte actuel » et motif utile | Lien Connexions pour owner, nouvelle lecture après autorisation |
| Projet inactif | « Projet archivé : consultation uniquement » | Aucun ajout/refresh/rebind |

Afficher le texte retenu sans HTML actif, images distantes chargées automatiquement
ou liens exécutables. Réemployer les composants existants ; aucune dépendance de
rendu supplémentaire n'est nécessaire pour ce lot. Un lien présent dans le texte
ne déclenche jamais une lecture. Le bouton externe est distinct de « Voir la
version utilisée », car l'outil distant peut afficher un contenu différent.

## 5. Historique, vérification, retrait et changement d'accès

« Vérifier dans l'outil » est une commande explicite depuis la source courante.
Elle capture la révision et l'observation attendues. Une vérification inchangée
actualise sa date, conserve la version locale et annonce « Contenu inchangé ».
Un changement montre la nouvelle lecture et un lien vers la précédente ; un
comparateur enrichi n'est pas nécessaire au premier parcours. A → B → A reste
trois lectures. Deux vérifications concurrentes peuvent demander de recharger
l'état courant ; ne pas renvoyer automatiquement une ancienne commande.

L'historique affiche les versions réellement enregistrées, dates et couvertures,
paginées indépendamment de la liste des sources. Une vérification inchangée
n'ajoute pas de ligne de version fictive. En l'absence d'endpoint d'historique des
vérifications, afficher seulement la dernière vérification et le reçu consultable.
Ne pas promettre une chronologie détaillée de tous les contrôles.

Un changement annonce « Les éléments fondés sur l'ancienne lecture sont à
réexaminer ». Les liens vers dépendances et leur nombre ne sont affichés que s'ils
sont fournis par le graphe/API, jamais calculés à partir de la page visible.
Les textes de livrables et anciennes citations restent inchangés. Un pack périmé
propose sa préparation existante ; aucune régénération ni publication automatique.

« Retirer du contexte » demande confirmation du titre et de la portée :
« Cette source ne sera plus proposée aux agents dans ce contexte. Son historique
et les documents qui la citent restent consultables. Le document dans [outil]
reste inchangé. » Une confirmation produit `detach`, sans lecture distante.

Le propriétaire peut « Changer la connexion » ou « Réactiver la source ». Le
formulaire montre ancienne/nouvelle connexion, portée, confirmation de partage
et bouton « Vérifier et réactiver » / « Vérifier avec cette connexion ». `rebind`
conserve la même référence et ne réactive qu'après une lecture autorisée réussie.
Échec ou réponse incertaine : ne pas présenter l'ancien contenu comme réattesté.
Un changement de connexion peut conserver la même observation si le contenu est
inchangé ; l'UI n'invente pas une version pour expliquer cette réattestation.

## 6. Reçus et reprise sans action automatique

Réemployer le modèle durable T16 : conserver avant envoi clé, action, portée,
référence éventuelle et payload exact, séparés par acteur/société/projet/action/
référence. L'ajout garde son entrée initiale normalisée ; une reprise ne la remplace
pas par l'UUID résolu. Aucun corps lu, extrait, clé d'API ou réponse distante dans
ce stockage. Retirer credentials/query superflus d'une URL par validation locale,
ou refuser l'entrée ; aucune donnée secrète collée ne doit être persistée.

Remontage, navigation retour ou reconnexion : **GET du reçu seulement**. Le suivi
indique l'action et sa portée même si la page actuelle change ; le résultat ancien
n'est jamais injecté dans la société/projet courant. Le reçu terminé atteste le
résultat de l'action ; un GET source distinct indique son état actuel. Une mise
à jour de source ne réécrit pas le résultat enregistré de la commande.

| Reçu | Message et comportement |
| --- | --- |
| not_received | « Le serveur n'a pas enregistré cette demande. » Reprise explicite avec même clé/payload uniquement si `can_retry` ; pas d'expiration inventée par l'horloge locale. |
| processing | « Lecture en cours. » Reconsulter le reçu local avec une fréquence bornée ; pas de POST ni lecture fournisseur automatique. |
| interrupted / retryable | « La lecture peut être reprise. » Bouton explicite si `can_retry`, respect du `retry_after` serveur ; prévenir qu'une nouvelle tentative consomme le budget de lecture. |
| completed | Résultat enregistré et lien vers sa source/observation exacte ; rafraîchissement local de l'état actuel uniquement. |
| failed | Message serveur nettoyé, aucune reprise automatique ; recharger les paramètres avant une nouvelle demande explicite. |
| expired | « Cette demande a expiré. » L'ancienne clé n'est jamais réutilisée pour une nouvelle lecture ; consulter d'abord l'état local puis préparer explicitement une nouvelle demande. |
| GET impossible / réponse inconnue | « Impossible de vérifier le résultat pour le moment. » Garder la commande, proposer de relire le reçu ; pas de seconde demande concurrente. |

`can_retry` reste nécessaire mais le POST contrôle encore les droits, connexions
et révisions. Un délai affiché n'est pas une admission garantie. Changement
d'acteur/société : cacher/annuler immédiatement les lectures et résultats de
l'ancienne identité ; partitionner et purger selon la frontière d'identité
existante. Une réponse tardive ne redirige jamais le nouvel acteur.

## 7. Limites d'usage en langage courant

Le contrat borne une lecture à 45 secondes et 64 Kio de texte ; l'UI peut annoncer
« La lecture peut prendre quelques instants » et la limite dans l'aide. Elle ne
dit pas que toute attente de 45 secondes prouve un échec : consulter le reçu.

Les bornes actuelles sont 120 lectures par heure et trois simultanées par société,
une minute entre tentatives sur une même source et 200 sources actives par portée.
Elles sont des limites serveur, pas une jauge « restante » calculée par le client.
Les GET locaux, lectures d'historique et reçus ne consomment pas ce budget distant.
Les tentatives échouées et reprises réseau peuvent le consommer.

Messages proposés : « Plusieurs lectures sont déjà en cours », « Cette source
vient d'être vérifiée ; réessayez après [heure] », « Le budget de lecture est
temporairement atteint », « L'outil demande d'attendre jusqu'à [heure] » et
« Ce contexte contient déjà le maximum de sources actives ». Utiliser une date
uniquement lorsqu'elle est fournie par le serveur ; ne pas inventer de reset ni
ajouter une limite quotidienne. Le retrait d'une source garde son historique.

## 8. Citations, graphe et usages quotidiens

Réutiliser une carte de source commune dans conversation, génération, lecture
de livrable, pack/handoff, graphe et constat du steward. Elle montre titre, outil,
projet/société, lecture locale/date, couverture et « Source observée ». Son lien
conserve toujours `source_kind` + UUID exact. Un type inconnu affiche une limite
explicite ; ne pas le traiter par défaut comme une connaissance confirmée.

Le PM voit les passages effectivement fournis à l'agent ; les limites d'extrait
et de sélection (20 sources externes, 8 Kio par extrait, 64 Kio cumulés, dans le
budget total existant) ne sont pas celles du corps local conservé. Montrer les
compteurs d'exclusion fournis par le pipeline et « Ouvrir la lecture complète ».
Ne pas déduire que toutes les sources du projet ont été lues par l'agent.

Le lead ouvrant une spécification ou un pack retrouve la même observation, même
après une nouvelle lecture distante. Les tickets publiés remontent à leur entrée
et version de livrable, puis aux observations citées. Une citation partielle
conserve son avertissement dans l'aperçu de publication T16 ; valider le livrable
ne transforme pas ses sources externes en règles validées.

Pour une `publication_observation`, conserver l'origine et le job de publication,
sans faux bouton « Retirer la source rattachée » ni faux rattachement. Sa commande
de vérification reste celle de la publication existante, avec ses contrôles de
marqueur/destination. Aucun nouveau POST de création d'objet ne découle du lecteur.
Les reçus inchangés regroupés sémantiquement n'inventent pas une nouvelle lecture
de contenu ; une citation historique explicite conserve néanmoins son propre UUID.
Les métadonnées absentes d'anciens reçus sont indiquées comme indisponibles.

L'extension GitHub existante reste **à arbitrer après le vertical T17** : préparer
le rendu pour `external_reference_observation` (métadonnées de PR/commit/référence)
et `github_code_file_observation` (dépôt, commit vérifié, chemin, lignes et couverture).
Ne pas assimiler les premières à du code lu ; un fichier sélectionné ne représente
pas le dépôt entier. Garder hash et détails techniques repliés, chemin/lignes
lisibles et lien exact. Aucun nouvel import GitHub, connecteur ou crawl n'est ajouté
à ce lot ; ne pas afficher ces sources comme utilisables par les agents tant que
leurs discriminants, provenance et fraîcheur n'ont pas été raccordés et testés.

## 9. Découpage frontend et vérifications avant clôture

1. Figer les petits écarts de contrat ci-dessous ; fixtures JSON communes et
   types exhaustifs, adaptateur des liens exacts, pas de branche de repli ambiguë.
2. Livrer le vertical Linear : liste, ajout/partage, détail exact, citation PM,
   artefact sourcé, aperçu T16 et handoff ; reçus et frontières d'identité compris.
3. Ajouter Notion et observations de publications avec le même lecteur local ;
   vérifier omissions, historique, retrait/réactivation et changements d'accès.
4. Recette complète et revue indépendante ; aucune annonce de contexte connecté
   tant que la filiation PM → livrable → lead n'est pas démontrée. Le coordinateur
   possède SQL/DB, pipeline, export, effacement et la clôture de T17.

Tests significatifs :

- Composants : tous rôles, absence de connexion/capacité, consentement réinitialisé,
  corps partiel/absent, dates nulles, connexion révoquée, historique exact, refus
  d'accès sans fallback, omissions inconnues et type de source inconnu.
- Reprise : réponse perdue puis reload/GET seulement, `existing` sans nouvelle
  lecture, reprise même clé/payload, `not_received` après longue absence,
  expiration serveur, changement de société/acteur, réponse tardive ignorée,
  limite/délai sans boucle POST automatique.
- Navigateur : clavier/focus et annonce d'état, 390×844 sans débordement, retour
  depuis citation historique, zéro exception inattendue/overlay, axe ; captures
  viewport du formulaire partagé et du contenu avec couverture.
- Intégration gardée par root : API/base réelles, outils HTTP **[FICTIF]**, compteurs
  prouvant GET sans réseau fournisseur et zéro doublon ; changement distant,
  actualisation inchangée et autorité révoquée pendant la lecture. Un scénario
  fictif ne vaut pas preuve de connexion réelle aux outils de l'entreprise.

### Coutures d'assemblage avant code

- Contrat backend précisé le 24 septembre : `GET /api/work-tools` ajoute
  `connections[].allow_existing_reads`, `read_retry_after` dérivé des audits et
  `capabilities[].read_existing`. Réutiliser `POST /api/work-tools/connections`
  owner avec `{id,provider,name,expected_revision,allow_existing_reads}` ; omettre
  `api_key` conserve le secret chiffré. Omettre le nouveau booléen conserve sa
  valeur et la compatibilité des anciennes commandes. Pas de nouvelle page de secrets.
- Les erreurs d'admission conservent `retryable` et `retry_after` absolu :
  `source_read_in_progress`, `source_refresh_too_soon`,
  `source_read_quota_exceeded`, `remote_rate_limit` ; le plafond de sources
  `source_limit_reached` est non reprenable. Aucun compteur de capacité disponible
  n'est présumé. L'arrêt opérateur réemploie son état existant à vérifier lors du
  raccordement. Root confirme le contrat documentaire avant implémentation.
- Root et backend fixent les `app_path` du graphe vers les trois lecteurs locaux
  proposés. Les sources de publication requièrent leur endpoint détail exact
  déjà prévu au §7 du contrat ; son DTO ne permet pas d'inventer une référence.
- Toute liste de dépendances/compteur d'impact supplémentaire nécessite une
  donnée serveur existante vérifiée ou un amendement explicite. Le premier
  parcours peut se contenter du lien vers le graphe et de l'alerte de fraîcheur.

## 10. Amélioration bornée après la baseline T16 : édition des cinq formats

La lecture structurée des cinq formats est livrée dans T16. Prévoir ensuite
l'édition ergonomique de leurs listes sans modifier publication ou provenance :
ajout/suppression de tickets dans **Tickets produit** et **Tickets techniques**, et
édition des critères et points à clarifier déjà structurés. **Kickoff**,
**Spécification** et **Plan technique** conservent leurs sections narratives et
permettent la modification de leurs points à clarifier. Les sections métier
obligatoires restent définies par le contrat typé ; pas de suppression de section
requise ni de concepteur de schéma arbitraire.

Pour un ticket, proposer titre, description, critères et citations conservées,
une suppression explicite et une possibilité d'annulation avant enregistrement.
Respecter 1..30 tickets et les limites serveur ; une nouvelle entrée est un
brouillon sans citation inventée. Réordonner/supprimer produit une nouvelle
version : l'indice d'une ancienne version ne désigne jamais automatiquement la
même tâche. Les reçus T16 restent liés à leur version/index d'origine et leur
avertissement interversion garde sa confirmation explicite. Enregistrer une
révision ne valide ni ne publie ; aucune mutation d'une version historique.

Ce complément commence après la baseline qualifiée, avec contrat de saisie et
tests dédiés. La préparation de fixture TP-011 (une entrée générée puis révision
explicite à deux/trois entrées) ne constitue pas la livraison de cette amélioration.
