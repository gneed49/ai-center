# Plan de raccordement du contexte — T17

> 2026-09-24 — préparation concrète, **aucune implémentation pipeline dans ce lot
> documentaire**. Feu vert de root requis après commit T16. Références :
> [spec](spec.md), [contrat API](api-contract.md), [plan](plan.md),
> [revue de conception](design-review.md). SQL, migrations, recette et clôture
> restent propriété de root ; lecteurs entrants et observations HTTP appartiennent
> au backend outils, interface à son propriétaire.

## 1. Vertical et types exacts

Une issue Linear explicitement rattachée produit une observation, proposée au PM
dans la portée autorisée. La réponse cite son UUID exact ; un brouillon structuré
conserve cette citation, est validé puis prévisualisé dans T16. Le pack du lead
conserve l'observation et sa couverture. Notion et les observations de publication
empruntent ensuite les mêmes consommateurs ; T17 n'est clos qu'avec les deux
familles et les scénarios d'autorité/fraîcheur.

Deux discriminants nouveaux, littéraux dans tous les contrats :
`tool_source_observation` et `publication_observation`. Les alias historiques de
connaissance/artefact restent inchangés à leurs frontières. Aucun `else => artifact`
ne peut absorber un type inconnu. Les nouvelles observations ont toujours
`entry_type=external_observation`, `trust=observed_external` et `mandatory=false`.
Leur titre, texte, statut métier ou validation humaine du livrable ne les changent
pas en règles confirmées.

La valeur de `version_public_id` du compilateur reste l'UUID exact d'observation ;
il est inutile de renommer ce champ sur tous les anciens packs. Le candidat ajoute
un discriminant explicite et les métadonnées bornées d'observation : fournisseur,
identité distante, référence ou publication, portée, version locale, dates,
couverture/omissions, hashes et provenance. `statement` est l'extrait retenu.
Les nouveaux champs sont compatibles avec la lecture des anciens packs : absence
de métadonnées externes ne doit jamais inventer une observation ou sa confiance.

Les sorties IA peuvent garder leurs listes d'UUID : le serveur possède le registre
exact des candidats envoyés et résout le type depuis ce registre, sous RLS. Un UUID
absent ou ambigu est refusé. Le modèle ne fournit ni type de table arbitraire ni
instantané substitutif. La provenance stockée expose aussi `kind` et `project_id`,
alias des champs `source_kind` et `source_project_public_id` nécessaires aux
consommateurs historiques.

## 2. Couture SQL minimale attendue de root

| Couture | Contrat consommé par le pipeline |
| --- | --- |
| `app.tool_source_observation_current(bigint) RETURNS boolean` | **Nom confirmé.** ID SQL exact de l'observation, invoker/RLS, sans mutation ni réseau. False si invisible/inconnue, projet inactif, référence retirée, head différent, observation indisponible, connexion désactivée/étrangère, capacité retirée ou révision actuelle non attestée. Partial disponible peut être éligible. |
| `app.publication_observation_current(bigint) RETURNS boolean` | Même nature. Appartenance au groupe sémantique courant disponible, connexion actuellement attestée/active de même société/fournisseur. Ne pas imposer `allow_existing_reads` aux publications. Legacy sans attestation n'est pas éligible avant relecture autorisée. |
| Projection SQL commune des observations de publication | Groupes **contigus** métier/couverture/identité ; premier ID/UUID canonique, version locale ordinale, projet/société via job, contenu et hashes, attestation actuelle du groupe. Une observation historique exacte garde son UUID propre ; A→B→A crée trois groupes. Nom et shape final à figer par root, pas de seconde implémentation Rust des groupes. |
| `context_pack_scope_sources` | Ajouter les deux types et deux FKs exclusives ; noms proposés `tool_source_observation_id`, `publication_observation_id`. Source publique/projet/société contrôlés ; décisions inclus/exclus existantes conservées. |
| `artifact_version_sources` | Les mêmes deux colonnes exclusives et types ; snapshot exact dans le champ existant, sans source remplacée par le head à la validation. |
| Validateurs / courant | Étendre `validate_scope_source_identity`, validateur de provenance d'artefact et `context_pack_scopes_current`. Seules les sources **incluses** gouvernent la fraîcheur persistante ; exclues restent un reçu de sélection. |
| Autorité de publication | `publication_observations.connection_id` + `connection_revision` nullable ensemble, immuables et hors hash métier, FK société. Une nouvelle lecture attestée peut rendre le groupe courant éligible sans réécrire son premier reçu legacy. |

