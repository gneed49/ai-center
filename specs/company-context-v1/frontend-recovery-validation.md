# Reprise de conversation, archives et renommage — validation frontend

Livraison locale du 21 septembre 2026, mandat autonome Company Context V1.
Références : `session-recovery-plan.md`, `company-data-plan.md`, CC-005.

## Changements

- Le scénario réel du graphe suivait une route inexistante
  `/projects/:id/graph`. Il suit désormais le lien de navigation de l’application
  vers `/graph?project=:id` et sélectionne le nom accessible complet du nœud
  (titre, type, projet). La capture d’échec montrait une page 404, et non un
  défaut de mise à jour du graphe. La recette API réelle est relancée séparément
  avec la prochaine photographie du schéma par l’agent principal.
- La bibliothèque et le catalogue des conversations utilisent le snapshot
  autorisé pour un projet absent de la liste active de l’entreprise. Un projet
  archivé conserve ses livrables, versions, agents historiques et conversations
  en lecture seule ; une session technique archivée n’efface pas son transcript
  sous prétexte que son pack est dépassé.
- Les messages affichent « Vous » seulement si le serveur indique `is_own`.
  Un collègue est désigné par son nom serveur ; un auteur historique inconnu
  reste « Membre non identifié ».
- Les reçus de commandes propres au demandeur permettent une reprise explicite
  après rechargement, avec le contenu soumis exact, la clé de commande d’origine
  et un `client_message_id` qui peut être distinct. Une lecture ou actualisation
  ne lance jamais de génération. Le statut processing ou un délai déclenche
  uniquement la relecture périodique ; les accès sans droit de reprise, archives,
  échecs définitifs et commandes expirées n’ont pas d’action de reprise.
- Une reprise réussie préserve un autre brouillon saisi entre-temps. Une réponse
  retrouvée après perte du retour HTTP annule l’ambiguïté à l’écran, sans
  proposer une nouvelle génération.
- Le renommage capture le `updated_at` exact lors de l’ouverture du formulaire,
  sans conversion JavaScript de la date. Le même nom et la même base réutilisent
  l’identité de commande ; un conflit propose de recharger explicitement le nom
  actuel. L’objectif n’est pas modifié par cette action.
- Le code exclu pour forme de clé ou d’identifiant sensible reçoit un libellé
  français spécifique. Cette exclusion n’est pas présentée comme exhaustive.

## Vérifications

La suite fonctionnelle Chromium complète a passé **94/94 scénarios**, répartis
entre desktop et compact. Deux parcours supplémentaires inspectent chacun six
écrans (vue d’ensemble, entreprise, agents, livrable, outils, code) avec captures,
contrôle de débordement et axe WCAG 2A/2AA/2.1AA. Ces parcours ont découvert un
mauvais enfant dans la liste de définitions des coûts d’automatisation, corrigé
avant leur passage vert. Les captures sont synthétiques et identifiées comme
`[FICTIF]`, conservées localement sous `.run/company-visual/`.

Les tests unitaires vérifient les identités distinctes de reprise, les espaces
originaux, l’absence d’auto-envoi, l’attribution des auteurs, l’historique archivé,
les versions en lecture seule et la base exacte du renommage. Les résultats
finaux de tests/lint/build sont dans `.run/frontend-*-latest.log`.

Aucun appel modèle réel, aucune publication ni invitation par e-mail n’ont été
réalisés dans cette recette frontend. Le contrôle réel PostgreSQL/API, les
fournisseurs réels et le déploiement constituent des preuves distinctes.

Complément Firefox : 48/48 scénarios passent sur Firefox Desktop (45,9 s), y
compris les nouveaux parcours et les six écrans avec axe/débordement. Le navigateur
Playwright Firefox1538 a été installé dans le cache utilisateur, sans dépendance
système. Les captures Agents et Code ont été inspectées et les captures Firefox
conservées avec celles de Chromium. Aucun défaut spécifique Firefox constaté.
