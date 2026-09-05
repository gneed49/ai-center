# Alpha Context Proof

> Statut : active
>
> Jalon cible : `v0.2.0-alpha.1`
>
> Périmètre client : web desktop et Tauri Linux uniquement

## Intention

AI Center doit prouver qu'une équipe peut conserver la maîtrise d'un contexte
partagé tout au long d'un travail réalisé dans ses outils existants. Le produit
ne devient ni un IDE, ni un runner, ni un générateur de code. Il transforme une
intention en connaissances confirmées, compile un contexte adapté, transmet ce
contexte à un outil externe puis réintègre références et preuves pour contrôler
la cohérence et la couverture.

La preuve doit être obtenue avec des données réelles non sensibles sur trois
projets, dont AI Center, et comparée à un workflow témoin qui transmet l'ensemble
du contexte sans sélection.

## Frontières non négociables

- GitHub et les outils de travail restent canoniques pour leurs objets.
- AI Center ne crée ni branche, ni commit, ni pull request, ni commentaire.
- Aucun code ou diff distant n'est exécuté ou importé par le connecteur GitHub.
- Aucun transcript brut n'est inclus par défaut dans un `ContextPack`.
- Aucun écran, build, test, viewport ou gate Android, iOS ou mobile n'appartient
  à ce jalon.
- Le moteur déterministe reste un double de test ; il ne constitue pas une
  preuve de valeur réelle.

## Vocabulaire de preuve

Chaque affirmation de validation utilise exactement l'un des statuts suivants.

| Statut                       | Sens                                                                                                    |
| ---------------------------- | ------------------------------------------------------------------------------------------------------- |
| `Vérifié`                    | Le comportement a été reproduit pendant la validation courante et une preuve localisable est indiquée.  |
| `Présent mais non reproduit` | Le code ou l'artefact existe, mais son comportement n'a pas été reproduit dans l'environnement courant. |
| `Déclaré`                    | Le résultat est rapporté par une source antérieure, sans preuve courante suffisante.                    |
| `Futur`                      | Le comportement est attendu par cette spécification, mais n'est pas encore livré.                       |

Un statut `Vérifié` doit toujours indiquer la commande, le scénario, la date et
l'artefact de preuve. La présence de code, un ancien rapport ou une sortie
déterministe ne suffit pas.

## Exigences

### Positionnement et connaissance

| ID      | Exigence                                                                                                        | Preuve d'acceptation                                                  |
| ------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| ACP-001 | AI Center est présenté et testé comme plan de contrôle contextuel, jamais comme outil de production.            | Revue des écrans, contrats et documentation ; aucune mutation GitHub. |
| ACP-002 | Une intention produit peut devenir une ou plusieurs connaissances atomiques confirmées.                         | Parcours UF-01 et UF-02 avec données persistées.                      |
| ACP-003 | Toute connaissance confirmée possède une version, une provenance, un auteur, un workspace et une graph version. | Assertions DB/API après confirmation et reload.                       |
| ACP-004 | Rejeter une proposition ne crée aucune connaissance confirmée.                                                  | Parcours bloqué UF-02 et audit associé.                               |
| ACP-005 | Réviser une connaissance crée une nouvelle version sans réécrire l'historique.                                  | Parcours UF-08 et contrôle des versions.                              |
| ACP-006 | Les affirmations de validation utilisent les quatre statuts de preuve définis ci-dessus.                        | Revue documentaire et rapport de release.                             |

### Gate, ContextPack et handoff

| ID      | Exigence                                                                                                                                      | Preuve d'acceptation                                  |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| ACP-010 | Le gate Produit bloque tant que les connaissances obligatoires courantes manquent.                                                            | UF-03, états bloqué puis passé.                       |
| ACP-011 | Le gate est évalué pour la graph version courante ; un ancien résultat ne peut pas autoriser un handoff.                                      | Test de révision entre gate et handoff.               |
| ACP-012 | Le Context Compiler inclut les obligations par règles déterministes et classe seulement les candidats optionnels avec l'IA.                   | Tests unitaires et trace de sélection.                |
| ACP-013 | Le budget initial de compilation est de 12 000 tokens, configurable et persisté.                                                              | Test sous/sur budget et lecture API.                  |
| ACP-014 | Chaque candidat conserve une décision `included` ou `excluded`, un motif codifié, une explication et son ordre.                               | Inspection API/DB du pack.                            |
| ACP-015 | Tous les identifiants de sources produits par un modèle sont validés côté serveur et rattachés au bon projet.                                 | Faux UUID, UUID d'un autre projet et sortie valide.   |
| ACP-016 | Un ContextPack est immutable et porte sa graph version, sa compiler version, son mode de sélection, son hash et les versions sources exactes. | Mutation refusée et export stable.                    |
| ACP-017 | Toute modification d'une source rend stale chaque pack dépendant, sans modifier son contenu historique.                                       | UF-08 avec comparaison des hashes.                    |
| ACP-018 | Une session Tech consomme exclusivement le ContextPack explicite du handoff.                                                                  | Test avec connaissance hors pack absente du résultat. |
| ACP-019 | Un handoff refuse un pack stale, hors projet ou produit sur une graph version antérieure.                                                     | Trois réponses `409`/`404` ciblées.                   |
| ACP-020 | Le dernier handoff et son pack sont restaurés après reload.                                                                                   | Playwright UF-05.                                     |
| ACP-021 | Le pack est exportable en JSON canonique et Markdown avec un digest SHA-256 identique pour un contenu identique.                              | Snapshot des deux formats et recalcul du digest.      |

