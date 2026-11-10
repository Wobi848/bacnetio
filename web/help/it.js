// Le guide dell'aiuto in italiano (docs/concept-help.md). Same guides and steps as en.js.
// Draft until someone from the trade has reviewed it (docs/concept-languages.md).

export default {
  find: {
    title: "Trovare i dispositivi",
    intro: "bacnetio invia un Who-Is e interroga anche gli indirizzi della rete uno per uno, così trova anche i dispositivi che non rispondono a un Who-Is.",
    steps: [
      "Collegate questo computer alla rete dell'impianto.",
      "Aprite {btn:side.searchSettings} nella barra laterale e scegliete la {btn:adapter.title} che porta all'impianto.",
      "Premete {btn:searchWithSettings}. I dispositivi compaiono nell'elenco man mano che rispondono.",
      "Scegliete un dispositivo: i suoi dati compaiono accanto all'elenco. {btn:dev.openPoints} legge i suoi punti dati.",
    ],
    causes: [
      "Nessun dispositivo trovato: controllate la scheda di rete. Un portatile con Wi-Fi e cavo ne ha due.",
      "Un firewall su questo computer può bloccare BACnet (porta UDP 47808). Su Windows, consentite bacnetio quando lo chiede al primo avvio.",
      "L'impianto usa un'altra porta: inseritela in {btn:port}.",
      "L'impianto si trova in un'altra sottorete: vedere {guide:vpn}.",
      "Manca un solo dispositivo: {btn:locate.short} lo apre tramite il suo indirizzo.",
    ],
  },
  vpn: {
    title: "Raggiungere un impianto dietro una VPN",
    intro: "Una VPN non inoltra i broadcast, quindi un Who-Is non raggiunge l'impianto. bacnetio interroga allora direttamente ogni indirizzo della sottorete dell'impianto, oppure si registra presso il BBMD dell'impianto.",
    steps: [
      "Collegate la VPN.",
      "Aprite {btn:side.searchSettings} e scegliete la scheda della VPN come {btn:adapter.title}.",
      "Inserite la sottorete dell'impianto in {btn:subnet}, per esempio 192.168.10.0/24. Se l'impianto ha un BBMD, inserite invece il suo indirizzo in {btn:bbmd}.",
      "Premete {btn:searchWithSettings}.",
    ],
    causes: [
      "Non si trova nulla: forse la VPN lascia passare solo alcune porte. BACnet ha bisogno di UDP 47808, o della porta dell'impianto indicata in {btn:port}.",
      "La sottorete da inserire è quella dell'impianto, non quella della VPN. Chi gestisce la rete dell'impianto la conosce.",
      "Una sottorete grande richiede tempo: interrogate solo la parte dove si trovano i controllori, per esempio un /24.",
      "Un solo dispositivo di cui conoscete l'indirizzo: {btn:locate.short} lo apre direttamente.",
    ],
  },
  write: {
    title: "Scrivere e rilasciare un valore",
    intro: "Un valore scritto agisce subito nell'impianto in funzione. bacnetio elenca ogni valore che mantenete in {btn:tab.log}, anche dopo un riavvio, così potete rilasciarlo prima di andarvene.",
    steps: [
      "Aprite il dispositivo e scegliete il punto dati.",
      "Nei suoi dettagli premete {btn:changeValue}, poi {btn:unlock.allow}. bacnetio lo chiede una volta per sessione.",
      "Inserite il valore e scegliete la {btn:priority}; 8 (operatore manuale) è la più comune. Premete {btn:writeValue}.",
      "Quando avete finito, rilasciate il valore con il pulsante di rilascio della vostra priorità nei dettagli, oppure con {btn:release} in {btn:tab.log}.",
      "Prima di lasciare l'impianto, {btn:leave} in {btn:tab.log} propone di rilasciare tutto ciò che mantenete ancora.",
    ],
    causes: [
      "bacnetio chiede prima di scrivere: una priorità più alta mantiene un altro valore, quindi il vostro non agirebbe. Le priorità nei dettagli mostrano chi mantiene cosa.",
      "Il dispositivo ha rifiutato: bacnetio mostra il motivo, per esempio nessun accesso in scrittura o una password errata, e lo conserva nel registro.",
      "Il valore torna indietro: un altro sistema, come la supervisione dell'edificio, lo riscrive con la stessa priorità o una più alta.",
      "Il punto è forzato: agisce un commutatore manuale in campo, e le priorità agiscono di nuovo solo quando viene rilasciato.",
    ],
  },
  points: {
    title: "Leggere i punti di un dispositivo",
    intro: "I punti dati di un dispositivo stanno in un elenco, raggruppati per parte d'impianto quando i nomi sono percorsi, altrimenti per tipo. Il punto scelto compare accanto con il valore in tempo reale, chi lo mantiene e tutte le proprietà.",
    steps: [
      "Scegliete il dispositivo nell'elenco e premete {btn:dev.openPoints}. bacnetio legge i suoi punti e mostra a che punto è.",
      "Trovate i punti con {btn:filter}, oppure mostratene solo alcuni con le viste, come {btn:view.faults} o {btn:view.overridden}.",
      "Scegliete un punto: il suo valore si aggiorna in tempo reale, le priorità mostrano chi lo mantiene, e sotto compaiono tutte le proprietà dell'oggetto.",
      "{btn:more} contiene ciò che serve ogni tanto: le esportazioni, {btn:compare.open}, {btn:print.open}, {btn:grouping.open} e {btn:ref.save}.",
    ],
    causes: [
      "La lettura è lenta: un dispositivo senza ReadPropertyMultiple, o su MS/TP, viene interrogato un valore alla volta.",
      "Un punto mostra un errore: il dispositivo ha rifiutato di fornire quella proprietà. I dettagli ne indicano il motivo.",
      "I gruppi non corrispondono all'impianto: {btn:grouping.open} raggruppa i punti dell'impianto secondo uno schema vostro.",
    ],
  },
  bbmd: {
    title: "Un BBMD in un'altra sottorete",
    intro: "Un BBMD inoltra i broadcast tra sottoreti. bacnetio si registra presso di esso come dispositivo esterno (foreign device), e il BBMD inoltra il Who-Is all'impianto.",
    steps: [
      "Aprite {btn:side.searchSettings}.",
      "Inserite l'indirizzo del BBMD in {btn:bbmd}, per esempio 10.20.0.10, con :porta se non usa la 47808.",
      "Premete {btn:searchWithSettings}.",
      "Per vedere come sono configurati i BBMD, aprite i dettagli di un dispositivo BBMD e premete {btn:bbmd.read}. bacnetio legge solo le sue tabelle e non le modifica mai.",
    ],
    causes: [
      "Il BBMD ha rifiutato la registrazione: forse non accetta dispositivi esterni, o la sua tabella è piena.",
      "Il BBMD non risponde: controllate indirizzo e porta, e che un firewall lasci passare l'UDP.",
      "Risponde solo una parte dell'impianto: i BBMD delle altre sottoreti mancano nelle tabelle l'uno dell'altro. {btn:bbmd.read} li mostra.",
    ],
  },
  mstp: {
    title: "MS/TP con un adattatore USB RS-485",
    intro: "Con un adattatore USB RS-485, bacnetio lavora direttamente su un bus MS/TP (Windows e Linux). Prima ascolta senza inviare nulla, poi entra nell'anello del token.",
    steps: [
      "Collegate l'adattatore al bus (A con A, B con B) e inseritelo in questo computer.",
      "Aprite {btn:side.searchSettings} e scegliete {btn:link.mstp} in {btn:link.title}. Scegliete l'adattatore in {btn:mstp.port}.",
      "Lasciate {btn:mstp.speed} su {btn:mstp.findSpeed} e premete {btn:mstp.listen}. bacnetio trova la velocità, le stazioni e un indirizzo libero per sé.",
      "Premete {btn:mstp.join}. I dispositivi del bus vengono letti, scritti e registrati come su IP.",
      "Quando avete finito, premete {btn:mstp.leave} nella barra laterale: libera la porta.",
    ],
    causes: [
      "Nessuna porta trovata: inserite l'adattatore e premete {btn:mstp.lookAgain}. Su Linux il vostro utente deve essere aggiunto una volta al gruppo dialout.",
      "Non si sente nulla: controllate il cablaggio (A e B), la terminazione e che l'adattatore sia sul bus.",
      "Solo frame danneggiati: forse A e B sono invertiti, manca la terminazione, oppure il bus funziona a una velocità che bacnetio non prova.",
      "Nessun master accoglie bacnetio nell'anello del token: ascoltate di nuovo ed entrate con l'indirizzo proposto.",
    ],
  },
  sc: {
    title: "Aprire un impianto BACnet/SC",
    intro: "In BACnet/SC i dispositivi comunicano tramite un hub, su connessioni cifrate con TLS. bacnetio ha bisogno di tre file dall'integratore: la CA dell'impianto, un certificato per bacnetio e la sua chiave privata, spesso tutti in un file PKCS#12.",
    steps: [
      "Aprite {btn:side.searchSettings} e scegliete {btn:link.sc} in {btn:link.title}. Inserite l'indirizzo dell'hub in {btn:sc.hub}.",
      "Per {btn:sc.line.ca}, {btn:sc.line.own} e {btn:sc.line.key} premete {btn:sc.choose} e scegliete il file. Un file PKCS#12 riempie tutte e tre le righe in una volta.",
      "Se un file ha una password, digitatela e premete {btn:sc.open}. Ogni riga dice cosa ha trovato bacnetio: valido fino al, scaduto, non di questa CA, non corrisponde.",
      "Premete {btn:sc.connect}. I dispositivi della rete SC vengono letti, scritti e registrati come su IP.",
      "Quando avete finito, premete {btn:sc.leave} nella barra laterale.",
    ],
    causes: [
      "Nessun hub risponde: controllate l'indirizzo e che questo computer raggiunga l'hub (VPN, firewall, porta).",
      "Il certificato dell'hub non è della CA dell'impianto: il file CA è sbagliato, o l'indirizzo non è l'hub di questo impianto.",
      "L'hub rifiuta il certificato di bacnetio: è di un'altra CA, è scaduto, o l'hub non lo ammette ancora. Chiedete all'integratore.",
      "La chiave non corrisponde: il file della chiave appartiene a un altro certificato.",
    ],
  },
  watch: {
    title: "Tenere d'occhio dei valori",
    intro: "La lista di controllo mostra insieme punti di più dispositivi, con i loro valori in tempo reale.",
    steps: [
      "Scegliete un punto nell'area di lavoro e premete {btn:watch.add}, oppure attivate l'occhio nella sua riga dell'elenco.",
      "Aprite {btn:tab.watch} nella barra laterale: i valori si aggiornano in tempo reale.",
      "Scegliete una riga per aprire il punto nel suo dispositivo.",
    ],
    causes: [
      "Un valore resta vecchio: il dispositivo non segnala le variazioni (COV) e viene interrogato a intervalli.",
    ],
  },
  record: {
    title: "Registrare valori",
    intro: "Una registrazione misura i valori scelti a intervalli o a ogni variazione, come grafici e come CSV o Excel. Viene salvata ogni minuto e sopravvive a un riavvio.",
    steps: [
      "Scegliete i punti con l'interruttore di registrazione nella loro riga dell'elenco, oppure con {btn:rec.add} nei dettagli.",
      "Aprite {btn:tab.recording}, scegliete l'intervallo in {btn:rec.interval} e premete {btn:rec.start}.",
      "Guardate i grafici, uno per uno o insieme, e ingrandite un periodo.",
      "Salvate i valori con {btn:rec.export} o {btn:xlsx.recExport}.",
    ],
    causes: [
      "Non viene misurato nulla: la registrazione parte solo dopo {btn:rec.start}.",
      "Un punto viene interrogato invece di essere segnalato a ogni variazione: il dispositivo non lo segnala da sé (COV rifiutato).",
      "Buchi nel grafico: il dispositivo in quel momento non ha risposto.",
    ],
  },
  reference: {
    title: "Vedere cosa è cambiato, confrontare due dispositivi",
    intro: "Un riferimento memorizza tutti i valori di un dispositivo, così vedete in seguito cosa è cambiato, per esempio prima e dopo la messa in servizio. Un confronto mostra due dispositivi affiancati.",
    steps: [
      "Aprite il dispositivo e premete {btn:ref.save} in {btn:more}.",
      "In seguito, la vista {btn:view.changed} mostra ogni punto il cui valore è cambiato da allora, con il valore che aveva.",
      "Per confrontare con un altro dispositivo, premete {btn:compare.open} in {btn:more} e sceglietelo. I punti vengono abbinati per nome.",
    ],
    causes: [
      "Tutto risulta cambiato: il riferimento appartiene a un altro stato dell'impianto. Salvatene uno nuovo.",
      "Un confronto mostra soprattutto «assente»: i due dispositivi chiamano i loro punti in modo diverso.",
    ],
  },
  exports: {
    title: "Excel, CSV, stampa, EDE",
    intro: "Ogni elenco esce da bacnetio come file: CSV ed Excel, una pagina da stampare o salvare come PDF, e file EDE per i sistemi di supervisione.",
    steps: [
      "L'elenco dei dispositivi e l'intero impianto: {btn:export} sopra l'elenco dei dispositivi, con {btn:export.devices}, {btn:xlsx.devicesExport}, {btn:xlsx.site} e {btn:ede.site}. I dispositivi non ancora letti vengono letti prima.",
      "I punti di un dispositivo: {btn:more} nel suo elenco di punti, con CSV, Excel, anche con le priorità, EDE e i suoi testi di stato.",
      "Per stampare: {btn:print.open} in {btn:more}, con il filtro dell'elenco. La finestra di stampa offre anche Salva come PDF.",
      "I valori mantenuti e la registrazione hanno i propri pulsanti per CSV ed Excel.",
    ],
    causes: [
      "Nel file EDE mancano dispositivi: quelli che non è stato possibile leggere vengono tralasciati, e bacnetio dice quanti sono.",
      "I file dell'impianto prendono il nome dell'impianto: dategli prima un nome in {btn:side.allSites}.",
    ],
  },
  capture: {
    title: "Cattura per Wireshark",
    intro: "bacnetio registra il proprio traffico BACnet in un file che Wireshark apre, senza installare nulla e senza diritti di amministratore. Su MS/TP contiene ogni frame del bus.",
    steps: [
      "Aprite {btn:tab.settings} e, in {btn:capture.title}, premete {btn:capture.start}.",
      "Fate ciò che volete osservare, per esempio leggere un dispositivo.",
      "Premete {btn:capture.stop}, poi {btn:capture.save}, e aprite il file in Wireshark.",
    ],
    causes: [
      "Manca il traffico di altri programmi: la cattura contiene solo ciò che bacnetio invia e riceve.",
      "La cattura era piena: si conservano al massimo 100 000 pacchetti, i più vecchi vengono tralasciati.",
    ],
  },
  sites: {
    title: "Conservare e riaprire impianti",
    intro: "Ogni ricerca conserva il suo impianto: i dispositivi, ciò che è stato letto e il raggruppamento. Lo riaprite più tardi, anche senza l'impianto, o lo passate ad altri come file.",
    steps: [
      "Aprite {btn:side.allSites} sotto il nome dell'impianto nella barra laterale.",
      "{btn:sites.rename} dà a un impianto un nome vostro; senza nome prende quello della rete.",
      "{btn:sites.open} mostra di nuovo un impianto; {btn:sites.file} lo salva, {btn:sites.import} ne legge uno.",
    ],
    causes: [
      "I valori sono vecchi: un impianto riaperto mostra ciò che è stato letto allora. Cercate di nuovo per leggere l'impianto adesso.",
    ],
  },
  plugins: {
    title: "Estensioni",
    intro: "Un'estensione aggiunge un controllo, un rapporto o una vista, un file ciascuna (….bacnetio-plugin.html). Legge ciò che legge bacnetio, ma non può mai scrivere nell'impianto.",
    steps: [
      "Aprite {btn:tab.settings} e, in {btn:plugins.title}, premete {btn:plugins.install}. Scegliete il file.",
      "L'estensione ottiene una pagina nella barra laterale e può segnare dispositivi e punti negli elenchi.",
      "Disattivatela o rimuovetela nello stesso punto.",
    ],
    causes: [
      "Installate solo estensioni di cui vi fidate: vedono tutto ciò che legge bacnetio.",
    ],
  },
  keys: {
    title: "Tasti",
    intro: "I tasti funzionano ovunque, tranne in un campo di inserimento.",
  },
};
