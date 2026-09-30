# Plan — Clôture après retrait des droits

> Statut : en cours
> Spec liée : spec.md

1. Migration 21 additive : colonne started_by_actor_id nullable puis DEFAULT
   app.current_actor_id(), politique INSERT restrictive et trigger d’immutabilité.
2. Helper SECURITY DEFINER à search_path vide / RLS désactivée uniquement en
   interne. Vérifier société+acteur GUC, appartenance et rôle réels contre le rôle capturé, run running et
   propriétaire exact. Ne permettre que cancelled et un code/message constants.
3. Autoriser seulement ai_center_runtime à l’exécuter, étendre le vérificateur
   de privilèges et la liste explicite des colonnes d’export.
4. Normaliser fail_model_run_in_transaction quand UPDATE RLS touche zéro ligne.
   Envelopper les finalisations chat/artefact pour effectuer le nettoyage après
   rollback sur toute sortie d’erreur, sans accepter un output après perte d’accès.
   Appliquer aussi le helper aux UPDATE de clôture Steward sans ligne visible ;
   garder la reprise de bail sous les droits normaux et son expiration bornée.
5. Tests PostgreSQL dans company_context (stack isolée pilotée par root), tests
   existants, compilation/Clippy. L’empreinte de schéma d’effacement finale est
   recalculée par root après inspection de la migration ; aucune base ici.

Rollback : retour du serveur précédent possible après migration additive. Ne
pas retirer l’attribution des runs ou le helper tant que des appels sont actifs.

## Validation locale et preuve restante

- `cargo check -p ai-center-server --all-targets` passé.
- `cargo clippy -p ai-center-server --all-targets -- -D warnings` passé après
  les 12 scénarios et les clôtures Steward ; journal
  `.run/model-run-access-clippy.log`. `git diff --check` passé.
- Migration 21 et schéma 21 identiques ; grant runtime 99 et inventaire du
  vérificateur incluent le helper. L’empreinte d’effacement reste pilotée par root.
- Scénario PostgreSQL livré : chat, artefact et Steward croisés avec révocation,
  rétrogradation viewer, owner → editor et editor → owner ; assertion de clôture
  sans réponse, artefact ou output. Le cas Steward vérifie aussi le bail restant
  borné puis une reprise autorisée après expiration simulée.
- Second scénario : refus autre acteur, autre société, état terminé et attribution
  historique NULL ; refus de réattribution et de création au nom d’un autre acteur.
- Les scénarios DB n’ont pas été exécutés par ce sous-lot : leur exécution réelle
  appartient à la recette PostgreSQL isolée pilotée par l’agent principal.
  Aucun fournisseur externe sollicité.
