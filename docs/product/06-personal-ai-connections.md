# Connexions IA personnelles

Évolution demandée le 7 septembre 2026. La [spécification de livraison](../../specs/provider-connections/spec.md)
complète le périmètre MVP ; les parcours E2E sont réalisés par le propriétaire.

## Parcours utilisateur

Dans **Réglages IA**, ajouter une connexion nommée, choisir un fournisseur et
saisir l'identifiant du modèle. OpenAI, Anthropic, Kimi/Moonshot, DeepSeek et
OpenRouter sont proposés avec leurs API officielles. Plusieurs clés du même
fournisseur peuvent être enregistrées sous des noms différents.

La clé est saisie dans un champ masqué et chiffrée côté serveur. Après
enregistrement, seule une indication masquée est consultable. « Modifier »
permet de changer le modèle, de renommer le profil ou de remplacer sa clé ;
laisser le champ de nouvelle clé vide conserve celle déjà enregistrée.

« Vérifier l'accès et les modèles » interroge le catalogue du fournisseur,
sans transmettre de contenu du projet. Le catalogue aide à choisir un
identifiant dans le formulaire. Sa réussite ne prouve pas qu'un modèle accepte
le format demandé, ni la qualité d'une génération ; les essais réels restent
nécessaires pour le modèle et le compte retenus.

Dans **Connexion utilisée**, sélectionner un profil puis appliquer le choix.
Il couvre les conversations, la sélection de contexte, les plans, leur
couverture et les analyses Steward. Une demande commencée conserve sa
connexion jusqu'à sa fin. Une erreur ne déclenche aucune bascule automatique.
La suppression de la connexion active remet explicitement le mode sans
fournisseur. La configuration du serveur reste un choix distinct.

Les profils appartiennent à la personne dans le workspace courant. Ils ne
sont pas partagés avec les autres membres. Les rôles owner/editor gèrent ces
réglages ; le rôle viewer reste en lecture métier et n'y accède pas.

## Abonnements

Un abonnement désigne ici une formule existante chez un fournisseur. Aucun
paiement ni abonnement commercial AI Center n'est ajouté.

**Claude par abonnement** utilise le client officiel local et un profil privé.
Après création du profil, « Connecter mon abonnement » propose le lien officiel
de connexion. Le compte est authentifié par Claude Code. Une tentative peut
être annulée ; son lien disparaît à la fin ou au changement de contexte.
La déconnexion est disponible sur la fiche, y compris si le compte authentifié
est un compte API ou d’organisation non admissible. Le motif reste affiché et
**Changer de compte** permet de reprendre le flux officiel sans recréer le profil.
La fin de l’authentification est vérifiée avant d’annoncer la déconnexion.

Le support local est limité aux capacités effectivement vérifiées par le
serveur : Linux, client compatible, abonnement personnel pris en charge et
absence de politique administrée qui invalide les restrictions. Une capacité
manquante produit une explication visible. Les comptes d'organisation restent
indisponibles dans cette version. Le
[contrat des abonnements](../../specs/provider-connections/subscriptions.md)
précise les prérequis et limites.

**ChatGPT via Codex** est affiché comme indisponible tant que le client ne
permet pas de garantir la désactivation de tous les outils d'action. La voie
OpenAI par clé API reste disponible. Les formules API et abonnement demeurent
distinctes : aucune conversion implicite de session en clé.

## Preuve de livraison

Les tests unitaires et d'intégration vérifient les contrats, l'isolation et les
échecs avec des fournisseurs simulés. Le propriétaire effectue la recette E2E
et ses essais avec de vrais comptes. Le
[guide de recette manuelle](../../specs/provider-connections/manual-verification.md)
fournit les vérifications à reproduire, sans les présumer réussies.
