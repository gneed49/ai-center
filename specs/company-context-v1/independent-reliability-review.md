# Revue indépendante — fiabilité et frontières de données

Date : 2026-09-21. Base : `966ce9bbb597327e0ebbc3084836c6dedd759eca`.
Périmètre : candidat de travail, y compris nouveaux fichiers non suivis,
`automation`, bootstrap Auth, effacement société, récupération de contexte
conversationnel et filtre des imports GitHub. Références : `AGENTS.md`,
spécification Company Context V1 et plans de livraison actifs. Les axes
Standards et Spec ont été examinés séparément, localement ; les autres slots
d'agents étaient occupés. Aucune mutation de base ni requête fournisseur réelle
n'a été exécutée pour cette revue.

## Standards

Aucun défaut supplémentaire confirmé de journalisation de contenus, de requête
SQL construite depuis une entrée libre ou de contournement de droits dans ce
périmètre. Le diagnostic PostgreSQL détaillé dans `error.rs` était déjà identifié
par le responsable de livraison ; son remplacement par une classe/SQLSTATE ne
constitue pas un nouveau constat de cette revue.

## Spec

- **P2 — Un séparateur dans une valeur pouvait contourner le filtre de secrets.**
  Dans `apps/server/src/work_tools/code_privacy.rs:60`, la sélection initiale de
  `=` avant `:` ignorait une affectation YAML/JSON dont la valeur contenait du
  padding base64. Une sonde compilant le module exact avec des valeurs
  synthétiques a montré que `api_key: "RklDVElGX09OTFlfREVNTw=="` passait alors
  qu'une valeur alphanumérique simple était refusée. Cela contrevenait à la
  protection ciblée CC-043 avant persistance. Signalé au responsable : le
  candidat utilise désormais le premier séparateur et possède des cas de
  régression ; la recette finale appartient au lot de correction. Ce filtre
  reste une défense conservatrice et ne prouve jamais l'absence exhaustive de
  secrets dans un dépôt.

- **P2 — Une opération abandonnée peut bloquer durablement l'effacement.**
  `supabase/schemas/12_data_erasure.sql:116` à `:119` refuse tout `model_runs.running`
  et `idempotency_records.processing`, même lorsque leur commande n'a plus de
  bail valide. Après un crash puis retrait de l'auteur, aucune reprise applicative
  ne clôt ces lignes ; l'effacement exige en outre une société en pause et tous
  ses projets archivés. La procédure `docs/operations/company-data-retention.md:116`
  ne décrit pas comment solder cet abandon. Conséquence : le chemin CC-054
  peut rester inutilisable sur une société inactive. Prévoir une clôture
  opérateur explicite des seules opérations abandonnées, avec contrôle des
  traitements réellement actifs et invalidation des anciens baux, puis un test
  crash → retrait → clôture → effacement. Il ne faut pas supprimer la garde
  globale ni assimiler un appel distant ambigu à un appel jamais effectué.
  Constat statique transmis aux responsables ; aucun effacement exécuté ici.

Les contrôles relus utilisent l'appartenance effective pour l'exécution, une
génération pour la pause/reprise, une réservation de quota persistée, une liste
explicite pour le bootstrap privé et des projections de contexte bornées au
workspace. La récupération conversationnelle annonce ses omissions ; les règles
obligatoires qui dépassent le budget font échouer la requête. Aucun défaut
supplémentaire concret n'a été confirmé dans ces chemins. Cette conclusion
porte sur la lecture ciblée, pas sur une certification de sécurité exhaustive.

## Vérifications et suite

La sonde du filtre est locale et synthétique. Le responsable rapporte pendant
la recette intégrée : reprise conversationnelle 2/2, identité des messages,
outbox 2/2, publication 6/6 et automatismes 5/5 passés. Ces résultats sont
rapportés, et ne remplacent pas le résultat final du candidat.

Un ancien test de connexion attendait une nouvelle clé HTTP pour reprendre un
message en échec terminal ; cette attente contredit le contrat CC-050 actuel.
La fixture vérifie maintenant le refus sans nouveau run, la conservation du
message terminal, puis un nouvel envoi explicitement distinct après changement
de réglage. Les reprises éligibles gardent les deux identités initiales. Cette
adaptation de test a été autorisée après la photo de recette ; aucun service
métier n'a été modifié dans ce cadre.

Total : Standards 0 nouveau constat ; Spec 2 constats P2, dont le filtre est
pris en correction et la clôture opérateur d'abandons reste à traiter.

## Résolution des constats — 22 septembre 2026

Ajout de la coordination, distinct de la revue indépendante ci-dessus : les
correctifs sont enregistrés dans `9be97b6`. Le filtre utilise le premier
séparateur et ses tests de régression passent. La migration de maintenance
`16_abandoned_maintenance.sql` ferme uniquement les travaux abandonnés sous
pause, arrêt des workers et acquittement opérateur. Elle clôt les réservations,
baux et commandes sans requalifier un résultat distant ambigu en absence
d'effet. Le scénario `company_context/abandoned.rs` passe, ainsi que les
scénarios d'effacement et de rollback. Ces deux constats sont corrigés et
vérifiés localement ; aucune nouvelle revue indépendante n'est revendiquée.
