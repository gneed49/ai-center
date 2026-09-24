# Qualification du lot tickets distincts — 24 septembre 2026

Statut : recette locale PostgreSQL, Auth, TLS, restauration et navigateur tickets
réussie ; publication du jalon et CI distantes à suivre. CC-G1 reste ouvert pour T17 et le candidat final.
Les appels réels et l’exploitation restent CC-G2/G3/G4.

## Candidat et corrections de revue

Branche `feat/company-context-v1`, changements locaux au-dessus de `11bc60f`.
Le jalon précédent a huit contrôles distants distincts réussis : cinq dans
[Desktop CI](https://github.com/gneed49/ai-center/actions/runs/35934774241)
et trois dans [OCI Build](https://github.com/gneed49/ai-center/actions/runs/35934774264).
Ces contrôles ne qualifient pas les changements T16 non encore poussés.

La revue du frontend a identifié un blocage de reprise lorsque l’horloge locale
déclarait la commande trop ancienne mais que le serveur ne l’avait jamais reçue.
Le délai local a été retiré : le reçu serveur décide de la possibilité de reprise.
Le montage reste une lecture ; la personne reprend explicitement la même commande.

La revue du backend a demandé une vérification atomique de l’expiration de la
commande au moment du commit. Le renouvellement périodique et la génération du
bail ne suffisent pas à fermer le cas d’une expiration pendant une attente.
La correction utilise l’horloge réelle après acquisition du verrou ; sa régression
PostgreSQL a réussi. La lecture de connexion par un éditeur a également été
corrigée : sa révision est capturée sans demander les droits de modification
du propriétaire, puis recontrôlée avant tout envoi par le worker.

## Schéma et préservation

La migration déclarative ajoute un unique champ `source_ticket_index` à la table
des publications, une unicité par entrée et un contrôle de l’immutabilité de
la source et du contenu préparé. Aucun nouveau moteur de jobs ni table de lot.
Le catalogue conserve 58 tables ; l’empreinte attendue de maintenance devient
`88f3623848234264a58535727a2e97b2` après examen du changement de colonne.

Le premier contrôle de schéma a réussi avec une publication historique fictive :
ses identifiants, contenu, marqueur, reçu et observation sont inchangés, avec
indice `-1`. La reprise de la base historique et les vingt migrations ont ensuite réussi,
y compris la seconde migration d’inventaire et les procédures d’effacement.
Journal privé initial :
`.run/ticket-publication-schema-review.log`.

## Parcours à qualifier

- API/DB : N créations par N entrées, sélection, aperçu exact, capacités
  atomiques, concurrence, autorisations, commandes et anciennes publications.
- Graphe/export/purge : deux observations renvoient à leurs deux entrées
  exactes ; les liens historiques survivent à une révision et ne divulguent
  aucun secret ni bail.
- Navigateur : sélection, confirmation, résultats individuels, perte de réponse
  après commit et reprise par GET, puis version source exacte.
- Le parcours navigateur avec API réelle génère d’abord un ticket via le moteur
  déterministe. L’API de révision enrichit explicitement la fixture en deux
  tickets produit ou trois tickets techniques ; relecture, validation et
  publication se font dans l’interface. Cela ne mesure pas la capacité d’un
  modèle réel à respecter un nombre de tickets demandé.

Les données et fournisseurs de cette recette portent la mention **[FICTIF]**.
Les requêtes des outils visent exclusivement le serveur de contrat local ;
aucune connexion de production n’est invoquée.


## Résultats acquis

- Qualité initiale : 184 tests web, 168 unités Rust, trois contrats synthétiques,
  78 contrôles Python ; format, liens, lint/Clippy et builds réussis.
  Journal : `.run/ticket-publication-quality.log`.
- Recette PostgreSQL reprise après correction : **82 scénarios réussis**,
  dont 35 société, cinq artefacts, quatre équipe et 14 publications. Les tests
  d’invariants, quotas, concurrence, liens historiques, export et purge T16 passent.
  Le test navigateur Rust ignoré ici fait l’objet d’une phase distincte.
- TLS PostgreSQL : CA de confiance acceptée, CA inconnue et mauvais nom refusés.
  Sauvegarde/restauration : 58 tables et 138 politiques vérifiées.
- Auth API et navigateur à deux comptes réussis ; quatre parcours navigateur
  contre API/PostgreSQL réelles réussis en 38,2 secondes.
  Journal commun : `.run/ticket-publication-database-recheck.log` (code 0).
- Matrice navigateur avec doubles HTTP : 152/153 réussis ; le cas Firefox
  attendait un diagnostic Chromium pour la coupure réseau volontaire. Après
  correction strictement limitée à cette requête attendue, le scénario Firefox
  a réussi. Aucun autre diagnostic réseau n’est ignoré.
- Revue indépendante : aucun nouveau défaut P1/P2 identifié dans le périmètre
  relu ; [rapport et limites](review-2026-09-24.md).

La première recette a exposé des erreurs de fixtures (rôle autorisé à simuler
le worker, statut HTTP attendu, ordre des versions), puis un défaut réel de
focus historique : l’arrivée d’un reçu déplaçait le focus demandé par le lien
vers un ticket. Sa régression composant a été reproduite avant correction ;
24 tests ciblés passent après correction, sans assouplir l’assertion navigateur.
Les échecs précédents ne sont pas comptés comme preuves réussies.


## Parcours intégré tickets terminé

Le parcours TP-011 réussit en 16,9 secondes dans Chromium sur API/PostgreSQL
réelles : deux tickets produit → deux issues Linear, puis trois tickets techniques
→ trois issues GitHub. Le harness vérifie cinq créations HTTP distinctes, cinq
jobs réussis, cinq observations et les contenus/marqueurs individuels. La réponse
perdue après admission est retrouvée par reçu et rechargement sans duplication ;
le lien historique conserve version, ticket et focus. Journal privé :
`.run/ticket-publication-browser-final.log` ; exécuter la phase `ticket-browser`
sur une base jetable fraîche avant les autres fixtures d’intégration.

Le build web a été repris après le correctif de focus et réussit, ainsi que
le formatage des ajouts navigateur/CI. La phase navigateur intègre désormais
la CI et conserve ses traces en cas d’échec. Cette preuve reste synthétique
pour les fournisseurs et ne ferme aucune gate réelle ou d’exploitation.


## Contrôle distant et rendu mobile

Le commit `442860b` est poussé. Les huit contrôles du workflow de push ont réussi ;
le workflow PR a révélé un défaut de synchronisation du test mobile, après succès
du parcours 2 + 3 : la largeur était mesurée immédiatement après redimensionnement,
pendant une transition de mise en page. Le rejeu des réponses fictives de la trace
reproduit le dépassement immédiatement (10/15 puis 6/15 essais), et aucun dépassement
après deux frames de rendu (0/15 dans chaque série).

Le test attend désormais ces deux frames avant le même contrôle strict de largeur,
sans marge, délai arbitraire ni répétition de l’assertion. Le journal
`.run/ticket-viewport-diagnosis.md` conserve les mesures. La CI du correctif doit
confirmer le parcours ; son premier échec n’est pas effacé du registre des preuves.
