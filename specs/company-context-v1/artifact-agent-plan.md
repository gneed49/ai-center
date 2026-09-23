# CC-010/011/012 — Production d’artefacts par les agents

## Écart corrigé

Les cinq types de documents existent, mais leur création est manuelle et les
livrables structurés FeatureBrief/TechnicalPlan ne rejoignent pas la bibliothèque
publiable. La copie d’une réponse de chat ne constitue pas une génération typée.

## Livraison

1. Commande explicite de génération depuis une conversation autorisée. Le profil
   produit prépare kickoff, spécification et tickets produit ; les profils
   technique/développeur préparent plan et tickets techniques avec un contexte
   transmis courant. Les autres profils conservent leur contrat déclaré.
2. Sortie structurée avec sections/tickets imposés par le type, citations exactes,
   incertitudes explicites et aucun statut de validation inventé. Contexte borné
   autorisé, conversation identifiée, versions et empreintes conservées.
3. Réutilisation des réservations IA, traces de modèle, limites, leases de commande
   et contrôles de versions/autorisation avant et après génération. Une réponse
   perdue est rejouable sans produire une nouvelle version à chaque tentative.
4. Conversion explicite d’un livrable historique sélectionné vers un brouillon,
   en conservant contenu et sources exacts, sans appel fournisseur ni substitution
   par une version plus récente.
5. UI : choix du document à demander, consigne, génération, relecture, modification,
   validation humaine, puis publication distincte vers la destination configurée.
   Aucun document généré n’est automatiquement validé, confirmé ou publié.

## Preuves

Contrats unitaires pour les cinq formes, sources forgées et limites ; tests UI
sur génération/conversion/échec ; scénarios PostgreSQL sur permissions, sources,
replay et versions historiques, à exécuter uniquement par la stack gardée.
Les fournisseurs simulés restent explicitement des fixtures [FICTIF]. Aucun
appel réel ni publication tierce n’est nécessaire à ces tests.

## État logiciel et preuves du lot

- Implémenté : cinq contrats structurés, agent et contexte de session explicites,
  génération bornée avec quotas et contrôle de sources, édition des sections et
  tickets synchronisée avec le Markdown, validation et publication distinctes.
- La migration `20260923101900_artifact_generation_sources.sql` ajoute les sources
  d’artefacts immuables au graphe et autorise l’opération dans les registres IA.
  Sources projet/société restent soumises aux permissions existantes.
- Conversion exacte FeatureBrief/TechnicalPlan, historique compris, vers un
  brouillon conservant la version, l’empreinte et le contenu originaux.
- Reprise : la consigne et l’identité de commande sont conservées dans le
  navigateur, par acteur/société/projet/type, sans token. Le reçu serveur GET est
  limité au même acteur/projet ; relire ou recharger ne déclenche aucun appel IA.
  Une reprise explicite réemploie la commande. Une commande expirée ne peut pas
  redémarrer implicitement via la remise à zéro générale de l’idempotence.
  La conversion d’un livrable conserve elle aussi une commande par
  acteur/société/projet/version, avec reçu dédié ; une réponse perdue suivie d’un
  rechargement ne crée pas un deuxième brouillon.
- Une correction de texte conserve les instantanés des sources inchangées,
  notamment la borne de conversation vue par l’agent ; les nouveaux messages
  d’une session ne sont pas rétroactivement attribués au brouillon.
- Vérifié localement : 163 unités serveur (dont 5 contrats × 5 protocoles en
  transport synthétique), 164 tests web, construction web et Clippy serveur.
- Préparé pour la stack gardée de l’intégration : sources artefacts exactes,
  idempotence, lecteur/autre société, conversion historique et archivage pendant
  génération ; scénario navigateur réel avec réponse perdue puis rechargement.
  Ces scénarios sont à exécuter par l’intégrateur, aucun accès DB/fournisseur réel
  n’ayant été lancé par ce lot.
- La clôture après révocation réutilise le correctif commun des model_runs du lot
  fiabilité ; elle ne doit jamais rétablir l’accès au contexte ou publier un résultat.