### Fiabilité et isolation

| ID      | Exigence                                                                                                                  | Preuve d'acceptation                                                |
| ------- | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| ACP-029 | L'alpha équipe utilise un magic link Supabase et Axum vérifie signature JWKS, `kid`, `iss`, `exp` et `sub`.               | JWT valide, expiré, mauvais issuer et mauvais kid.                  |
| ACP-030 | Toute ressource est limitée au workspace et au projet portés par la requête authentifiée.                                 | Matrice RLS et isolation-routing.                                   |
| ACP-031 | Les rôles `owner`, `editor` et `viewer` ont des permissions explicitement testées.                                        | Tests grants + RLS par commande.                                    |
| ACP-032 | Les commandes mutantes exigent authentification, workspace, identifiant client et `Idempotency-Key`.                      | Tests HTTP des en-têtes manquants et valides.                       |
| ACP-033 | Même clé et même corps rejouent le résultat ; même clé et corps différent retournent `409`.                               | Tests de concurrence et replay.                                     |
| ACP-034 | Timeout, réponse perdue, double clic et retry ne dupliquent aucune mutation.                                              | UF-10 et assertions DB.                                             |
| ACP-035 | Un appel fournisseur n'est jamais maintenu dans une transaction DB ouverte.                                               | Test d'intégration avec fournisseur lent et inspection des verrous. |
| ACP-036 | Le mode réel ne bascule jamais silencieusement vers le moteur déterministe.                                               | Fournisseur indisponible → erreur explicite.                        |
| ACP-037 | Chaque appel de modèle conserve statut, modèle, versions de prompt/schéma, usage, latence, tentatives et erreur nettoyée. | Inspection d'un `model_run` réussi et échoué.                       |
| ACP-038 | L'outbox possède lease, retries bornés, backoff, compteur et état final.                                                  | Tests crash/reprise et concurrence.                                 |
| ACP-039 | Responses API utilise une sortie structurée et `store:false` ; aucune rétention zéro n'est affirmée sans contrat séparé.  | Inspection du faux provider et de la documentation de release.      |

### Steward et obsolescence

| ID      | Exigence                                                                                     | Preuve d'acceptation                                          |
| ------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| ACP-040 | Le steward classe une paire candidate en `contradiction`, `compatible` ou `ambiguous`.       | Corpus annoté et rapport de campagne.                         |
| ACP-041 | Une évaluation conserve les deux versions sources, son fingerprint, son run et sa confiance. | Inspection DB/API.                                            |
| ACP-042 | Plusieurs contradictions peuvent coexister et aucune source inconnue n'est acceptée.         | Test multi-insights et source forgée.                         |
| ACP-043 | `accept`, `dismiss` et `resolve` ont des effets distincts et audités.                        | UF-07/UF-08.                                                  |
| ACP-044 | `resolve` exige une révision ou supersession atomique d'au moins une connaissance.           | Requête sans mutation refusée ; transaction complète réussie. |
| ACP-045 | L'invalidation cible les projections dépendantes ; le fallback global est visible et audité. | Graphe avec deux branches dépendantes.                        |
| ACP-046 | Aucune génération n'est possible depuis une session ou un pack stale.                        | Tests API et UI sans override.                                |

### Références externes et preuves

| ID      | Exigence                                                                                                              | Preuve d'acceptation                              |
| ------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| ACP-050 | Une référence externe possède une identité stable, des observations append-only et un état de synchronisation.        | Import puis deux refreshes.                       |
| ACP-051 | Le premier connecteur est une GitHub App privée strictement read-only.                                                | Revue des permissions et test négatif d'écriture. |
| ACP-052 | Seules les URLs `github.com` sont acceptées et les appels API sont reconstruits depuis des identifiants validés.      | Tests SSRF, redirection et URLs ambiguës.         |
| ACP-053 | L'import est limité aux métadonnées explicitement autorisées ; aucun contenu de fichier ou diff n'est stocké.         | Inspection des observations et logs.              |
| ACP-054 | Une preuve importée commence `candidate` et devient `valid` uniquement après validation humaine.                      | UF-09.                                            |
| ACP-055 | Changement de head SHA, suppression ou perte d'accès rend la preuve `stale` ou `unavailable` sans perte d'historique. | Refresh GitHub simulé puis inspection.            |
| ACP-056 | Une preuve valide recalcule la couverture des exigences qu'elle satisfait.                                            | UF-09, avant/après.                               |

