# CC-T11 — Régulation commune et arrêt de l’automatisation

Un contrôle persistant par société active ou suspend les nouvelles générations
et publications. Chaque modification incrémente une génération : arrêter puis
réactiver ne doit pas ressusciter un appel ancien. Seul un propriétaire peut
modifier ce contrôle ; les membres peuvent lire son état et l’usage agrégé.

Tous les moteurs sélectionnés (serveur, connexion personnelle, abonnement local,
simulation et Steward) passent par un wrapper unique. Avant l’appel, une
réservation durable est créée sous verrou société et vérifie le quota horaire et
la concurrence, pour la société et pour l’acteur au sein de cette société.
Les plafonds par acteur évitent qu’un seul membre monopolise toute la capacité.
L’usage personnel est visible séparément du total société. Les deux plafonds
s’appliquent simultanément et persistent après recréation d’un wrapper.
La réservation expire après une borne dure si le processus disparaît.
Pendant l’appel, un contrôle court vérifie suspension, génération et adhésion ;
le futur local est abandonné après suspension. Cela ne rétracte pas un appel
accepté par le fournisseur, dont le coût peut rester inconnu.

Les réservations ne contiennent ni contexte, ni réponses, ni clés. Les limites
sont configurables par l’opérateur dans des plages bornées ; le réglage société
ne peut les augmenter. La publication déjà envoyée conserve son reçu et une
réponse ambiguë demeure à vérifier. Le worker ne consomme pas de nouveau travail
pendant une suspension. Les refus temporaires restent rejouables avec la même
identité de commande.

Preuves : concurrence multi-appels et limites avant transport sur PostgreSQL,
arrêt d’un appel bloqué, reprise après réactivation, persistance après nouveau
wrapper/processus, refus viewer/editor du contrôle et isolement des sociétés.
Les doubles sont explicitement synthétiques ; aucun coût fournisseur réel n’est
engagé par ces tests.

Le bootstrap privé exige une identité opérateur autorisée (`AI_CENTER_COMPANY_CREATORS`)
ou un rôle propriétaire déjà accepté. L’identité Auth seule, y compris issue
d’une inscription directe à l’API publique Auth, ne donne pas accès à la
création d’une société financée par le moteur serveur. La capacité est exposée
avant le formulaire pour guider les nouveaux membres vers leur invitation.
