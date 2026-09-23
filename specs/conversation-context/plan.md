# Plan d'implémentation — Continuité des conversations métier

> Statut : en cours
> Spec liée : `spec.md`

1. Introduire un module serveur qui capture sous la transaction autorisée une
   fenêtre des messages précédant l’INSERT du message courant. Le plafond de
   lignes et les extraits limitent aussi les données lues. Conserver le snapshot
   dans les métadonnées append-only de ce message et le restaurer à chaque retry,
   sans le recopier dans la vue publique de session.
2. Construire un DTO explicite séparant provenance, rôles, textes et limites.
   Appliquer un plafond sur le JSON final, y compris les échappements UTF-8.
3. Ajouter ce DTO à `AgentInput`, aux données du transport structuré et à
   l'empreinte du tour. Conserver une trace sans textes dans la réponse.
4. Tester la fenêtre et le contrat de transport sans fournisseur externe ;
   ajouter un scénario PostgreSQL sous `company_context` pour les autorisations,
   le brainstorming de plusieurs tours, la capture et les citations interdites.
5. Exécuter formatage, unités et Clippy. Confier le scénario PostgreSQL à la
   stack isolée déjà pilotée par l'agent principal, sans lancer de seconde base.

Aucune migration ni modification des règles RLS. Aucun secret, appel facturé ou
résumé de conversation. Le retour arrière est un retour du code serveur ; la
trace JSON additive reste lisible par les clients existants.

## Validation au 23 septembre 2026

- Formatage et contrôle des espaces : passés.
- `cargo test -p ai-center-server --lib` : **160 réussis, 10 ignorés** ; inclut
  trois tests de fenêtre (récence, budget UTF-8/JSON et extrait SQL) et le contrat
  d’historique sur les cinq transports simulés. Journal local :
  `.run/conversation-context-units.log`.
- Le scénario `company_context::conversation` est livré pour le prochain passage
  de la stack PostgreSQL isolée pilotée par l’agent principal. Il couvre deux
  tours, auteur collègue, sessions/sociétés étrangères, message tardif, empreinte
  exacte, provenance sans texte et refus d’un message comme connaissance citée.
  Il n’est pas présenté comme exécuté à ce stade.
- Clippy global final : `cargo clippy -p ai-center-server --all-targets -- -D
  warnings` passé après les travaux simultanés, journal
  `.run/model-run-access-clippy.log`.
- Aucun fournisseur réel sollicité ; la pertinence sémantique reste une preuve
  distincte à établir avec le compte et le budget autorisés.

## Correction de la concurrence lors d’une reprise

La revue a identifié qu’une simple borne d’ID exclut les nouveaux IDs mais pas
le commit tardif d’un ancien ID. Le snapshot est désormais durable dès le premier
INSERT du message, et la restauration vérifie société via session autorisée,
identités, rôles, compteurs et budgets. Un quatrième test unitaire couvre la
restauration et les refus. Un second scénario PostgreSQL réserve un ancien identifiant avant le premier
échec fournisseur, puis insère et commit le message après cet échec, et exige
un historique strictement identique au retry. La réservation seule évite de
retenir les verrous FK de projet pendant la première tentative ; le test est
borné à 30 secondes et l’insertion tardive possède un lock_timeout de 5 secondes. Il vérifie aussi le refus
lisible des anciens messages sans capture. Aucun appel fournisseur réel.

Validation finale des unités après snapshot durable : `cargo test -p
ai-center-server --lib` **162 réussis, 10 ignorés**, journal
`.run/conversation-context-units-final.log`. Compilation des scénarios
`company_context` réussie ; exécution PostgreSQL à faire par root.
