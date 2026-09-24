# Spécification — Clôturer un appel IA après retrait des droits

> Statut : réalisation autorisée par le mandat Company Context V1
> Dernière mise à jour : 2026-09-23

## Intention et sources

CC-003/CC-014 et ADR 0003 imposent que les appels IA interrompus laissent un état
vérifiable, sans contourner les droits. Aujourd’hui la révocation ou rétrogradation
lecteur pendant un appel retire à l’UPDATE normal RLS l’accès au model_run : le
fournisseur s’arrête, mais le run peut rester running indéfiniment.

## Comportement attendu

Les nouveaux runs mémorisent l’acteur qui les a commencés, de manière immuable.
Les lignes historiques restent sans attribution inventée. Une fonction privée
très étroite peut seulement annuler un run encore running, de la société et de
l’acteur de la requête, lorsque cet acteur n’est plus membre accepté avec droit
d’écriture, ou lorsque son rôle réel ne correspond plus au rôle capturé pour la
requête (y compris owner → editor et editor → owner). Elle ne retourne aucune donnée métier et ne prend ni output, ni
payload, ni statut libre. Elle n’accorde aucun accès de lecture ou de publication.

Les écritures normales restent sous RLS. Sur erreur pendant le fournisseur ou
la finalisation, la clôture de secours utilise une transaction séparée si
nécessaire, sans persister la réponse IA après révocation. Un autre acteur,
une autre société, un ancien run sans acteur et un run déjà terminé ne peuvent
pas être modifiés. Un acteur dont l’autorisation d’écriture est restée identique ne peut pas
utiliser ce mécanisme.

## Critères et preuves

- [ ] Révocation, rétrogradation viewer et changements owner ↔ editor pendant un
  appel : provider interrompu, model_run cancelled, code fixe access_changed,
  aucune réponse/livrable persisté.
- [ ] Acteur d’origine fixé à l’INSERT et impossible à réattribuer par UPDATE.
- [ ] Autre acteur, société étrangère, run historique sans acteur, run terminé et
  auteur encore autorisé refusés par le helper sans donnée renvoyée.
- [ ] Chat et génération d’artefacts partagent le même nettoyage de finalisation ;
  les échecs du Steward utilisent le même helper sans persister sa réponse.
- [ ] Inventaire runtime et export incluent les nouveaux éléments sans secret.

Aucune tentative de récupérer des réponses provider ni de réessayer après
révocation. Le déploiement requiert la migration 21 avant le serveur. Une panne
DB empêche la clôture immédiate : elle reste une récupération opérationnelle,
pas un motif de contournement des permissions.

Le bail de progression du Steward reste protégé par les droits normaux. Après
un changement d’autorité, sa libération immédiate peut être invisible sous RLS ;
un membre encore autorisé peut le reprendre après son expiration, au plus onze
minutes après la prise de bail. Le model_run est clôturé immédiatement sans
élargir les droits sur les reçus ou sur la progression.
