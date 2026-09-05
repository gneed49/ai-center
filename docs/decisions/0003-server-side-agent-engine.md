# 0003 — Encapsuler le moteur agentique côté serveur

> Statut : acceptée  
> Date : 2026-08-18
> Révision : 2026-09-05

## Contexte

Produit et Tech partagent un moteur mais exigent instructions, retrieval et
sorties différentes. Les réponses doivent devenir des propositions structurées,
pas des mutations implicites.

## Options considérées

1. Appels modèle depuis React.
2. SDK fournisseur directement dans le domaine.
3. Port `AgentEngine` côté serveur, adapter OpenAI et test double déterministe.

## Décision

Adopter l’option 3. L’adapter OpenAI appelle la Responses API avec Structured
Outputs et un modèle choisi explicitement par la configuration. Les schémas de
sortie sont stricts. Le domaine valide à nouveau toute sortie avant persistance.

## Conséquences

- `OPENAI_API_KEY` reste hors des clients.
- Les tests ne dépendent ni du réseau ni d’un résultat probabiliste.
- Le fournisseur peut être remplacé sans changer le graphe.
- Un échec modèle conserve le message et expose une erreur classifiée. Seules
  les erreurs transitoires permettent de réessayer la même commande.

## Révision du 2026-09-05

La configuration OpenAI exige désormais un `OPENAI_MODEL` explicite et calibré
pour le déploiement. Aucun nom de modèle n'est choisi par défaut. Cela remplace
le défaut `gpt-5.6-luna` de la décision initiale ; une calibration non exécutée
reste une porte de validation ouverte, sans preuve fournisseur implicite.

La reprise dépend de la classe de l'échec : délai dépassé, connexion, limite de
débit et erreur serveur sont transitoires. Quota épuisé, authentification,
autorisation, ressource absente, requête invalide et sortie hors contrat sont
permanents. Une réponse permanente est rejouée par l'idempotence ; elle ne
relance pas automatiquement le fournisseur. Cette distinction remplace la
formulation initiale qui rendait tout échec modèle réessayable.

## Conditions de révision

Réviser si un SDK Rust officiel apporte un bénéfice fonctionnel net ou si un
autre fournisseur devient une exigence produit.
