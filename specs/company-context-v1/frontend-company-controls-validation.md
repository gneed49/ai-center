# Contrôles entreprise — frontend, 21 septembre 2026

Selon `automation-plan.md` et `company-data-plan.md`, l'écran entreprise
présente désormais les limites et l'activité réelles du serveur, la pause et
la reprise propriétaires avec génération attendue, l'export complet canonique
et l'archivage/restauration des projets.

- Les appels sur la dernière heure et simultanés sont comparés aux limites
  serveur. Un coût absent est « Non disponible », avec le nombre d'appels
  dépourvus d'estimation ; ce n'est jamais un coût nul implicite.
- La pause vise les appels locaux et nouveaux traitements IA/publication.
  L'écran explique qu'une demande déjà acceptée par un service externe n'est
  pas rétractable et peut encore avoir un résultat ou un coût.
- La commande de pause conserve sa génération initiale lors d'une reprise
  ambiguë, même si la vue s'est actualisée entre-temps.
- L'export demande les octets JSON canoniques à `/api/company/export` avec
  `no-store`, vérifie encore l'identité avant téléchargement et ne recompose
  jamais un export partiel dans le navigateur. Le fichier reste propriétaire.
- Archiver retire un projet actif ; le restaurer est une commande explicite
  distincte. L'historique reste lié depuis la liste d'archives et les anciens
  packs de contexte sont signalés comme dépassés après restauration.

Validation locale : **122 tests réussis dans 31 fichiers**, lint sans
avertissement, compilation TypeScript/Vite réussie. Six scénarios nouveaux
couvrent coûts inconnus, reprise avec génération initiale, accès nonowner,
archive/restauration, export explicite avec nettoyage du Blob et refus d'un
export terminé après changement d'entreprise.

Ces contrôles UI ont été exercés avec fixtures `[FICTIF]` et API simulées.
Ils ne constituent pas une preuve d'arrêt d'un fournisseur réel ni de
sauvegarde distante. Les tests serveur de pause/annulation, d'isolation et de
complétude de l'export sont distincts et menés dans leurs lots backend.
