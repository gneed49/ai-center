# Campagne comparative privée

Ce dossier versionne le protocole `ContextPack` contre `full_dump`, ses schémas
et ses garde-fous. Il ne contient aucun corpus, prompt concret, résultat brut,
URL de dépôt ou secret.

Deux outils distincts sont disponibles :

- [alpha-eval.py](../../../scripts/alpha-eval.py) initialise le manifest,
  valide les métriques expurgées et produit le rapport final ;
- [alpha-live-eval.py](../../../scripts/alpha-live-eval.py) prépare les plans
  A/B et peut exécuter les appels Responses API, mais reste en dry-run tant que
  deux confirmations explicites ne sont pas fournies.

Le runner utilise la Responses API avec `store:false` et un JSON Schema strict,
conformément à la
[référence officielle OpenAI](https://developers.openai.com/api/reference/cli/resources/responses/methods/create).
`store:false` signifie que la réponse n'est pas enregistrée comme ressource
API récupérable ; ce document ne le présente pas comme une garantie générale
de rétention zéro.

## Invariants de sécurité

- Tous les inputs et outputs live doivent être sous
  `.run/alpha-context-proof/`. Le runner refuse tout autre chemin.
- `.run/` est ignoré par Git. Les fichiers privés sont écrits avec le mode
  `0600`.
- Le runner ne charge aucun fichier `.env`, ne lit jamais `.env.local` et ne
  journalise jamais la clé ou les payloads.
- La clé n'est lue depuis `OPENAI_API_KEY` qu'après `--execute` et le token de
  confirmation exact. Un dry-run n'accède pas à cette variable.
- Aucun retry réseau automatique n'est effectué : une réponse perdue peut être
  facturée. La réservation reste alors bloquée jusqu'à réconciliation.
- L'URL fournisseur est fixée à `https://api.openai.com/v1/responses` ; elle
  n'est pas configurable par le corpus.
- Le scanner local bloque les noms de champs sensibles, les motifs de token,
  les clés privées et les adresses e-mail détectables. Il complète, mais ne
  remplace pas, la revue humaine du corpus.
- Les logs ne contiennent que des identifiants pseudonymes, hashes, compteurs,
  statuts et coûts. Les erreurs HTTP ne recopient jamais le body fournisseur.
- Aucun test ou job CI n'exécute le chemin live.
- Cette campagne couvre uniquement le web desktop et Tauri Linux ; aucun mobile
  ou Android.

## Fichiers locaux

```text
.run/alpha-context-proof/
├── campaign.json                # manifest pseudonyme
├── live-config.json             # modèles, prix gelés, contrat sélectionné
├── private-cases.json           # tâches et contextes réels autorisés
├── plans/
│   ├── calibration.json
│   ├── main.json
│   └── reserve.json
├── live/
│   ├── budget-ledger.jsonl      # réservations append-only
│   ├── events.jsonl             # événements expurgés
│   ├── raw/                     # réponses fournisseur privées
│   └── blind/                   # sorties A/B sans nom de condition
├── runs.jsonl                   # métriques expurgées
├── evaluations.jsonl
└── report.json
```

Contrats versionnés :

- [campaign.schema.json](campaign.schema.json) ;
- [live-config.schema.json](live-config.schema.json) ;
- [private-cases.schema.json](private-cases.schema.json) ;
- [live-plan.schema.json](live-plan.schema.json) ;
- [budget-event.schema.json](budget-event.schema.json) ;
- [run.schema.json](run.schema.json) ;
- [evaluation.schema.json](evaluation.schema.json) ;
- [report.schema.json](report.schema.json).

## 1. Préparer le manifest privé

```bash
mkdir -p .run/alpha-context-proof
python3 scripts/alpha-eval.py init \
  --output .run/alpha-context-proof/campaign.json
python3 scripts/alpha-eval.py validate \
  --manifest .run/alpha-context-proof/campaign.json \
  --allow-placeholders
```

Le template contient exactement trois projets pseudonymes, douze tâches et
soixante paires équilibrées. Avant toute préparation live :

1. remplacer toutes les valeurs `REPLACE_…` ;
2. confirmer le consentement et l'absence de secret, donnée personnelle ou
   donnée sensible pour chaque corpus ;
3. calculer un fingerprint SHA-256 de chaque corpus privé ;
4. annoter les faits critiques, éléments pertinents et exclusions avec des
   identifiants, jamais avec le contenu brut ;
5. valider sans `--allow-placeholders`.

## 2. Initialiser le contrat live

```bash
python3 scripts/alpha-live-eval.py init \
  --manifest .run/alpha-context-proof/campaign.json \
  --config .run/alpha-context-proof/live-config.json \
  --cases .run/alpha-context-proof/private-cases.json
```

Compléter ensuite les deux fichiers privés :

- un ou deux modèles maximum pour la calibration ;
- une capture datée des prix input/output officiels, en USD par million de
  tokens ;
- les payloads `context_pack` et `full_dump` correspondant exactement à la
  même tâche ;
- les UUID sources autorisés et le hash du ContextPack ;
- un petit sous-ensemble `calibration_case_ids` couvrant au moins un handoff et
  une paire ;
- `reserve_case_ids` uniquement pour les cas déclarés instables avant reprise.

Configurer aussi une limite de dépense de 100 USD sur le projet OpenAI dédié.
Le ledger local applique `10 + 70 + 20`, mais cette limite fournisseur reste la
seconde barrière contre une erreur de prix configuré ou un appel hors runner.

## 3. Calibrer au plus deux modèles

Créer le plan privé. Sans `--seed`, une graine cryptographique nouvelle est
générée et son commitment SHA-256 est enregistré :

```bash
python3 scripts/alpha-live-eval.py plan \
  --manifest .run/alpha-context-proof/campaign.json \
  --config .run/alpha-context-proof/live-config.json \
  --cases .run/alpha-context-proof/private-cases.json \
  --stage calibration \
  --output .run/alpha-context-proof/plans/calibration.json
```

Prévisualiser chaque job avant toute autorisation. Ce chemin ne lit pas la clé
et ne fait aucun appel réseau :

```bash
python3 scripts/alpha-live-eval.py run \
  --manifest .run/alpha-context-proof/campaign.json \
  --config .run/alpha-context-proof/live-config.json \
  --cases .run/alpha-context-proof/private-cases.json \
  --plan .run/alpha-context-proof/plans/calibration.json \
  --job-id job-REPLACE_FROM_PLAN \
  --runs .run/alpha-context-proof/runs.jsonl \
  --ledger .run/alpha-context-proof/live/budget-ledger.jsonl \
  --events .run/alpha-context-proof/live/events.jsonl \
  --raw-dir .run/alpha-context-proof/live/raw \
  --blind-dir .run/alpha-context-proof/live/blind
```

Le chemin facturable doit recevoir la clé par le gestionnaire de secrets du
processus. Ne jamais la coller dans la commande, le terminal partagé ou la
conversation, et ne pas `source` le fichier `.env.local` :

```bash
python3 scripts/alpha-live-eval.py run \
  ...mêmes arguments... \
  --execute \
  --confirm RUN_OPENAI_ALPHA_CONTEXT_PROOF
```

Avant l'envoi, le runner réserve le coût maximum de l'appel à partir :

- du double de la taille UTF-8 de toute la requête plus un overhead gelé ;
- de `max_output_tokens` ;
- des prix input/output gelés.

Une réservation est refusée si elle ferait dépasser 10 USD de calibration,
70 USD de campagne principale, 20 USD de réserve ou 100 USD au total. Le coût
réel calculé depuis `usage` remplace ensuite la réservation. Les réservations
pendantes continuent à consommer leur maximum.

Afficher l'état conservateur du budget :

```bash
python3 scripts/alpha-live-eval.py budget \
  --ledger .run/alpha-context-proof/live/budget-ledger.jsonl
```

Après tous les jobs de calibration, le runner gèle automatiquement le modèle le
moins coûteux ayant obtenu : 95 % de sorties structurées valides, 100 % de
sources valides, 100 % des faits critiques, 85 % de rappel du contexte
pertinent et 70 % d'exactitude sur les contradictions du sous-échantillon :

```bash
python3 scripts/alpha-live-eval.py freeze-model \
  --manifest .run/alpha-context-proof/campaign.json \
  --config .run/alpha-context-proof/live-config.json \
  --runs .run/alpha-context-proof/runs.jsonl
```

Le contrat gelé enregistre le modèle, les versions runner/prompt/schéma et le
hash de tous les runs de calibration. Toute modification ultérieure invalide un
plan déjà produit.

## 4. Produire le plan A/B principal

```bash
python3 scripts/alpha-live-eval.py plan \
  --manifest .run/alpha-context-proof/campaign.json \
  --config .run/alpha-context-proof/live-config.json \
  --cases .run/alpha-context-proof/private-cases.json \
  --stage main \
  --output .run/alpha-context-proof/plans/main.json
```

Pour chaque handoff, le plan contient exactement trois répétitions de chacune
des conditions. L'ordre des conditions et leur label aveugle `A/B` sont
randomisés. Chaque paire de cohérence reçoit trois répétitions `context_pack`.
Avec le corpus minimal, le plan principal contient 252 appels : 72 handoffs et
180 classifications.

L'évaluateur reçoit uniquement les fichiers `live/blind/`. Le fichier plan,
`runs.jsonl` et le ledger révèlent la condition et restent hors de sa vue.

## 5. Réserve et reprise sûre

Les répétitions quatre et cinq sont planifiées uniquement pour les
`reserve_case_ids` dont l'instabilité a été constatée :

```bash
python3 scripts/alpha-live-eval.py plan \
  --manifest .run/alpha-context-proof/campaign.json \
  --config .run/alpha-context-proof/live-config.json \
  --cases .run/alpha-context-proof/private-cases.json \
  --stage reserve \
  --output .run/alpha-context-proof/plans/reserve.json
```

Après timeout, déconnexion ou réponse perdue, ne jamais relancer le job : la
réservation reste `pending`, car l'appel peut avoir été facturé. Si un crash est
survenu après l'écriture de `runs.jsonl` mais avant le settlement, cette commande
reconstruit uniquement le settlement prouvé :

```bash
python3 scripts/alpha-live-eval.py reconcile-ledger \
  --ledger .run/alpha-context-proof/live/budget-ledger.jsonl \
  --runs .run/alpha-context-proof/runs.jsonl
```

Une réservation sans métrique persistée nécessite une réconciliation manuelle
avec la consommation du projet fournisseur ; le runner refuse de la libérer ou
de la réessayer automatiquement.

## 6. Agréger la preuve

```bash
python3 scripts/alpha-eval.py summarize \
  --manifest .run/alpha-context-proof/campaign.json \
  --runs .run/alpha-context-proof/runs.jsonl \
  --evaluations .run/alpha-context-proof/evaluations.jsonl \
  --output .run/alpha-context-proof/report.json
```

Le propriétaire note tous les comparatifs et le second évaluateur au moins
25 %. L'ordre est aveugle et aucune égalité textuelle n'est utilisée comme
oracle. Le rapport échoue tant que l'échantillon, le budget ou un seuil est
incomplet. Seul un rapport agrégé relu et expurgé peut rejoindre le registre de
release.

## Tests hors ligne

```bash
python3 -m unittest discover -s scripts/tests -v
python3 -m py_compile scripts/alpha-eval.py scripts/alpha-live-eval.py
```

Ces tests valident la limite de deux modèles, les trois répétitions A/B, le
contrat `store:false` + JSON Schema strict, les réservations concurrentes, le
stop budget, la réconciliation et l'expurgation. Ils n'utilisent pas de clé et
n'effectuent aucun appel réseau.
