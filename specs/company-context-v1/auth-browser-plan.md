# CC-002/003/004 — Deux comptes réels dans le navigateur local

Compléter les preuves séparées Auth HTTP, invitations PostgreSQL et UI simulée
par une recette commune. Sur la seule stack jetable gardée : créer deux comptes
Auth fictifs, vérifier leurs OTP, restaurer leurs sessions réelles dans deux
contextes Chromium puis créer la société et une invitation depuis l'interface.

Vérifier acceptation, projet partagé, lecteur refusé en écriture, changement de
rôle, retrait effectif sur l'ancien JWT et impossibilité de réutiliser l'ancienne
invitation. Aucun fournisseur IA, compte client ou SMTP externe. Les secrets
éphémères ne sont ni des fixtures versionnées ni des diagnostics Playwright :
fichier privé temporaire, trace/vidéo/capture désactivées, suppression finale.
Le helper doit refuser une cible autre que la stack gardée et nettoyer ses
processus et utilisateurs après succès/échec ; l'orchestrateur détruit ses volumes.

La recette attend également la réponse refusée et l’état d’erreur rendu après
rechargement ; l’absence d’un titre pendant le chargement ne vaut pas preuve de
révocation dans l’interface. La connexion est préparée par OTP Auth local : ce
scénario ne qualifie pas l’envoi SMTP ou le formulaire de connexion.
