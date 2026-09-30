# Qualification en cours — contexte des outils existants

Statut : **implémentation en cours**. Aucun critère ETC ni gate Company Context V1
n’est fermé par ce document. Les preuves ci-dessous concernent uniquement la base
jetable, le code local et les fournisseurs HTTP synthétiques.

## Schéma et sécurité

Deux tables nouvelles : références rattachées et observations immuables. Le catalogue
passe de 58 à 60 tables ; les huit colonnes ajoutées aux tables existantes portent
les FKs de provenance, les attestations et la capacité de lecture. Aucune colonne
existante supprimée. Les 294 FKs et les colonnes ont été relues ; empreinte revue :
`0e4ba7e13c76db41cfb4677a5d465c6c`.

La migration déclarative générée `20260924003601_existing_tool_context.sql` et
sa revue `20260924004410_tool_source_authority_review.sql` passent l’application
et la reprise historique. Le générateur legacy omet FORCE RLS, certaines ACL et
l’option security_invoker des vues : les déclarations ont été ajoutées explicitement
à la migration générée. Le guard runtime exige maintenant les privilèges exacts,
FORCE RLS et la vue soumise aux droits de l’appelant. La première exécution a été
refusée tant que l’inventaire ne connaissait pas les deux tables ; la reprise
corrigée a réussi. Journal privé `.run/tool-sources-schema-final.log`.

Les reçus de publication anciens gardent leur contenu et leur identité ; leur
attestation reste nulle. Une vérification explicite peut réattester un groupe
inchangé sans modifier son observation historique. A→B→A donne trois groupes.
L’unicité, les triggers différés de head et les liens de scope doivent encore
passer les nouveaux tests métier ; l’application d’une migration ne suffit pas.

## Preuves logicielles intermédiaires

- Lecteur entrant isolé : 12 tests purs et HTTP loopback, avec Clippy strict.
  Aucun vrai compte Notion/Linear, aucune nouvelle dépendance.
- Premier raccordement du pipeline : 186 unités Rust réussies, dix tests nécessitant
  la base ignorés. Ce passage précède la fin des services transactionnels et ne
  qualifie pas encore le candidat intégré.
- Graphe, export, effacement gouverné, reprise des versions et autorité pendant
  une analyse du steward sont raccordés localement ; tests PostgreSQL en préparation.

## Vérifications restant à exécuter

Tests services/API et SQL sous le rôle runtime, exactitude des citations en chat,
artefact et pack, changement de source et de connexion pendant la génération,
quotas et cooldown, réponse perdue et reçus, provenance externe dans le graphe et
les exports, purge sans dommage aux autres projets, navigateur avec API réelle,
revue indépendante, recette globale et CI du commit final.

Les observations GitHub déjà présentes restent un complément explicite à raccorder
au contexte chat/artefact après ce lot ; leur présence dans le graphe/steward ne
constitue pas cette preuve. Les comptes fournisseurs réels et le déploiement restent
soumis aux choix de budget et de destination encore attendus.
