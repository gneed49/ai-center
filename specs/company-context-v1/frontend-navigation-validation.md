# Navigation société et agents — validation locale du 21 septembre

## Livraison

- Navigation claire conforme à la direction documentée : Vue d’ensemble,
  Graphe, Agents, Projets, À vérifier, Équipe, Connexions.
- Entreprises autorisées, identité et rôle réels, changement de société et
  contexte projet visible ; aucune identité de démonstration dans le produit.
- Vue d'ensemble sans indicateur moteur inventé ni somme de versions de graphe.
- Catalogue d'agents lu depuis l'API, choix société/projet et profil, vraie
  création de conversation, reprise d'une session existante et historique.
- Accès lecteur sans création ; lead technique orienté vers le handoff existant.
- Routes société/graphe/agents et liste projets intégrées ; bootstrap société
  accessible depuis le sélecteur d'entreprise après connexion.
- Pages chargées à la demande : bundle JavaScript principal mesuré à 275,35 kB
  minifié / 84,66 kB gzip, contre environ 570 kB avant découpage.

## Preuves

Au terme de ce lot, la suite web collective compte **76 tests réussis dans 17
fichiers**. Les huit tests ajoutés pour la navigation et les agents vérifient
le scope des requêtes, les reprises, la stabilité de commande, le relais
technique, les permissions lecteur, un projet inaccessible, l'identité réelle
et le changement d'entreprise. Fixtures uniquement `[FICTIF]`.

Lint web, compilation TypeScript, build Vite et contrôle des différences
réussis. Les concepts `company-graph.png` et `project-agent.png` ont été lus
visuellement avant implémentation ; leur présence ne constitue pas une preuve
de fidélité du rendu actuel.

## Limites

Recette navigateur et comparaison de captures encore à effectuer dans le lot
d'intégration global. Les tests Playwright historiques doivent recevoir les
nouvelles routes société dans leurs fixtures. Le nouveau catalogue ouvre la
conversation existante ; la composition complète conversation/livrable du
concept reste un travail distinct. Aucun déploiement ni fournisseur réel
n'a été exercé pendant ce lot.
