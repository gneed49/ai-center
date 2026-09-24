# Connexions, publications et validité des sources — frontend, 21 septembre 2026

Réalisation CC-T06/07 et présentation des signaux CC-T08 selon les contrats
actifs de `connectors-plan.md` et des modèles serveur. Aucun compte externe
réel, clé réelle ou publication distante n'a été exercé par ce lot frontend.

## Comportements livrés

- `/settings/tools`, relié aux réglages IA et aux livrables : connexions société
  Notion/Linear/GitHub. Ajout/rotation, renommage, test explicite et désactivation
  réservés au propriétaire. La clé reste dans l'input et la pile de la requête ;
  l'identité de reprise ne conserve qu'une empreinte et un UUID. Aucun secret
  en query cache ou mutation cache. Le changement de société efface le formulaire.
- L'écran distingue clé enregistrée, test positif et droits sur la destination.
  Un test positif ne prétend pas valider les droits de publication.
- Les limites et l'activité affichées proviennent du serveur. Une absence de
  coût connu est indiquée, jamais remplacée par un coût nul.
- Publication d'une version validée et courante, après confirmation affichant
  le document, sa version, sa connexion et sa destination capturée. Une
  modification concurrente de la destination ne redirige pas la confirmation :
  le serveur compare explicitement les valeurs attendues et refuse le changement.
- Demande en attente, traitement, réussite, ambiguïté, échec, conflit externe,
  indisponibilité et annulation conservent leur sens distinct. La file est
  actualisée pendant le traitement ; aucune demande en attente n'est nommée
  « publiée ». Annulation uniquement avant traitement.
- Résultat ambigu : saisie d'un identifiant d'objet puis réconciliation en
  lecture, jamais relance aveugle de création. Une lecture ultérieure conserve
  les observations et ne remplace pas le contenu externe. Liens HTTPS limités
  aux hôtes des outils attendus.
- Les signaux affichent séparément l'état de leur traitement et l'actualité
  de leurs sources (`current`, `stale`, `unknown`). Une valeur absente n'est
  jamais affichée comme actuelle. Le type `context_gap` est un contexte à
  compléter, pas une contradiction démontrée.
- Un signal dépassé peut être écarté avec justification mais ne peut plus être
  accepté ou résolu à partir de ses anciennes sources. Les connaissances
  d'un autre scope ne sont pas proposées comme mutations locales. Les versions,
  identités et scopes d'origine restent consultables dans la provenance.

## Preuves

Suite web collective : **116 tests dans 29 fichiers**, tous réussis ; lint
sans avertissement. TypeScript/Vite ont passé après les interfaces et pages
connecteurs, publication et badges ; la suite complète a ensuite ajouté les
quatre scénarios d'actualité des sources. Les tests HTTP/UI restent synthétiques.

Les onze nouveaux tests connexions/publication vérifient les caches sans clé,
la reprise de commande, le changement d'identité, le test explicite, la
confirmation de destination figée, le refus draft/historique/lecteur,
l'annulation d'un job en attente, la réconciliation sans création et les liens
externes. Quatre tests supplémentaires couvrent les sources inconnues,
dépassées ou issues d'un autre projet, ainsi que le type contexte incomplet.

La recette navigateur contre APIs réelles, l'envoi distant sur comptes de test,
les coûts effectifs et la résilience opérationnelle restent des preuves
séparées. Aucune réussite locale n'est présentée comme déploiement ni
qualification réelle Notion/Linear/GitHub.
