# 0001 — Séparer les surfaces clientes du plan de contrôle serveur

> Statut : acceptée  
> Date : 2026-08-18

## Contexte

Le scope initial privilégiait une PWA et repoussait les applications desktop.
Le démarrage réel se fait sous Linux et doit préparer web, desktop et mobile
sans lier le cœur contextuel à une machine.

## Options considérées

1. Web/PWA uniquement.
2. Tout le domaine dans Tauri.
3. Web responsive partagé, enveloppe Tauri et serveur indépendant.

## Décision

Adopter l’option 3. React/Vite produit la surface responsive unique. Tauri v2
l’embarque sous Linux et fournit uniquement les capacités natives nécessaires.
Un serveur Rust/Axum porte le domaine et peut être déployé indépendamment.

## Conséquences

- Le développement Linux valide immédiatement web et desktop.
- Tauri ne devient ni une base locale ni un serveur central déguisé.
- Une future application mobile peut consommer la même API.
- Deux processus Rust existent, mais leurs responsabilités restent nettes.

## Révision

Réviser si le fonctionnement hors ligne complet devient un critère P0 ou si
Tauri mobile impose une contrainte incompatible avec le bundle partagé.