### Validation et promotion

| ID      | Exigence                                                                                                               | Preuve d'acceptation                    |
| ------- | ---------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| ACP-070 | Les dix parcours définis ci-dessous couvrent succès, blocage, erreur, retry et reload.                                 | Rapport Playwright desktop.             |
| ACP-071 | La certification couvre Chromium 1440×900, Chromium 1024×768, Firefox desktop et un smoke Tauri Linux.                 | Artefacts CI/pré-alpha.                 |
| ACP-072 | La campagne réelle couvre exactement trois projets autorisés, au moins 12 handoffs et 60 paires équilibrées.           | Manifest validé localement.             |
| ACP-073 | Les conditions ContextPack et dump complet utilisent le même modèle, contrat et tâche avec au moins trois répétitions. | Rapport A/B.                            |
| ACP-074 | Le budget total de campagne ne dépasse jamais 100 USD.                                                                 | Agrégateur et arrêt externe configuré.  |
| ACP-075 | Les seuils quantitatifs de `validation.md` sont tous franchis avant l'alpha équipe.                                    | Rapport final `passed: true`.           |
| ACP-076 | Aucun secret, contenu sensible ou donnée personnelle n'entre dans le dépôt ou les artefacts publics.                   | Secret scan et revue du manifest privé. |
| ACP-077 | Aucun test, build, viewport ou gate mobile n'est lancé par la CI ou la campagne.                                       | Revue des workflows et logs.            |

## Les dix parcours de certification

| ID    | Parcours                      | État initial                           | Action principale                                           | Mutation attendue                                             | Preuve                                           | Blocage / erreur                  | Retry et reload                                      |
| ----- | ----------------------------- | -------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------ | --------------------------------- | ---------------------------------------------------- |
| UF-01 | Créer un projet               | Utilisateur membre, aucun projet       | Saisir nom et intention                                     | Projet, nœuds et audit créés dans le workspace                | Snapshot project-scoped                          | Validation `422`, panne API       | Double clic idempotent ; projet visible après reload |
| UF-02 | Confirmer la connaissance     | Session Produit active                 | Envoyer un message, confirmer ou rejeter les propositions   | Versions confirmées seulement pour les propositions acceptées | Provenance, graph version, audit                 | Provider `503`, sortie invalide   | Saisie préservée ; replay sans doublon               |
| UF-03 | Passer le gate                | Connaissances incomplètes              | Évaluer, compléter, réévaluer puis générer le Feature Brief | Gate courant et livrable versionné                            | Raisons du blocage puis état `passed`            | Gate ancien/stale refusé          | Même état après reload                               |
| UF-04 | Compiler et exporter          | Gate courant passé                     | Compiler, inspecter les inclusions/exclusions, exporter     | ContextPack immutable et export digéré                        | Sources exactes, raisons, budget, hash           | Budget dépassé, source forgée     | Retry retourne le même résultat logique              |
| UF-05 | Réaliser le handoff           | Pack courant du projet                 | Créer le handoff et ouvrir la session Tech                  | Handoff et session liés au pack                               | Route et session restaurées                      | Pack stale/hors projet refusé     | Dernier handoff restauré après reload                |
| UF-06 | Produire plan et couverture   | Session Tech liée au pack              | Générer un plan puis inspecter la couverture                | Livrable, lignées et coverage items                           | Exigences sourcées, trous visibles               | Provider indisponible, pack stale | Retry sans second livrable concurrent                |
| UF-07 | Détecter et décider           | Deux connaissances candidates          | Attendre le steward, ouvrir l'insight, accept/dismiss       | Assessment et décision audités                                | Deux versions sources et fingerprint             | Résultat ambigu, source invalide  | Inbox cohérente après reload                         |
| UF-08 | Résoudre et recompiler        | Contradiction ouverte                  | Réviser une source via `resolve`, puis recompiler           | Nouvelle version, nouvelle graph version et dépendances stale | Chaîne auditée de l'ancien au nouveau pack       | Conflit optimiste `409`           | Réessai depuis la dernière version                   |
| UF-09 | Importer une preuve GitHub    | PR/commit autorisé dans un dépôt privé | Attacher, refresh, valider la preuve                        | ExternalReference, observation, preuve et couverture          | URL canonique, SHA, checks et validation humaine | 401/403/404/429/5xx, SSRF         | ETag/idempotence ; perte d'accès conservée           |
| UF-10 | Reprendre sans perte ni fuite | Deux projets et plusieurs membres      | Offline, reconnexion, changement de projet et reload        | Aucune mutation parasite                                      | Historique durable et isolation                  | URL forgée, réponse perdue        | Pas de duplication ni mélange de cache               |

## Critères de sortie

Le jalon sort uniquement lorsque tous les critères ACP P0 sont `Vérifié`, que la
CI desktop est verte et que le rapport comparatif réel passe tous ses seuils.
Une fonctionnalité présente, déclarée ou démontrée uniquement en mode
déterministe ne peut pas être utilisée pour promouvoir l'alpha équipe.
