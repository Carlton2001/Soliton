# Echo Studio : étude de diffusion Wi-Fi

Date : 21 octobre 2026. Statut : **piste abandonnée à la demande de l'utilisateur**,
remplacée par l'ajout d'appareils Bluetooth via Windows et leur sélection dans
les sorties audio locales de RAP. Aucun service payant à exploiter pour RAP.
Le contenu ci-dessous documente l'étude historique : prototype local testé,
aucune lecture sur Echo Studio validée. Ne pas relancer proxy Amazon, tunnel ou
skill pour le parcours Bluetooth. Les autorisations expérimentales ci-dessous
décrivent les anciens essais ; elles ne constituent pas une demande de reprise.

L'utilisateur a explicitement autorisé le recours expérimental à une API Alexa
non officielle le 21 octobre 2026. Le §11.4 des spécifications a été actualisé
pour cette exception. Il ne dispose pas d'adresse HTTPS publique : le guide
prévoit un tunnel temporaire facultatif, sans achat de domaine.

Le prototype indépendant est dans [`tools/alexa-probe`](../tools/alexa-probe/README.md).
Il utilise `alexa-remote2` pour le contrôle depuis Node.js, une alternative à
`alexapy` dont le principe a été examiné pendant l'étude. Aucun runtime Node.js
n'est introduit dans l'application Rust. Le choix définitif de l'adaptateur et
son intégration GPUI restent conditionnés par l'essai sur le Studio.

## Besoin confirmé

- Echo Studio cylindrique (génération 2019).
- Aucun matériel supplémentaire : ni Fire TV ni récepteur externe.
- Lecture des fichiers de RAP par Wi-Fi, avec choix de l'enceinte depuis RAP.
- Éviter une conversion avec perte ; le transport Wi-Fi seul ne garantit pas cet objectif.
- Conserver la bibliothèque strictement en lecture seule.
- Aucun compte développeur Amazon/AWS, déploiement de skill, certificat ou
  tunnel à configurer manuellement par un utilisateur final de RAP.

## Distribution : distinction entre éditeur et utilisateur

Le montage du prototype est une procédure de développement. Il ne constitue
pas un parcours acceptable pour distribuer la fonctionnalité aux utilisateurs.
L'inscription Amazon Developer concerne le créateur de la skill. Une skill
commune, certifiée et publiée par l'éditeur RAP, permettrait aux utilisateurs
de l'activer et, selon l'architecture retenue, d'associer leur installation.
Cela ne doit pas demander un compte développeur à chaque utilisateur.

Cette voie nécessite une infrastructure HTTPS gérée et un mécanisme isolant
les installations, sessions et bibliothèques. Une association de compte ou de
session ne doit pas être confondue avec l'accès aux API non officielles de
pilotage des Echo : ce dernier reste un problème distinct. Le prototype, avec
sa configuration mono-utilisateur, sa clé unique et son tunnel temporaire, ne
doit pas être déployé comme service mutualisé sans conception spécifique.

La certification, l'exploitation et les coûts de cette infrastructure restent
à évaluer ; aucune publication ou souscription d'hébergement n'est autorisée
par cette seule étude. L'acceptation des fichiers lossless par le Studio reste
à tester avant d'investir dans cette intégration distribuable.

