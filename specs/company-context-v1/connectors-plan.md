# Connecteurs et publications durables — CC-T06 / CC-T07

> Réalisation autorisée le 21 septembre 2026. État : en cours, aucune connexion
> réelle configurée et aucune écriture distante exécutée par ce lot à ce stade.

## Contrat borné

Le propriétaire de la société enregistre dans l'application une connexion
Notion, Linear ou GitHub. La clé est chiffrée côté serveur avec le mécanisme
existant ; les réponses et journaux ne la contiennent jamais. Les destinations
par type/société/projet restent celles de `04_artifacts.sql`.

Une personne autorisée publie explicitement une version validée et courante.
Le serveur capture sa version, son hash, sa connexion et sa destination dans
un job persistant. Les APIs officielles créent une page Notion ou une issue
Linear/GitHub ; AI Center conserve la référence canonique et une observation
datée. Une nouvelle version demande une nouvelle publication explicite. La
première capacité livrée ne promet pas de mise à jour automatique d'un objet
externe modifié par une autre personne.

Chaque objet reçoit une référence de publication AI Center unique. Le
traitement externe est hors transaction SQL, borné en délai/taille et sans
redirection. Un job révoqué ou dont les permissions ne sont plus valides ne
commence pas un appel externe.

## États et reprises

- `queued` : demande durable, aucun appel effectué.
- `processing` : bail détenu, tentative enregistrée avant l'appel.
- `succeeded` : identité distante validée et observation enregistrée.
- `failed` : refus explicite sans création confirmée ; raison nettoyée.
- `needs_review` : résultat ambigu, réponse perdue ou crash après prise du job.
  Aucune répétition aveugle de création, notamment pour Notion.
- `conflict` / `unavailable` : une observation ultérieure révèle une modification
  ou une perte d'accès ; l'historique réussi reste conservé.
- `cancelled` : acteur/connexion retiré avant l'appel ou annulation autorisée.

La reprise d'un résultat ambigu est une réconciliation de l'objet distant,
avec référence canonique, destination et marqueur de la publication vérifiés.
Une absence dans une recherche partielle ne prouve pas l'absence de création.
Une réconciliation infructueuse laisse l'état ambigu ; elle ne crée rien.

## Découpage et preuves

1. Connexions de société, contrats HTTP, chiffrement et engagements de requête
   sans secret ; autorisations et révocation.
2. Jobs/reçus/observations, idempotence locale, claim/bail et worker reprenable
   après redémarrage, contrôle acteur/connexion au moment de l'appel.
3. Adapters officiels avec tests HTTP locaux : succès, refus, erreurs GraphQL
   dans HTTP 200, timeout, redirection, réponse invalide et taille excessive.
4. Publication de version immuable, export canonique et réconciliation/refresh
   sans écrasement ; contrats et PostgreSQL isolé.
5. Intégration routes/worker et frontend ; qualification réelle distincte,
   seulement sur comptes/destinations de test autorisés.

Fichiers détenus par ce lot : `apps/server/src/work_tools/`,
`apps/server/src/routes/work_tools.rs`, `supabase/schemas/06_work_tools.sql`,
`apps/server/tests/work_tools.rs`. Le coordinateur intègre routes, worker,
migration générée et privilèges runtime centraux.

Un claim serveur éventuellement privilégié est limité à la file et à ses
baux, réservé au rôle runtime et placé dans le schéma privé. Il ne donne aucun
accès public ni accès aux clés dans le navigateur. Les opérations métier et
les lectures des connexions restent filtrées par société et appartenance.

## Sources à vérifier pendant l'implémentation

- [Notion : création d'une page](https://developers.notion.com/reference/post-page)
  et [lecture Markdown](https://developers.notion.com/reference/retrieve-page-markdown).
- [Linear : API GraphQL](https://linear.app/developers/graphql).
- [GitHub : issues](https://docs.github.com/en/rest/issues/issues).

La réussite des doubles HTTP ou PostgreSQL ne qualifie pas une intégration
réelle. Les ressources absentes et capacités limitées restent visibles ; aucun
connecteur n'est présenté comme connecté ou publication comme réussie avant
le résultat correspondant.
