# Validation locale — 21 septembre 2026

Le lot UI de WEB-05/06/07 est implémenté. La validation utilise uniquement des
fixtures `[FICTIF]`, sans fournisseur réel ni données utilisateur.

| Vérification | Résultat |
| --- | --- |
| Unités et composants web | 62 tests réussis dans 13 fichiers, dont 13 nouveaux contrats du lot. |
| Lint web | Réussi, zéro avertissement. |
| TypeScript et build Vite | Réussis ; avertissement historique de bundle principal supérieur à 500 kB. |
| Isolation identité | Réponse texte et copie différée refusées après changement de workspace. |
| Commande après 504 | Envoyer et Réessayer reprennent le même UUID ; vraie édition crée une nouvelle commande. |
| Export | Bytes canoniques conservés, MD/JSON, version dans le nom de fichier, fraîcheur vérifiée sans cache et pack obsolète bloqué. |

Les tests Playwright existants ont seulement été actualisés pour le nouveau
libellé d'erreur ; ils n'ont pas été exécutés. Aucun navigateur, appel IA,
GitHub réel, conteneur d'hébergement ou déploiement n'est certifié ici.

## Limites maintenues explicitement

- La copie réelle dépend des permissions navigateur et d'HTTPS ; le
  téléchargement reste proposé quand la copie échoue.
- La fraîcheur est contrôlée lors de l'action. Un fichier déjà exporté reste
  rattaché à sa version même si les connaissances évoluent ensuite.
- Ce lot ne conserve pas les brouillons et clés de commande après un
  rechargement, et ne change pas les délais proxy/serveur. La reprise durable
  et la preuve via proxy de WEB-07 restent à compléter dans le lot global.
- L'export ne marque pas automatiquement un travail externe comme terminé,
  ni une référence comme preuve validée.
