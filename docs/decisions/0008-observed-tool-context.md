# Contexte externe observé, distinct des connaissances confirmées

Statut : accepté pour réalisation dans le mandat autonome ; T17 reste à
implémenter et qualifier selon [sa spécification](../../specs/existing-tool-context/spec.md).

Les équipes doivent pouvoir partir des documents et tickets qu’elles possèdent
déjà. AI Center conserve une référence à l’objet canonique et des observations
immuables de ce qui a été lu ; les agents, livrables, transmissions et constats
citent ces observations exactes. Une observation reste une donnée externe avec
ses dates et sa couverture, même si un livrable qui la cite est validé. Elle ne
devient pas implicitement une règle de l’entreprise ni une instruction pour un agent.

Deux entités dédiées représentent le rattachement Notion/Linear et son historique.
Réutiliser les publications aurait imposé un artefact et un envoi fictifs ; élargir
les références GitHub aurait mêlé deux familles d’autorisation et remis en cause
leurs invariants de dépôt et de commit. Les observations de publications existantes
restent dans leurs tables et rejoignent la même projection de contexte, sans copie
ni assouplissement de leurs contrôles de reçu. Cette séparation est une décision
de provenance, pas la création d’un second système de documentation.

Une vérification identique garde l’identité du contenu déjà observé. Un changement,
un retrait ou une indisponibilité empêche de le présenter comme contexte courant,
sans réécrire les citations historiques. L’état de connexion est vérifié au moment
de l’utilisation ; un changement d’autorité pendant une opération ne peut valider
son résultat avec les anciennes permissions. Les lectures restent ciblées,
explicites et bornées ; leur couverture ne signifie pas synchronisation exhaustive.

La société reste la frontière d’accès existante : le rattachement partage le
snapshot dans ce périmètre, sans prétendre reproduire les permissions individuelles
de l’outil distant. Une future confidentialité par projet ou une synchronisation
continue exigerait une nouvelle décision, de nouveaux contrôles et sa propre recette.
