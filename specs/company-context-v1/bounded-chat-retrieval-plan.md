# Recherche de contexte bornée pour les conversations

Plan approuvé le 21 septembre 2026. Aucun service vectoriel ni fournisseur
supplémentaire. Le compilateur de ContextPack conserve ses exigences et son
échec explicite lorsqu'un ensemble obligatoire dépasse le budget.

Le chat générique utilise une projection `load_for_query` fondée sur la question
courante. Dans le scope société, elle recherche parmi les projets actifs
autorisés de la même société. Dans un projet, elle conserve les relations
confirmées à profondeur deux et le contexte société. PostgreSQL classe les
connaissances confirmées et derniers artefacts validés par pertinence textuelle
simple, puis pertinence de scope et récence, avec ordre final déterministe.

Les règles métier et contraintes société restent obligatoires. Elles ne sont
jamais écartées pour faire entrer une réponse dans le budget. Un excès de ces
obligations produit un refus explicite expliquant que le contexte doit être
réduit. Les autres sources sont limitées en nombre et taille avant l'appel IA.
Une société comportant plus de 160 connaissances ou 20 artefacts ne bloque
donc plus tout échange quand son contexte obligatoire tient dans le budget.

La projection conserve les UUID exacts de version, les projets sources et les
compteurs de graphe nécessaires à la vérification de concurrence. Elle indique
les limites et omissions de connaissances, artefacts et scopes au modèle et
dans la trace de réponse. La consigne ne permet jamais de présenter un résultat
partiel comme une couverture exhaustive. Les résumés de projets restent des
résumés, pas des connaissances confirmées.

Preuves : question ciblée retrouvant une règle interprojet depuis le scope
société ; société étrangère exclue ; au-delà de 160 connaissances et 20 artefacts
la question reçoit un contexte pertinent et explicitement borné ; obligations
trop nombreuses refusées ; snapshot périmé pendant l'appel rejeté ; compilateur
et contraintes des agents techniques inchangés.
