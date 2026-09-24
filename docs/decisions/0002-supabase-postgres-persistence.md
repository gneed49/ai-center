# 0002 — Utiliser Supabase PostgreSQL derrière l’API Rust

> Statut : acceptée  
> Date : 2026-08-18

## Contexte

Le graphe, ses versions et ses projections doivent être durables, auditables et
déployables. Le MVP est mono-utilisateur mais doit conserver workspace et acteur.

## Options considérées

1. SQLite local dans Tauri.
2. Data API Supabase appelée directement depuis React.
3. Supabase PostgreSQL accédé par le serveur Rust.

## Décision

Adopter l’option 3 avec SQLx. Les tables applicatives vivent dans le schéma privé
`app`, géré par des schémas déclaratifs et migrations versionnées. Le client ne
possède aucun accès base direct. La RLS est activée en défense en profondeur ;
le serveur applique l’autorisation par workspace.

## Conséquences

- Le modèle relationnel, les transactions et les indexes servent directement la
  traçabilité et le graphe.
- La base locale Supabase est reproductible par Docker.
- Supabase Auth et la Data API restent ajoutables sans refonte du domaine.
- Une coupure client ne peut pas effacer une connaissance déjà confirmée.

## Révision

Réviser si un mode offline-first complet est priorisé ou si le produit exige une
réplication locale multi-master.
