# Recette E2E à réaliser par le propriétaire

Conformément à la consigne du 7 septembre 2026, aucun parcours de cette liste
n'est exécuté ou présumé réussi par l'agent. Les tests automatiques de la
livraison restent des unités, faux HTTP/processus et intégrations PostgreSQL.

## Démarrer

Depuis le dossier du dépôt :

```bash
./dev
```

Le lanceur Linux démarre PostgreSQL/Supabase, le serveur et l'application Tauri.
Les prérequis et commandes d'arrêt sont dans le
[guide de développement](../../docs/development.md). Le mode sans fournisseur
est le défaut. `./dev doctor` peut diagnostiquer un prérequis manquant.

## Clés API et sélection

1. Ouvrir **Réglages IA** et ajouter une connexion avec une clé personnelle et
   un modèle réellement disponible sur le compte. Ne pas envoyer la clé dans
   un chat, un rapport ou une capture d'écran.
2. Vérifier l'accès au catalogue. Ouvrir **Modifier**, choisir un identifiant
   et enregistrer. Laisser la nouvelle clé vide doit conserver la précédente.
3. Ajouter une deuxième connexion du même fournisseur, puis une d'un autre
   fournisseur. Chaque nom, modèle et clé masquée doit rester distinct.
4. Choisir un profil dans **Connexion utilisée**, appliquer, recharger et
   vérifier la persistance du choix. Tester une conversation puis la boucle
   Produit → ContextPack → Handoff → plan → couverture → historique, avec le
   contenu et le budget du propriétaire. Vérifier le fournisseur/modèle
   effectivement servi et l'absence d'invention de provenance.
5. Essayer une connexion invalide ou un modèle non autorisé. L'application doit
   exposer l'échec sans utiliser une autre clé. Remplacer la clé depuis sa fiche
   et réessayer selon la classe d'erreur affichée.
6. Supprimer le profil actif : le mode sans fournisseur doit devenir actif.
7. Avec deux utilisateurs autorisés et deux workspaces, vérifier que les
   connexions et les saisies temporaires ne passent jamais d'un contexte à
   l'autre. Les analyses différées doivent conserver l'acteur demandeur.

## Abonnement compatible

1. Si Claude est disponible, créer son profil et cliquer sur **Connecter mon
   abonnement**. Terminer dans le flux officiel avec son compte personnel.
2. Vérifier l'annulation et l'expiration d'une tentative. Après connexion,
   sélectionner le profil et tester une demande structurée sur un petit
   contexte autorisé, en vérifiant les limites et la facturation fournisseur.
3. Déconnecter le profil. Une nouvelle demande ne doit pas réutiliser une clé
   API de substitution. Tester aussi le message d'indisponibilité lorsque le
   client compatible manque.
4. ChatGPT/Codex ne propose pas de connexion active dans cette livraison :
   l'absence de tous les outils d'action doit d'abord être démontrée. Cette
   limitation est attendue ; elle ne constitue pas une connexion réussie.

Noter les versions, le modèle, l'état observé et les erreurs sans enregistrer
de clé, lien OAuth, token, cookie ou contenu personnel dans les preuves.
