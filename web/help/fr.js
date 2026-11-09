// Les guides de l'aide en français (docs/concept-help.md). Same guides and steps as en.js.
// Draft until someone from the trade has reviewed it (docs/concept-languages.md).

export default {
  find: {
    title: "Trouver les appareils",
    intro: "bacnetio envoie un Who-Is et interroge aussi les adresses du réseau une par une, pour trouver même les appareils qui ne répondent pas à un Who-Is.",
    steps: [
      "Raccordez cet ordinateur au réseau de l'installation.",
      "Ouvrez {btn:side.searchSettings} dans la barre latérale et choisissez l'{btn:adapter.title} qui mène à l'installation.",
      "Appuyez sur {btn:searchWithSettings}. Les appareils apparaissent dans la liste au fur et à mesure qu'ils répondent.",
      "Choisissez un appareil : ses informations s'affichent à côté de la liste. {btn:dev.openPoints} lit ses points de données.",
    ],
    causes: [
      "Aucun appareil trouvé : vérifiez l'adaptateur réseau. Un portable avec Wi-Fi et câble en a deux.",
      "Un pare-feu sur cet ordinateur peut bloquer BACnet (port UDP 47808). Sous Windows, autorisez bacnetio quand il le demande au premier démarrage.",
      "L'installation utilise un autre port : saisissez-le sous {btn:port}.",
      "L'installation se trouve dans un autre sous-réseau : voir {guide:vpn}.",
      "Un seul appareil manque : {btn:locate.short} l'ouvre par son adresse.",
    ],
  },
  vpn: {
    title: "Atteindre une installation derrière un VPN",
    intro: "Un VPN ne transmet pas les broadcasts, donc un Who-Is n'atteint pas l'installation. bacnetio interroge alors directement chaque adresse du sous-réseau de l'installation, ou s'inscrit auprès du BBMD de l'installation.",
    steps: [
      "Connectez le VPN.",
      "Ouvrez {btn:side.searchSettings} et choisissez l'adaptateur du VPN comme {btn:adapter.title}.",
      "Saisissez le sous-réseau de l'installation sous {btn:subnet}, par exemple 192.168.10.0/24. Si l'installation a un BBMD, saisissez plutôt son adresse sous {btn:bbmd}.",
      "Appuyez sur {btn:searchWithSettings}.",
    ],
    causes: [
      "Rien trouvé : le VPN ne laisse peut-être passer que certains ports. BACnet a besoin d'UDP 47808, ou du port de l'installation sous {btn:port}.",
      "Le sous-réseau à saisir est celui de l'installation, pas celui du VPN. Le responsable du réseau de l'installation le connaît.",
      "Un grand sous-réseau prend du temps : n'interrogez que la partie où se trouvent les automates, par exemple un /24.",
      "Un seul appareil dont vous connaissez l'adresse : {btn:locate.short} l'ouvre directement.",
    ],
  },
  write: {
    title: "Écrire et libérer une valeur",
    intro: "Une valeur écrite agit immédiatement dans l'installation en service. bacnetio liste chaque valeur que vous maintenez sous {btn:tab.log}, même après un redémarrage, pour que vous puissiez la libérer avant de partir.",
    steps: [
      "Ouvrez l'appareil et choisissez le point de données.",
      "Dans ses détails, appuyez sur {btn:changeValue}, puis sur {btn:unlock.allow}. bacnetio le demande une fois par session.",
      "Saisissez la valeur et choisissez la {btn:priority} ; 8 (opérateur manuel) est la plus courante. Appuyez sur {btn:writeValue}.",
      "Quand vous avez terminé, libérez la valeur avec le bouton de libération de votre priorité dans les détails, ou avec {btn:release} sous {btn:tab.log}.",
      "Avant de quitter le site, {btn:leave} sous {btn:tab.log} propose de libérer tout ce que vous maintenez encore.",
    ],
    causes: [
      "bacnetio demande avant d'écrire : une priorité plus haute maintient une autre valeur, la vôtre n'agirait donc pas. Les priorités dans les détails montrent qui maintient quoi.",
      "L'appareil a refusé : bacnetio affiche sa raison, par exemple pas d'accès en écriture ou un mauvais mot de passe, et la garde dans le journal.",
      "La valeur revient : un autre système, comme la gestion technique du bâtiment, la réécrit à la même priorité ou à une plus haute.",
      "Le point est forcé : un commutateur manuel sur site agit, et les priorités n'agissent à nouveau qu'une fois qu'il est relâché.",
    ],
  },
  points: {
    title: "Lire les points d'un appareil",
    intro: "Les points de données d'un appareil sont dans une liste, groupés par partie d'installation quand les noms sont des chemins, sinon par type. Le point choisi s'affiche à côté avec sa valeur en direct, qui le maintient et toutes ses propriétés.",
    steps: [
      "Choisissez l'appareil dans la liste et appuyez sur {btn:dev.openPoints}. bacnetio lit ses points et indique l'avancement.",
      "Trouvez des points avec {btn:filter}, ou n'en affichez qu'une partie avec les vues, comme {btn:view.faults} ou {btn:view.overridden}.",
      "Choisissez un point : sa valeur suit en direct, ses priorités montrent qui le maintient, et toutes les propriétés de l'objet suivent en dessous.",
      "{btn:more} contient ce qui sert de temps en temps : les exports, {btn:compare.open}, {btn:print.open}, {btn:grouping.open} et {btn:ref.save}.",
    ],
    causes: [
      "La lecture est longue : un appareil sans ReadPropertyMultiple, ou sur MS/TP, est interrogé une valeur à la fois.",
      "Un point affiche une erreur : l'appareil a refusé de donner cette propriété. Les détails indiquent sa raison.",
      "Les groupes ne correspondent pas à l'installation : {btn:grouping.open} groupe les points du site selon un modèle à vous.",
    ],
  },
  bbmd: {
    title: "Un BBMD dans un autre sous-réseau",
    intro: "Un BBMD transmet les broadcasts entre sous-réseaux. bacnetio s'y inscrit comme appareil étranger (foreign device), et le BBMD transmet le Who-Is à l'installation.",
    steps: [
      "Ouvrez {btn:side.searchSettings}.",
      "Saisissez l'adresse du BBMD sous {btn:bbmd}, par exemple 10.20.0.10, avec :port s'il n'utilise pas 47808.",
      "Appuyez sur {btn:searchWithSettings}.",
      "Pour voir comment les BBMD sont configurés, ouvrez les détails d'un appareil BBMD et appuyez sur {btn:bbmd.read}. bacnetio lit seulement ses tables et ne les modifie jamais.",
    ],
    causes: [
      "Le BBMD a refusé l'inscription : il n'accepte peut-être pas d'appareils étrangers, ou sa table est pleine.",
      "Le BBMD ne répond pas : vérifiez son adresse et son port, et qu'un pare-feu laisse passer l'UDP.",
      "Seule une partie de l'installation répond : les BBMD des autres sous-réseaux manquent dans les tables les uns des autres. {btn:bbmd.read} les montre.",
    ],
  },
  mstp: {
    title: "MS/TP par un adaptateur USB RS-485",
    intro: "Avec un adaptateur USB RS-485, bacnetio travaille directement sur un bus MS/TP (Windows et Linux). Il écoute d'abord sans rien envoyer, puis entre dans l'anneau à jeton.",
    steps: [
      "Raccordez l'adaptateur au bus (A sur A, B sur B) et branchez-le sur cet ordinateur.",
      "Ouvrez {btn:side.searchSettings} et choisissez {btn:link.mstp} sous {btn:link.title}. Choisissez l'adaptateur sous {btn:mstp.port}.",
      "Laissez {btn:mstp.speed} sur {btn:mstp.findSpeed} et appuyez sur {btn:mstp.listen}. bacnetio trouve la vitesse, les stations et une adresse libre pour lui-même.",
      "Appuyez sur {btn:mstp.join}. Les appareils du bus sont lus, écrits et enregistrés comme sur IP.",
      "Quand vous avez terminé, appuyez sur {btn:mstp.leave} dans la barre latérale : cela libère le port.",
    ],
    causes: [
      "Aucun port trouvé : branchez l'adaptateur et appuyez sur {btn:mstp.lookAgain}. Sous Linux, votre utilisateur doit être une fois ajouté au groupe dialout.",
      "Rien entendu : vérifiez le câblage (A et B), la terminaison et que l'adaptateur est bien sur le bus.",
      "Seulement des trames endommagées : A et B sont peut-être inversés, la terminaison manque, ou le bus tourne à une vitesse que bacnetio n'essaie pas.",
      "Aucun maître ne prend bacnetio dans l'anneau à jeton : écoutez à nouveau et entrez avec l'adresse proposée.",
    ],
  },
  watch: {
    title: "Garder un œil sur des valeurs",
    intro: "La liste de suivi montre ensemble des points de plusieurs appareils, avec leurs valeurs en direct.",
    steps: [
      "Choisissez un point dans l'espace de travail et appuyez sur {btn:watch.add}, ou activez l'œil dans sa ligne de la liste.",
      "Ouvrez {btn:tab.watch} dans la barre latérale : les valeurs suivent en direct.",
      "Choisissez une ligne pour ouvrir le point dans son appareil.",
    ],
    causes: [
      "Une valeur reste ancienne : l'appareil ne signale pas les changements (COV) et il est interrogé à intervalles.",
    ],
  },
  record: {
    title: "Enregistrer des valeurs",
    intro: "Un enregistrement mesure les valeurs choisies à intervalle régulier ou à chaque changement, en graphiques et en CSV ou Excel. Il est sauvegardé chaque minute et survit à un redémarrage.",
    steps: [
      "Choisissez des points avec l'interrupteur d'enregistrement dans leur ligne de la liste, ou avec {btn:rec.add} dans les détails.",
      "Ouvrez {btn:tab.recording}, choisissez l'intervalle sous {btn:rec.interval} et appuyez sur {btn:rec.start}.",
      "Regardez les graphiques, un par un ou ensemble, et zoomez sur une période.",
      "Enregistrez les valeurs avec {btn:rec.export} ou {btn:xlsx.recExport}.",
    ],
    causes: [
      "Rien n'est mesuré : l'enregistrement ne tourne qu'après {btn:rec.start}.",
      "Un point est interrogé au lieu d'être signalé à chaque changement : l'appareil ne le signale pas de lui-même (COV refusé).",
      "Des trous dans le graphique : l'appareil n'a pas répondu à ce moment-là.",
    ],
  },
  reference: {
    title: "Voir ce qui a changé, comparer deux appareils",
    intro: "Une référence mémorise toutes les valeurs d'un appareil, pour voir plus tard ce qui a changé, par exemple avant et après la mise en service. Une comparaison montre deux appareils côte à côte.",
    steps: [
      "Ouvrez l'appareil et appuyez sur {btn:ref.save} sous {btn:more}.",
      "Plus tard, la vue {btn:view.changed} montre chaque point dont la valeur a changé depuis, avec la valeur qu'il avait.",
      "Pour comparer avec un autre appareil, appuyez sur {btn:compare.open} sous {btn:more} et choisissez-le. Les points sont associés par leur nom.",
    ],
    causes: [
      "Tout compte comme changé : la référence appartient à un autre état de l'installation. Enregistrez-en une nouvelle.",
      "Une comparaison montre surtout « absent » : les deux appareils nomment leurs points différemment.",
    ],
  },
  exports: {
    title: "Excel, CSV, impression, EDE",
    intro: "Chaque liste quitte bacnetio sous forme de fichier : CSV et Excel, une page à imprimer ou à enregistrer en PDF, et des fichiers EDE pour les systèmes de gestion.",
    steps: [
      "La liste des appareils et tout le site : {btn:export} au-dessus de la liste des appareils, avec {btn:export.devices}, {btn:xlsx.devicesExport}, {btn:xlsx.site} et {btn:ede.site}. Les appareils pas encore lus sont lus d'abord.",
      "Les points d'un appareil : {btn:more} dans sa liste de points, avec CSV, Excel, aussi avec les priorités, EDE et ses textes d'état.",
      "Pour imprimer : {btn:print.open} sous {btn:more}, avec le filtre de la liste. La boîte de dialogue d'impression propose aussi Enregistrer en PDF.",
      "Les valeurs maintenues et l'enregistrement ont leurs propres boutons pour CSV et Excel.",
    ],
    causes: [
      "Le fichier EDE ne contient pas tous les appareils : ceux qui n'ont pas pu être lus sont laissés de côté, et bacnetio dit combien.",
      "Les fichiers du site portent le nom du site : donnez-lui d'abord un nom sous {btn:side.allSites}.",
    ],
  },
  capture: {
    title: "Capture pour Wireshark",
    intro: "bacnetio enregistre son propre trafic BACnet dans un fichier que Wireshark ouvre, sans rien installer et sans droits d'administrateur. Sur MS/TP, toutes les trames du bus y figurent.",
    steps: [
      "Ouvrez {btn:tab.settings} et, dans {btn:capture.title}, appuyez sur {btn:capture.start}.",
      "Faites ce que vous voulez observer, par exemple lire un appareil.",
      "Appuyez sur {btn:capture.stop}, puis sur {btn:capture.save}, et ouvrez le fichier dans Wireshark.",
    ],
    causes: [
      "Le trafic d'autres programmes manque : la capture ne contient que ce que bacnetio envoie et reçoit.",
      "La capture était pleine : au plus 100 000 paquets sont gardés, les plus anciens sont laissés de côté.",
    ],
  },
  sites: {
    title: "Garder et rouvrir des sites",
    intro: "Chaque recherche garde son site : les appareils, ce qui a été lu et le regroupement. Vous le rouvrez plus tard, même sans l'installation, ou le transmettez sous forme de fichier.",
    steps: [
      "Ouvrez {btn:side.allSites} sous le nom du site dans la barre latérale.",
      "{btn:sites.rename} donne à un site votre propre nom ; sans nom, il porte celui du réseau.",
      "{btn:sites.open} affiche à nouveau un site ; {btn:sites.file} l'enregistre, {btn:sites.import} en lit un.",
    ],
    causes: [
      "Les valeurs sont anciennes : un site rouvert montre ce qui a été lu alors. Relancez une recherche pour lire l'installation maintenant.",
    ],
  },
  plugins: {
    title: "Extensions",
    intro: "Une extension ajoute un contrôle, un rapport ou une vue, un fichier chacune (….bacnetio-plugin.html). Elle lit ce que bacnetio lit, mais ne peut jamais écrire dans l'installation.",
    steps: [
      "Ouvrez {btn:tab.settings} et, dans {btn:plugins.title}, appuyez sur {btn:plugins.install}. Choisissez le fichier.",
      "L'extension obtient une page dans la barre latérale et peut marquer des appareils et des points dans les listes.",
      "Désactivez-la ou supprimez-la au même endroit.",
    ],
    causes: [
      "N'installez que des extensions de confiance : elles voient tout ce que bacnetio lit.",
    ],
  },
  keys: {
    title: "Touches",
    intro: "Les touches agissent partout, sauf dans un champ de saisie.",
  },
};
