# AI Center

AI Center relie le travail des équipes, les décisions et les résultats de leurs outils autour d'un contexte commun traçable.

## Langage

**Société** : espace partagé d'une organisation et frontière d'accès de ses membres. Deux sociétés ne partagent aucune connaissance implicitement.

**Projet** : initiative de la société rassemblant équipe, objectifs, agents, connaissances, livrables et références externes.

**Espace société** : contexte général des règles et décisions transverses, accessible aux membres autorisés et distinct des projets métier.

**Agent métier** : assistant doté d'un rôle explicite et d'un périmètre de connaissance, qui aide à comprendre, réfléchir et produire des propositions ou des livrables. Son rôle ne lui accorde pas de droits supplémentaires.

**Connaissance** : décision, règle, exigence, contrainte ou question confirmée, versionnée et reliée à son origine. Une proposition de l'IA reste distincte d'une connaissance confirmée.

**Relation** : lien explicite entre des éléments du contexte, portant un sens, un état et une provenance.

**Graphe de contexte** : ensemble des éléments et relations autorisés d'un projet ou de la société. Une vue globale ne supprime ni les périmètres ni les versions de ses sources.

**ContextPack** : instantané immuable du contexte sélectionné pour une tâche, avec les sources exactes, les raisons de sélection et les limites de sa construction.

**Livrable** : document ou ensemble de tickets produit à partir d'un contexte, conservé en versions et soumis à une décision explicite de publication.

**Entrée de ticket** : travail décrit dans une version précise d'un livrable, avec son titre, ses critères et ses sources. Sa position dans une nouvelle version ne prouve pas qu'il s'agit du même travail.

**Destination** : emplacement choisi dans AI Center ou dans un outil externe pour publier une version précise d'un livrable.

**Référence externe** : identité et adresse d'un objet qui reste détenu par un outil existant, accompagnées d'observations datées.

**Source rattachée** : référence externe explicitement partagée dans le contexte d'un projet ou de la société. Son retrait du contexte actif ne supprime ni l'objet dans son outil, ni les observations déjà citées.

**Observation** : contenu et limites de lecture d'une source externe à un instant donné, conservés pour pouvoir retrouver ce qu'un agent a effectivement reçu. Une observation n'est pas une connaissance confirmée.
_Éviter_ : vérité synchronisée, règle validée automatiquement.

**Vérification** : lecture explicite de l'état actuel d'une source, dont le résultat peut confirmer une observation inchangée, révéler un changement ou rester indisponible. La dernière vérification ne réécrit pas les anciennes citations.

**Publication** : demande explicite de créer un objet dans une destination à partir d'une version relue du livrable ou d'une entrée de ticket. Son reçu atteste la création observée, pas l'accomplissement du travail décrit.

**Preuve** : élément examiné et validé pour étayer une exigence. Un lien, une réponse IA ou l'absence de résultat de recherche ne constituent pas seuls une preuve.

**Constat de cohérence** : anomalie ou contradiction potentielle présentée avec ses sources et ses limites pour examen humain.

**Steward** : agent de cohérence qui observe les changements de manière asynchrone, rapproche les sources autorisées et présente les constats sans modifier les décisions à la place de l'équipe.
