# Qualification locale de l'éditeur de tickets — 30 septembre

Périmètre : socle T17 qualifié `82c3173`, lot T19 et correctif d'exploitation
PostgreSQL 17.11. Les modifications T18 en cours sont exclues de cette recette,
exécutée dans un snapshot privé. Aucun appel IA facturé ou outil externe réel.

## Résultat fonctionnel

Le parcours navigateur avec API Rust et PostgreSQL locaux crée, dans l'éditeur,
deux tickets produit puis trois tickets techniques. Il vérifie les citations
conservées, les ajouts sans citation, les versions et cinq créations distinctes
chez les fournisseurs HTTP **[FICTIF]** Linear/GitHub. Il retire ensuite l'entrée
du milieu, consulte le lien version 3/index 2, valide la version 5 et vérifie
l'avertissement de nouvelle publication sans créer une sixième issue.

Résultat : **un scénario navigateur réussi en 33,2 s**, incluant viewport 390 px,
focus, navigation clavier et axe WCAG 2 A/AA et 2.1 AA sans violation. La capture
mobile `/tmp/t19-ticket-editor-mobile.png` a été inspectée : champs, commandes,
informations de citations et sources restent lisibles sans débordement horizontal.

## Défauts trouvés et corrigés

- **TEE-R01, P2** : la sauvegarde redécoupait des critères multilignes historiques
  non modifiés. Ils sont désormais conservés exactement ; la régression est
  couverte et contre-vérifiée dans [la revue indépendante](review-2026-09-30.md).
- **TEE-R02, P2** : les sources dans une zone défilante d'un fieldset désactivé
  n'étaient pas parcourables au clavier. Le fieldset reste actif, les cases seules
  restent verrouillées, et la région nommée reçoit le focus visible. Le scénario
  vérifie Tab, End et le défilement effectif avant le contrôle axe complet.
- **TEE-R03/R04, P2** : changer le type de livrable pouvait perdre la consigne
  puis un clic immédiat pouvait envoyer l'ancien type. La saisie est désormais
  stable par acteur/société/projet, les reçus restent séparés par type et le
  choix est contrôlé avant toute admission. Les tests rouges puis verts et la
  contre-revue couvrent aussi la création manuelle. La suite finale du snapshot
  compte **216 tests web réussis**, avec lint et construction réussis.

## Preuves techniques

- Qualité du snapshot : **209 tests web, 188 unités Rust, trois contrats
  synthétiques, 91 tests Python réussis** ; dix tests Rust nécessitant une base
  ignorés dans la seule passe unitaire. Format, liens, lint, Clippy strict et
  constructions web/API réussis. Après le dernier correctif d'accessibilité,
  les 209 tests web, lint et construction passent de nouveau.
- Sur l'image officielle `supabase/postgres:17.11.0.002`, version effective
  `170011` attestée : 23 migrations historiques, 32 assertions pgTAP et les
  suites d'intégration du socle réussies ; TLS avec certificats/nom d'hôte,
  restauration de 60 tables et 143 politiques, Auth API et navigateur à deux
  comptes, puis parcours de sources Notion/Linear réussis.
- La première recette s'est arrêtée sur TEE-R02. Après correction, le parcours
  tickets est rejoué ; ses cinq créations et reçus sont vérifiés côté serveur.
  La passe suivante des quatre régressions navigateur métier a réussi trois
  scénarios et révélé une perte de saisie après changement du type de livrable.
  Une reproduction ciblée a aussi trouvé une fenêtre où un clic immédiat pouvait
  encore envoyer l'ancien type. Après correction et contre-revue, **les quatre
  parcours passent en 47,2 s** sur le snapshot final, dont réponse perdue/reprise,
  génération/édition/validation, graphe, historique et relais PM/technique.

Journaux privés : `.run/t19-ops-quality.log`, `.run/t19-web-final.log`,
`.run/t19-pg17-11-integration.log`, `.run/t19-ticket-final.log` et
`.run/t19-generation-final.log`. Les 216 unités finales, lint et construction
sont consignés dans `/tmp/t19-generation-{all-units,lint,build}.log`.
Les volumes de chaque stack jetable sont nettoyés par leurs gardes propres.

## Limites

Ce résultat qualifie le logiciel local de ce lot. La CI doit encore qualifier
son commit exact. Il ne prouve ni génération multi-ticket par un fournisseur IA
réel, ni comptes Linear/GitHub réels, ni SMTP externe, ni déploiement HTTPS,
ni restauration sur une infrastructure séparée. T18 et CC-G2/G3/G4 restent ouverts.