Pour une source rattachée, l'autorité **courante** vient de
`tool_source_references.connection_id/connection_revision`, pas nécessairement des
colonnes historiques de l'observation. Un refresh identique peut réattester le même
head après rotation/rebind. Pour une publication, la projection expose de même
l'autorité attestée du groupe, même si son premier reçu avait une autre attestation
ou aucune. Le prédicat et la projection doivent employer exactement cette règle.

Les observations de publication n'ont pas de `project_id` : conserver la FK
observation/société puis valider le projet via le job ; ne pas inventer une colonne
projet redondante. Les trois tables historiques `context_pack_sources`,
`context_pack_selection_items`, `deliverable_sources` ne requièrent pas de nouvelles
colonnes pour ce vertical : les observations passent par les scoped sources et la
filiation existante du livrable vers son pack. Root complète aussi les FKs/steward,
graph endpoints, export et purge déjà inventoriés dans le plan principal.

## 3. Sélection et budgets sans accès plus large

Conserver les scopes actuels : projet, société et projets reliés par relations
confirmées, profondeur deux ; requête globale société selon son parcours existant.
Le projet est une frontière de contexte, pas une nouvelle ACL privée. Les deux
prédicats filtrent les candidats actifs sous les droits courants.

Le chat et la compilation partagent une projection d'observations, mais gardent
leur mode de sélection : pertinence de question pour le chat, candidats bornés
pour le compilateur. Au maximum **20 externes au total**, pas 20 de chaque type,
8 Kio UTF-8 par extrait et 64 Kio cumulés, dans le plafond existant de 160 000 octets
de contexte sérialisé et dans le budget de tokens du pack. Compter métadonnées et
provenance dans ce plafond ; tronquer à une frontière UTF-8, signaler la coupe.
Les règles société confirmées restent prioritaires et ne sont jamais évincées par
un titre externe prétendant être obligatoire.

Avant la sélection, dédupliquer l'objet `(projet,fournisseur,external_id)` entre
source rattachée et publication : observation éligible la plus récente puis ordre
stable type/UUID. Garder le type et l'UUID réels du candidat retenu ; ne pas fusionner
les provenances historiques. Les compteurs indiquent candidats éligibles, doublons,
exclusions par plafond/portée et extraits ; la sélection n'est pas exhaustive.

T17 couvre ici Notion/Linear. Les références GitHub et fichiers de corpus restent
le complément explicitement différé ; aucune nouvelle lecture GitHub, couverture
exhaustive du dépôt ou type distant inventé n'est inclus dans ce raccordement.

## 4. Autorité pendant un appel IA

`graph_version` est nécessaire à la cohérence du snapshot mais insuffisant pour
une révocation de connexion. Ajouter un petit module partagé, par exemple
`scope_context/authority.rs`, avec un type interne `ObservedAuthorityStamp` :

```text
source_kind, source_id, source_public_id, source_project_id,
reference_public_id | publication_public_id,
connection_public_id, connection_revision
```

Ce type n'est ni un secret ni une autorisation reçue du client. Il est produit par
une lecture jointe sous RLS du registre exact fourni au modèle. Le snapshot garde
ces stamps ; la branche pack les recharge depuis ses seules sources incluses.
L'empreinte d'entrée IA engage les identités/autorités publiques capturées en plus
du contenu. Ne pas stocker des identifiants SQL internes dans le JSON utilisateur.

1. **Avant IA** : contrôler courant/éligibilité et capturer l'autorité des
   observations effectivement envoyées. Le sélecteur de pack reçoit les candidats,
   donc son contrôle final porte sur tous ceux envoyés, même ensuite exclus.
   Un pack déjà compilé ne requiert que ses sources incluses. Transaction courte,
   aucun verrou conservé pendant le fournisseur.
