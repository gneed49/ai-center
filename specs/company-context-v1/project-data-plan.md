# Portabilité et effacement d'un projet — complément CC-054

Plan accepté par la coordination le 21 septembre dans le cadre de l’autorisation autonome du propriétaire ; recette en cours le 22 septembre. L'export et la purge d'une société entière ne
constituent pas une preuve d'effacement d'un projet isolé. Aucun effacement de
données utilisateur réelles n'est prévu dans cette session.

## Export projet

Ajouter `GET /api/projects/{id}/export`, propriétaire uniquement, y compris
pour un projet archivé. Le conteneur société n'est pas un projet exportable.
La réponse en pièce jointe sans cache distingue `data` (historique possédé par
le projet) et `referenced_sources` (versions exactes d'autres scopes citées dans
ses packs, artefacts ou analyses), avec UUID du projet source et provenance.

Une liste blanche de colonnes conserve les mêmes exclusions que l'export
société : aucun secret, configuration de connexion, jeton, lease de commande,
contenu de commande de reprise ou invitation. Le rôle propriétaire est vérifié
dans la transaction de lecture cohérente et RLS reste actif. Aucun parcours
récursif implicite d'une société entière : chaque source citée est exportée à
sa version exacte ; les sources qui citent à leur tour d'autres documents sont
identifiées comme références sans prétendre exporter leur fermeture complète.

Les relations transverses donnent leurs extrémités et leurs scopes, sans
inclure les conversations des projets voisins. Les réglages hérités de société
peuvent être décrits par une projection publique, jamais par la configuration
de connexion. Les identités d'auteur déjà présentes restent explicites ; la
fonction n'est pas une anonymisation.

Le budget cumulé reste 25 000 lignes et 32 Mio. Un dépassement refuse l'export
entier ; aucun fichier partiel marqué complet. Tests : versions locales et
externes exactes, source ancienne conservée, aucun contenu voisin non cité,
projet étranger refusé, viewer refusé, archive lisible, limites explicites.

## Purge opérateur d'un projet sans dépendance externe

Une nouvelle migration additive et des fonctions `SECURITY INVOKER` privées
sont nécessaires après la recette courante. Ne pas étendre silencieusement
la fonction société ni ses anciennes migrations. Le script opérateur aura un
mode projet séparé, UUID société + UUID projet + aperçu puis confirmation du
reçu. Le runtime et l'API ne reçoivent aucun droit d'effacement.

L'aperçu résout uniquement un projet métier du workspace exact. Il vérifie
l'inventaire des tables et clés étrangères contre une liste revue, puis
détermine les lignes possédées par le projet, directement ou via leur parent
(par exemple les observations de publication ou sources d'artefact). Les
lignes société partagées, membres, connexions et réglages globaux restent
exclues. L'ordre de suppression est explicite et respecte les contraintes
immédiates ; les cycles de versions utilisent seulement les contraintes déjà
différables.

Avant toute suppression, refuser si une ligne appartenant à un autre projet
référence la cible ou l'un de ses objets : graphe dans les deux sens, sources
de packs et leurs estampilles, sources de livrables et artefacts, analyses
Steward et résolutions, observations et références typées. Inspecter aussi les
références polymorphes prévues par le modèle ; une nouvelle table ou relation
non classée impose une revue. Le reçu explique les dépendances bloquantes par
type et nombre. La fonction ne supprime ni ne réécrit les sources historiques
d'un autre projet pour forcer le passage. Les textes copiés librement hors des
liens canoniques ne sont pas détectables exhaustivement : ce périmètre ne
doit pas être présenté comme effacement de toute mention textuelle.

Exécution : société en pause, projet archivé, aucun travail actif susceptible
d'écrire dans le périmètre. Prendre les verrous de maintenance bornés dans un
ordre stable, recalculer inventaire, dépendances et comptes, revalider le reçu,
puis supprimer dans une transaction unique. Étendre l'exception des triggers
immutables au rôle opérateur direct avec workspace, projet et transaction
exacts ; une exception basée uniquement sur le workspace serait trop large
pour ce nouveau mode. Le test doit prouver qu'un effacement hors projet est
refusé même dans cette transaction. Le reçu final conserve seulement les
identifiants, comptes et exclusions, hors des données supprimées.

Preuves sur fixtures `[FICTIF]` : deux projets d'une même société et une
seconde société ; purge de la cible sans changement logique des autres,
refus d'une dépendance dans chaque catégorie ci-dessus, refus de GUC forgées
par le runtime, rollback sans suppression partielle, dérive de schéma refusée,
immutabilité normale intacte. Auth, sauvegardes et objets distants restent
soumis à leurs procédures séparées.
