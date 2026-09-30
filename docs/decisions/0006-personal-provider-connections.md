# 0006 — Connexions fournisseurs personnelles et exécution stable

> Statut : acceptée pour l'implémentation demandée le 7 septembre 2026.

## Contexte et options

Le propriétaire demande plusieurs clés configurables dans AI Center et des
abonnements fournisseurs lorsque leur mécanisme officiel est compatible.
Une clé globale par serveur ne permet pas plusieurs comptes personnels ; un
appel direct depuis le navigateur exposerait les secrets et déplacerait les
invariants métier côté client. Cette décision étend l'ADR 0003.

## Décision

Une **connexion fournisseur** est un profil nommé appartenant à une personne
dans un workspace : mode d'accès, fournisseur et modèle. Plusieurs profils
du même fournisseur coexistent. La sélection personnelle est persistante ;
elle est résolue une fois pour chaque opération IA, conservée jusqu'à sa fin
et attribuée dans les traces du modèle. Les clés sont chiffrées côté serveur
avec un contexte cryptographique lié au propriétaire et à la connexion.

Les événements différés conservent l'acteur demandeur : le Steward utilise sa
sélection, même si un autre membre déclenche le traitement de l'événement.
Un échec ne remplace jamais une connexion sélectionnée par celle d'un autre
compte. Supprimer le profil actif sélectionne le mode déterministe.

Les abonnements passent par un client officiel intact et une authentification
personnelle. Aucun token d'une autre application n'est importé. Le client
local ne reçoit aucun outil d'action, accès au projet, hook ou serveur MCP ;
la capacité est désactivée si cette frontière n'est pas garantie. Le mécanisme
interne de validation StructuredOutput de Claude reste permis. AI Center
conserve sa frontière de contrôle du contexte, sans devenir un runner.

## Conséquences et révision

Les sauvegardes de base chiffrées exigent une conservation séparée du secret
de chiffrement pour être restaurables. Les données d'authentification du client
officiel restent gérées par ce client. Une formule ChatGPT ou Claude ne finance
pas automatiquement les appels d'une clé API.

Réviser cette décision si une connexion d'équipe est demandée explicitement,
si un protocole fournisseur officiel permet un abonnement distant isolé, ou si
Codex fournit une désactivation démontrable de tous les outils d'action. Une
évolution de version ou des politiques du client local doit revalider sa
capacité, jamais élargir silencieusement ses permissions.