2. **À chaque retour IA** : verrouiller les projets selon l'ordre trié existant,
   puis les advisory partagés des connexions triées (clé exacte commune avec
   save/disable propriétaire). Relire les autorités et les deux prédicats. Exiger
   même type/UUID, même connexion/révision et éligibilité toujours vraie. Garder ces
   verrous jusqu'au commit du résultat ou de son échec nettoyé.
3. **Inchangé vs rotation** : refresh identique sous même autorité ne provoque pas
   de conflit ; ne comparer ni `reference.revision`, ni last_checked, ni le dernier
   ID de reçu de publication. Une rotation/rebind pendant l'IA provoque un conflit
   même si une relecture identique a déjà réattesté la source avant le retour.
4. **Toutes les sorties** : aucun message, proposition, artefact, pack, livrable ou
   sortie brute de modèle ne doit être nouvellement persisté sur le chemin d'une
   autorité perdue. Les validateurs peuvent travailler en mémoire ; leurs chemins
   d'échec écrivant `model_runs.output` passent aussi par le garde. Clôturer avec
   code sûr/métadonnées disponibles et `output=None` lorsque ce garde échoue ;
   conserver le helper étroit d'annulation après perte d'appartenance déjà livré.
   Aucun nouvel appel IA automatique pour masquer ce conflit.
5. **Deux appels du plan technique** : contrôler après génération et avant départ
   du reviewer de couverture, puis après couverture. Une connexion devenue invalide
   entre les deux appels empêche le second départ. Le run déjà achevé sous autorité
   valide reste historique ; le livrable n'est pas committé si le second contrôle
   échoue.

Ne pas prendre un nouveau projet après advisory connexion. Si un chemin final
résout plusieurs scopes, son jeu de projets doit être connu avant ces verrous.
La capture actuelle peut être une requête jointe sans verrou longue durée ; la
finalisation, elle, doit sérialiser avec la rotation jusqu'au commit. Le booléen
SQL seul ne remplace pas cette exclusion de mutation.

## 5. Fichiers du lot pipeline et changement minimal

| Fichier / fonction | Changement ciblé |
| --- | --- |
| `apps/server/src/context.rs` | Candidat/selection avec origine explicite et métadonnées externes ; `is_contract_required` refuse toujours les deux origines observées. Contenu/provenance du pack garde type, confiance, dates, couverture et UUID. Mettre à jour version du compilateur, pas les anciens packs. |
| `scope_context.rs` | Étendre `SourceCandidate`, `ScopeSnapshot`, `load`, `persist`, sources de pack ; ajouter capture d'autorité séparée des graph stamps. Deux bindings FK supplémentaires dans scoped sources. |
| `scope_context/chat.sql`, `chat.rs` | Ajouter projection des deux types, compteurs et limites communs ; remplacer les branches binaires connaissance/artefact par un match exhaustif. SQL trie avant bornage pour éviter une première fenêtre arbitraire. |
| Nouveau `scope_context/observations.rs` + projection SQL si utile | Projection/DTO canonique bornés, dédup et troncature UTF-8. Réutilisé par chat, compilation et résolution de provenance ; pas de logique fournisseur/HTTP dans ce module. |
| Nouveau `scope_context/authority.rs` | Capture depuis sources exactes / pack inclus et garde final projets puis advisory connexions. Pas de droit RLS élargi, pas de fonction privilégiée pour lire le contenu. |
| `service.rs::send_message_command` | Garder le snapshot d'autorité dans les deux branches chat direct/pack ; le vérifier avant écritures de réussite **et de sortie invalide**. Métadonnées de réponse conservent types/extraits/omissions. |
| `service.rs::compile_context_pack_command` et handoff | Sélecteur voit les deux types ; autorité des candidats vérifiée au retour ; persistance exacte dans scoped sources. À l'utilisation du pack, `pack_current` inclut les nouveaux prédicats et la fraîcheur incluse. |
| `service.rs::generate_technical_plan_command` | Ajouter le garde d'autorité aux deux phases IA et chemins d'erreur. Exigences obligatoires et preuves métier restent fondées sur les connaissances confirmées, pas les observations distantes. |
| `service/artifact_generation.rs` | Conserver autorité directe ou pack ; résoudre les UUID cités parmi le registre exact, étendre `exact_sources` aux deux types, puis garde avant persistance. Ne pas résoudre vers la capture courante en remplacement de celle utilisée. |
| `artifacts/{models,sources,validation}.rs` | Autoriser les deux kinds, résolution historique sous RLS, FKs exactes, `store` et `from_version` ; inclure les deux kinds parmi citations autorisées du draft typé. Les nouvelles snapshots gardent type/URL/date/couverture/hashes. |
| `artifacts/mod.rs` | Préserver snapshots des mêmes sources lors d'une édition/validation comme aujourd'hui ; exports Markdown affichent source observée/couverture, sans prétendue fraîcheur immuable. |
| `artifacts/conversion.rs` | Vérifier le pont via son context_pack exact ; ne pas aplatir le pack vers les heads externes. Pas de nouvelle FK directe de livrable si cette filiation suffit ; tester le chemin converti. |
| `artifacts/generation_contract.rs`, `agent.rs` | Ajuster les consignes « contexte confirmé » en distinguant savoir confirmé et données externes observées ; whitelist UUID serveur conservée. Aucun tool d'attachement/refresh/publication autonome. |
| `work_tools/ticket_projection.rs` | Petite couture à réserver au pipeline : accepter les deux kinds cités et rendre date/version/couverture dans le corps métier et l'aperçu exact. Ne pas modifier admission, worker, clé de publication ni corps historiques T16. |

