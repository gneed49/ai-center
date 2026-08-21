# 0003 — Encapsuler le moteur agentique côté serveur

> Statut : acceptée  
> Date : 2026-08-18

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
Outputs et un modèle configurable, par défaut `gpt-5.6-luna`. Les schémas de
sortie sont stricts. Le domaine valide à nouveau toute sortie avant persistance.

## Conséquences

- `OPENAI_API_KEY` reste hors des clients.
- Les tests ne dépendentent ni du réseau ni d’un résultat probabiliste.
- Le fournisseur peut être remplacé sans changer le graphe.
- Un échec modèle conserve le message et retourne une erreur réessayable.

## Révision

Réviser si un SDK Rust officiel apporte un bénéfice fonctionnel net ou si un
autre fournisseur devient une exigence produit.