Références : [publication et certification](https://developer.amazon.com/en-US/docs/alexa/devconsole/test-and-submit-your-skill.html),
[parcours d'association de compte](https://developer.amazon.com/docs/alexaplus/account-linking/add-account-linking.html).

## Conclusion actuelle

Une skill peut faire lire un flux audio à une Echo. Des projets existants
combinent cette voie avec le pilotage non officiel du compte Alexa pour lancer
la skill sur une enceinte choisie. Cela fournit une architecture candidate sans
nouveau matériel, mais ne démontre ni sa fiabilité sur ce Studio ni la lecture
sans perte de ses fichiers FLAC.

La disponibilité d'une API publique permettant de découvrir les Echo puis de
déclencher directement une lecture depuis une application Rust/Windows n'a pas
été établie. Ne pas présenter cette absence de voie documentée comme une
impossibilité physique de diffusion Wi-Fi.

## Architecture candidate à expérimenter

1. RAP prépare un média explicitement autorisé dans son service de diffusion.
2. Une passerelle associe ce média à une session de lecture et à l'enceinte cible.
3. Le pilotage Alexa lance une skill sur cette enceinte.
4. La skill répond à Alexa avec une directive `AudioPlayer.Play` et l'URL du média.
5. L'Echo récupère le flux ; les événements de lecture de la skill permettent de
   distinguer démarrage confirmé, fin et erreur.

Le code de Music Assistant utilise `alexapy`, une passerelle `/ma/push-url` et
`run_custom` pour déclencher la skill. Son prototype exige un endpoint HTTPS
public pour la skill et pour les flux des appareils sans écran. Il ne liste pas
l'Echo Studio parmi ses appareils explicitement testés.

Cette architecture dépend d'Internet et du compte Amazon pour le contrôle.
Ne pas supposer que tous les octets audio passent nécessairement par le cloud :
My Media for Alexa annonce un transport audio local par défaut. Ce comportement
ne constitue pas une garantie pour une skill RAP.

## Qualité et autres voies

- La documentation `AudioPlayer` des custom skills liste AAC/MP4, MP3 et des
  conteneurs de playlists/streaming, sans garantir FLAC ou ALAC. L'acceptation
  des fichiers originaux reste à tester ; aucune conversion MP3/AAC ne doit être
  activée silencieusement pour obtenir artificiellement un test concluant.
- Les Music Skills disposent d'une fonction officielle Premium Audio avec
  plusieurs qualités, dont HD/UHD. C'est une voie différente des custom skills.
  Son accès, son utilisation en France et le pilotage depuis RAP restent à
  établir ; elle implique une infrastructure de fournisseur musical.
- Les Quick Links permettent une sélection d'appareil sur une page Amazon,
  mais exigent une custom skill publiée et certifiée. Leur liste documentée
  ne nomme pas explicitement le Studio. Ils ne démontrent donc pas un sélecteur
  natif RAP et une compatibilité Studio.
- L'interface caméra/interphone RTC utilise notamment Opus ou G.711 : elle ne
  démontre pas une voie musicale sans perte et n'est pas retenue pour ce besoin.
- Fire TV et les récepteurs optiques externes sont exclus par la demande.

## Conditions avant intégration dans RAP

1. Exception au §11.4 autorisée par l'utilisateur pour ce prototype ; conserver
   cette portée limitée lors de l'intégration ultérieure à l'application.
2. Disposer d'une skill de test sur le compte Amazon associé au Studio et d'un
   endpoint HTTPS utilisable. Ne pas publier de skill, configurer de tunnel ou
   exposer la bibliothèque automatiquement pendant cette étude.
3. Vérifier sur le Studio un fichier original lossless court, sans transcodage,
   puis les fréquences et profondeurs pertinentes. Vérifier l'URL demandée et
   les octets servis ; cela ne mesure pas le traitement interne de l'enceinte.
4. Vérifier le démarrage sur l'enceinte choisie, puis pause, reprise, arrêt,
   succession des pistes et déconnexion. Ne pas déduire le succès du seul
   retour HTTP de la commande : attendre les événements réels de lecture.
5. Si les essais réussissent, prévoir un adaptateur optionnel dans
   `rap-network`, indépendant de GPUI, avec délais bornés et états d'erreur
   explicites. Adapter le sélecteur existant seulement après cette validation.

Les tests locaux couvrent les octets d'origine, les requêtes HTTP partielles,
la révocation des accès, les événements tardifs, les délais, les contrôles de
compte/skill et le choix par identifiant exact. La connexion Amazon réelle et
l'énumération des appareils et capacités ont réussi sous Windows le 21 octobre
2026. Aucun essai audio sur une Echo ni aucune validation lossless de bout en
bout n'a été effectué. Aucun Rust n'a été modifié.

Après accord explicite pour transmettre uniquement les trois signaux
synthétiques via Cloudflare, le passage par un tunnel HTTPS réel a également
réussi : contenus SHA-256 inchangés, requêtes partielles et révocation vérifiées.
La skill de développement reste à créer ; ce contrôle de transport n'est pas
un essai de lecture sur l'enceinte.

## Sources examinées

- [Music Assistant : code du fournisseur Alexa](https://github.com/music-assistant/server/blob/main/music_assistant/providers/alexa/__init__.py)
- [Prototype : configuration](https://github.com/alams154/music-assistant-alexa-skill-prototype/blob/master/README.md)
- [Prototype : appareils testés](https://github.com/alams154/music-assistant-alexa-skill-prototype/blob/master/COMPATIBILITY.md)
- [Prototype : limitations](https://github.com/alams154/music-assistant-alexa-skill-prototype/blob/master/LIMITATIONS.md)
- [Amazon : exigences des flux AudioPlayer](https://developer.amazon.com/en-US/docs/alexa/custom-skills/use-long-form-audio.html)
- [Amazon : événements et directives AudioPlayer](https://developer.amazon.com/en-US/docs/alexa/custom-skills/audioplayer-interface-reference.html)
- [Amazon : Premium Audio pour Music Skills](https://developer.amazon.com/en-US/docs/alexa/music-skills/premium-audio-badging-drm.html)
- [Amazon : Quick Links](https://developer.amazon.com/en-US/docs/alexa/custom-skills/create-a-quick-link-for-your-skill.html)
- [Amazon : RTC et codecs](https://developer.amazon.com/docs/alexaplus/device-apis/about-alexa-rtc)
- [My Media for Alexa : transport local et formats](https://www.mymediaalexa.com/)
