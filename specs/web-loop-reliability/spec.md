# Spécification — Reprise fiable et export du contexte web

> Statut : implémenté localement, validation logicielle effectuée  
> Responsable : livraison V1 web  
> Dernière mise à jour : 2026-09-21

## Intention

L'autorisation utilisateur du 21 septembre lance la réalisation de la V1
professionnelle. Ce lot borné concrétise WEB-05/06/07 de l'état des lieux :
ne pas annoncer à tort qu'un message n'a pas été enregistré après une réponse
perdue, préserver son identité au réessai, et permettre d'utiliser un ContextPack
dans un outil externe depuis l'interface web existante.

## Résultat attendu

Le même message réessayé depuis la session conserve la même commande après une
erreur. Un pack courant peut être copié en Markdown ou téléchargé en Markdown
et JSON depuis le handoff, avec sa version et sa provenance canoniques.

## Contexte et sources

- [Alpha Context Proof](../alpha-context-proof/spec.md), ACP-051 et export externe.
- [Décision de contrôle du contexte](../../docs/decisions/0005-context-control-not-tool-replacement.md).
- Endpoint existant `GET /api/context-packs/{id}/export?format=json|markdown`.
- Le frontend reste consommateur de l'API ; aucune clé ni contenu de pack
  n'est ajouté au stockage navigateur par ce lot.

## Périmètre

### Inclus

- Réessai d'un message identique, depuis Envoyer ou Réessayer, avec même UUID.
- Distinction entre résultat non confirmé et rejet connu ; consultation de la
  session après une erreur sans prétendre annuler le traitement serveur.
- Export authentifié, isolation après changement d'identité, format serveur
  conservé, nom de fichier stable avec ID/version, statut obsolète explicite.

### Exclu

- Nouvelle console graphe et administration d'entreprise.
- Modification du contrat serveur, délais proxy, opérations asynchrones.
- Persistance des brouillons/commandes après fermeture ou rechargement, à
  traiter avec la stratégie globale de reprise WEB-07.
- Preuve réelle d'usage du fichier dans un outil externe ou de GitHub.

## Critères d'acceptation

- [x] WR-01 : réponse 504 ou perdue puis Envoyer sans édition ne crée pas de
  nouvelle identité ; une édition réelle crée une nouvelle commande.
- [x] WR-02 : une erreur de transport ne prétend ni que le message est absent
  ni que l'appel fournisseur est annulé ; le contenu reste disponible.
- [x] WR-03 : copier Markdown et télécharger Markdown/JSON utilise l'API
  authentifiée, sans réécrire le contenu, ses sources et sa version.
- [x] WR-04 : un pack obsolète est explicitement signalé et ne peut être
  proposé comme contexte courant ; la fraîcheur est vérifiée à l'action.
- [x] WR-05 : un changement d'identité pendant la lecture ne copie ni ne
  télécharge de données du workspace précédent.

## Preuves attendues

Tests unitaires HTTP et tests de composants en mémoire, lint et build web.
Les parcours navigateur et fournisseurs réels ne sont pas certifiés par ce lot.
Résultats et périmètre : [validation.md](validation.md).

## Risques et questions ouvertes

Une évolution de connaissances après l'instant de vérification peut rendre
obsolète un fichier déjà exporté ; son ID/version permettent la traçabilité.
Les permissions presse-papiers dépendent du navigateur et nécessitent HTTPS.
