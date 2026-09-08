# Deux projets fictifs — validation du contexte

> Statut : active — demande du propriétaire du 8 septembre 2026.

## Intention et amendement

Le propriétaire demande : « pour les projets je m'en fou invente 2 cas teste
toi même ». Cette consigne autorise deux projets inventés et retire, pour ce
lot, l'attente du choix de deux projets réels. Elle conserve sa consigne de
validation par tests unitaires et d'intégration ; les E2E restent à sa charge.

Cette validation complète [Alpha Context Proof](../alpha-context-proof/spec.md).
Les scénarios synthétiques ne deviennent pas des observations humaines ni une
campagne comparative auprès de fournisseurs réels. Les seuils et les preuves
de publication de l'alpha ne sont pas modifiés.

## Cas versionnés

1. **Médiathèque Partagée** : une association suit les prêts de ses ouvrages.
   Son historique de prêts reste consultable sans expiration. Une proposition
   technique de purge après 30 jours entre en conflit avec cette règle ; sa
   correction doit permettre de reconstruire un contexte courant.
2. **Atelier Réparable** : un atelier suit les réparations d'appareils.
   Les photos de diagnostic sont supprimées après 30 jours. Son plan doit
   conserver cette exigence, même si l'autre projet a une règle de conservation
   opposée. Un changement dans la médiathèque ne doit pas invalider ce projet.

Ces projets, noms et contenus sont fictifs et sans donnée personnelle. Ils
restent dans les fixtures de test, pas dans le seed de développement ni dans
un manifeste présenté comme une campagne réelle.

## Interfaces testées et acceptation

Le périmètre déjà autorisé couvre les services publics du serveur et les
contrats du compilateur de contexte. PostgreSQL est réel et jetable ; le moteur
IA déterministe est un double explicite. Aucun fournisseur distant n'est appelé.

| ID      | Critère                                                                                                                               | Preuve attendue                                                                                                 |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| SYN-001 | Les deux cas parcourent intention, propositions, confirmation, gate, brief, ContextPack, handoff et plan technique.                   | Tests d'intégration avec assertions sur les sorties des services.                                               |
| SYN-002 | Le gate refuse un projet incomplet ; seules les connaissances confirmées alimentent son contexte.                                     | Test avant confirmation, puis passage après confirmation ; texte non confirmé absent du pack.                   |
| SYN-003 | Le contexte et les sources d'un plan appartiennent à leur projet ; une référence au pack de l'autre projet est refusée.               | Deux projets simultanés dans le même workspace ; assertions positives et négatives sur UUID et contenu.         |
| SYN-004 | La sélection, les versions et la provenance restent vérifiables ; les informations inutiles ne deviennent pas obligatoires.           | Assertions unitaires sur le contrat du compilateur et assertions d'intégration sur les exports.                 |
| SYN-005 | La contradiction médiathèque bloque l'utilisation du contexte concerné ; une révision conserve l'ancien pack et permet de recompiler. | Résolution par le service, ancienne version préservée, nouveau pack courant ; pack atelier inchangé et courant. |
| SYN-006 | Un plan sans preuve externe conserve une couverture manquante.                                                                        | Assertions sur la couverture des deux cas, sans succès métier ou preuve inventés.                               |
| SYN-007 | Les tests sont collectés par les commandes existantes et exécutés sans E2E sur la seule base d'intégration.                           | Commandes exactes, résultats datés, inventaire des scénarios et limites dans le rapport.                        |

## Hors portée

Pas de parcours navigateur, application native ou mobile, de clé réelle,
d'invitation, de paiement, de déploiement ou de push. Aucune nouvelle fonction
produit n'est requise si les contrats existants satisfont les scénarios.