Le Feature Brief historique produit déterministement les connaissances confirmées :
ne pas lui injecter silencieusement des textes externes comme règles métier. Le
parcours d'artefact généré et celui du pack sont les points d'intégration externes
de ce vertical. Le backend outils possède `work_tools/sources/**`, les DTO wire,
routes, connexions/advisory et attestation HTTP de publications ; root possède
graphe/steward/exports/maintenance et leurs SQL. Seule la couture `ticket_projection`
doit être explicitement réservée lors du feu vert pour éviter un conflit d'ownership.

## 6. Vérifications nécessaires avant clôture

- Unités : discriminants exhaustifs ; « règle obligatoire » dans une observation
  reste optionnel ; budgets UTF-8 et métadonnées ; 20 externes cumulés ; deux
  familles d'un même objet ne sont pas proposées deux fois ; sources inconnues
  refusées, injection traitée comme données.
- Intégration gardée root, fixtures **[FICTIF]** : attachement → chat citant l'UUID
  → artefact typé validé → aperçu T16 portant la même observation → pack/handoff
  → génération lead avec cette source exacte ; pipeline Notion et publication
  également, pas seulement la lecture entrante.
- Autorité : moteur factice contrôlé par barrières pendant l'appel, rotation,
  rebind, retrait/capacité, archive et révocation acteur ; zéro résultat persisté,
  run terminé sans sortie brute sous autorité perdue. Le même scénario avec sortie
  IA invalide vérifie le chemin d'erreur. Un refresh inchangé même autorité reste
  admissible ; une réattestation après rotation en vol reste conflit.
- Publications : legacy non attesté absent des candidats actifs mais lisible en
  historique ; refresh attesté lève cette exclusion ; unchanged conserve groupe,
  UUID canonique et pack courant ; A→B→A trois groupes ; ancien UUID exact conservé.
- Fraîcheur : observation incluse changée/retirée rend pack non courant ; observation
  exclue modifiée ne périme pas un pack existant ; édition/validation d'artefact
  conserve la snapshot citée, aucun refresh caché.
- Export, RLS, graphes, purge et intégration navigateur restent coordonnées par
  root. Vérifier la non-régression T16 et les anciens packs sans nouveaux champs.

Pas de nouveau moteur de recherche/vectorisation, file, tâche autonome, fournisseur
ou secret requis. Le code peut commencer après confirmation des coutures SQL et
ownership ; les résultats de ce document sont une préparation, pas un PASS T17.
