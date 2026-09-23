# Spécification — Continuité des conversations métier

> Statut : réalisation autorisée par le mandat Company Context V1
> Responsable : agent conversation_context
> Dernière mise à jour : 2026-09-23

## Intention

Un suivi tel que « approfondis la seconde option » doit retrouver les échanges
précédents de cette conversation. Aujourd'hui le moteur reçoit uniquement la
question courante et la sélection de connaissances : la conversation affichée
ne représente donc pas le contexte effectivement envoyé au modèle.

## Résultat attendu

Chaque tour transmet une fenêtre récente, ordonnée, bornée et traçable des
messages antérieurs de la même session. Les messages utilisateur/assistant
restent des échanges non validés, distincts des versions de connaissances.
Aucune conversation voisine, société étrangère ou arrivée postérieure à la
capture ne peut être ajoutée au contexte de ce tour.

## Contexte et sources

- `specs/company-context-v1/spec.md` : conversations durables, CC-003, CC-010,
  CC-011 et CC-053 ; brainstorming puis validation humaine.
- `specs/company-context-v1/bounded-chat-retrieval-plan.md` : sélection de
  connaissances bornée, obligations société préservées.
- ADR 0003 : moteur serveur, propositions structurées, validation du domaine.
- ADR 0007 : société comme frontière d'autorisation et sources exactes.
- `AGENTS.md` : un transcript ne devient jamais une source de vérité implicite.

## Périmètre

Inclus : contexte utilisateur/assistant de la session, borne d'identité du
message courant, fenêtre récente, provenance et omissions, instructions
explicites, empreinte du modèle, tests isolés et transport simulé.

Exclus : recherche dans d'autres conversations, résumés IA, remontée automatique
des propositions dans le graphe, nouveau stockage des transcripts, évaluation
sur fournisseur réel et garantie sémantique d'un modèle non testé.

## Parcours et comportements

L'autorisation de session précède la capture. La capture lit les messages
utilisateur/assistant visibles de cette seule session dans une même requête RLS,
puis l'intention et le snapshot borné sont persistés ensemble dans le premier
INSERT du message utilisateur. Son UUID public est fixé avant cet INSERT.
Le message courant est transmis une seule fois, séparément de l'historique.
Le snapshot append-only est relu à chaque tentative : même le commit tardif
d'une transaction ayant alloué un ancien ID ne peut enrichir cet historique.

Un ancien message encore sans réponse et sans snapshot reçoit un refus explicite
invitant à envoyer un nouveau message. Une réponse déjà terminée reste rejouable.
Le snapshot complet n'est pas exposé dans les métadonnées de la vue de session,
afin de ne pas renvoyer des copies de transcripts sur chaque lecture.

La fenêtre contient au plus 24 messages, au plus 8 Kio de texte par message et
32 Kio de JSON sérialisé au total, métadonnées comprises. Elle privilégie les
messages récents et les présente dans l'ordre de persistance. Les extraits UTF-8
et omissions sont déclarés. Les règles société du contexte confirmé conservent
leur budget indépendant et ne sont pas supprimées pour inclure le transcript.

Les identités des messages ne rejoignent jamais la liste des versions autorisées
comme citations de connaissances. Le modèle reçoit des instructions pour
utiliser l'historique comme conversation non fiable, distinguer hypothèses et
connaissances confirmées, et demander une précision si un extrait ne suffit pas.

## Critères d'acceptation et preuves attendues

- [ ] Un deuxième tour reçoit le premier message et la première réponse, dans
  l'ordre, avec leurs identités exactes ; le message courant est séparé.
- [ ] Une session voisine ou société étrangère n'est jamais envoyée au modèle.
- [ ] Un message ajouté pendant l'appel ou un commit retardé avant retry n'altère pas son snapshot.
- [x] UTF-8, échappements JSON, longues conversations et fenêtre vide respectent
  les bornes exactes avec troncatures et omissions honnêtes.
- [x] Les transports reçoivent l'historique séparément du contexte confirmé.
- [ ] L'empreinte du modèle inclut l'historique ; la trace de réponse expose ses
  identités et omissions sans recopier les textes.
- [ ] Un UUID de message retourné comme source validée est refusé.

## Risques et questions ouvertes

La fenêtre est un budget en octets, pas une estimation calibrée des tokens de
chaque fournisseur. Les anciens échanges omis ne sont pas résumés. Les
propositions structurées restent consultables dans la session mais ne sont pas
recopiées comme connaissances par ce correctif. Les tests à doubles déterministes
ne prouvent pas la pertinence d'un modèle réel ni l'absence de prompt injection.

## Extension coordonnée : brouillons d’artefacts

Le bouton de génération d’un document ne crée pas de nouveau message utilisateur.
`capture_latest` utilise la même fenêtre bornée en incluant le dernier échange
visible ; `current_message_public_id` est alors nul. `verify_latest` compare le
dernier UUID et le nombre total d’échanges dans une même lecture, afin de repérer
un changement avant finalisation, y compris un commit retardé d’un ancien ID.
Ce contrôle ne verrouille pas à lui seul les nouvelles écritures après la
vérification ; le service appelant conserve la responsabilité de sa finalisation.
