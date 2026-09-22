# Lecture de code GitHub ciblée — complément CC-T07

Plan autorisé le 21 septembre 2026, avant implémentation. Aucun code externe
n'est exécuté et aucun accès réel n'est présumé.

Un membre éditeur choisit un projet, une connexion GitHub de sa société, un dépôt
`owner/repository`, un commit complet de 40 caractères hexadécimaux et au plus dix
chemins de fichiers. Le serveur appelle uniquement les endpoints officiels
GitHub ; ni URL arbitraire, branche mutable, chemin parent, archive complète ni
clone/exécution ne sont admis.

La vérification du commit précède les lectures. Les arbres Git sont parcourus
sans récursion globale, puis les blobs sont chargés par SHA et leur hash Git
est recalculé : la route Contents seule ne suffit pas car elle suit certains
liens symboliques. Les répertoires partagés sont mis en cache pendant le corpus. Chaque fichier demandé produit
une observation : texte UTF-8 réellement lu avec SHA de blob/hash local et lignes,
ou état insuffisant explicite (`missing`, `inaccessible`, `too_large`,
`binary`, `unsupported`, `unavailable`). Limites : 64 KiB par fichier, 256 KiB
de texte total et dix fichiers (huit segments par chemin) ; les réponses sont
bornées avant décodage. Au plus 32 requêtes et 60 secondes pour un corpus.
Symlinks, sous-modules, réponse tronquée ou identité inattendue ne deviennent pas
une preuve de code. Un fichier absent n'autorise pas à conclure que la fonction
n'existe dans aucun autre fichier.

Les corpus et fichiers sont immuables, attachés au projet et à la société ; les
FK typées et RLS empêchent l'accès intersociétés. Le contexte du steward peut
référencer un fichier observé exact et ses lignes. Les métadonnées, fichiers
illisibles et corpus partiels restent signalés comme insuffisants. Une alerte
parle du périmètre observé, jamais d'une conformité universelle du dépôt.

Livraison : module `work_tools/code.rs` + adapter borné, routes de création/liste/
lecture, `09_github_code.sql`, tests HTTP locaux de formats/limites et tests RLS
PostgreSQL isolés, puis sources du steward et interface de sélection/lecture.
La commande garde sa clé idempotente ; ses appels de lecture passent les quotas
outils. Aucun changement de schema06 déjà figé.

Critères : même demande rejouée ne crée pas deux corpus ; commit/path exacts
restent consultables après nouvelles lectures ; limites/binaire/manquant sont
visibles ; deux sociétés isolées ; aucune exécution ni secret exposé ; citations
du steward résolvent une observation et des lignes existantes.

Sources primaires consultées : [GitHub, contenu des dépôts](https://docs.github.com/en/rest/repos/contents#get-repository-content), [arbres Git](https://docs.github.com/en/rest/git/trees#get-a-tree) et [blobs Git](https://docs.github.com/en/rest/git/blobs#get-a-blob).
