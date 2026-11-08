// Die Anleitungen der Hilfe auf Deutsch (docs/concept-help.md). Same guides and steps as en.js.

export default {
  find: {
    title: "Geräte finden",
    intro: "bacnetio sendet ein Who-Is und fragt zusätzlich die Adressen des Netzes einzeln ab. So werden auch Geräte gefunden, die auf ein Who-Is nicht antworten.",
    steps: [
      "Den Computer mit dem Netz der Anlage verbinden.",
      "In der Seitenleiste {btn:side.searchSettings} öffnen und den {btn:adapter.title} wählen, der zur Anlage führt.",
      "{btn:searchWithSettings} drücken. Die Geräte erscheinen in der Liste, sobald sie antworten.",
      "Ein Gerät wählen: Seine Angaben stehen neben der Liste. {btn:dev.openPoints} liest seine Datenpunkte.",
    ],
    causes: [
      "Kein Gerät gefunden: den Netzwerkadapter prüfen. Ein Laptop mit WLAN und Kabel hat zwei.",
      "Eine Firewall auf diesem Computer kann BACnet sperren (UDP-Port 47808). Unter Windows bacnetio zulassen, wenn beim ersten Start danach gefragt wird.",
      "Die Anlage nutzt einen anderen Port: ihn unter {btn:port} eintragen.",
      "Die Anlage liegt in einem anderen Subnetz: siehe {guide:vpn}.",
      "Ein einzelnes Gerät fehlt: {btn:locate.short} öffnet es über seine Adresse.",
    ],
  },
  vpn: {
    title: "Anlage hinter einem VPN erreichen",
    intro: "Ein VPN leitet keine Broadcasts weiter, ein Who-Is erreicht die Anlage also nicht. bacnetio fragt dann jede Adresse im Subnetz der Anlage direkt ab oder meldet sich beim BBMD der Anlage an.",
    steps: [
      "Das VPN verbinden.",
      "{btn:side.searchSettings} öffnen und als {btn:adapter.title} den Adapter des VPN wählen.",
      "Unter {btn:subnet} das Subnetz der Anlage eintragen, zum Beispiel 192.168.10.0/24. Hat die Anlage einen BBMD, stattdessen seine Adresse unter {btn:bbmd} eintragen.",
      "{btn:searchWithSettings} drücken.",
    ],
    causes: [
      "Nichts gefunden: Das VPN lässt vielleicht nur einzelne Ports durch. BACnet braucht UDP 47808 oder den Port der Anlage unter {btn:port}.",
      "Einzutragen ist das Subnetz der Anlage, nicht das des VPN. Wer das Netz der Anlage betreut, kennt es.",
      "Ein grosses Subnetz dauert lange: nur den Teil abfragen, in dem die Regler stehen, zum Beispiel ein /24.",
      "Ein einzelnes Gerät mit bekannter Adresse: {btn:locate.short} öffnet es direkt.",
    ],
  },
  write: {
    title: "Wert schreiben und freigeben",
    intro: "Ein geschriebener Wert wirkt sofort in der laufenden Anlage. bacnetio führt jeden Wert, den du hältst, unter {btn:tab.log}, auch nach einem Neustart. So kannst du ihn freigeben, bevor du gehst.",
    steps: [
      "Das Gerät öffnen und den Datenpunkt wählen.",
      "In seinen Details {btn:changeValue} drücken, dann {btn:unlock.allow}. bacnetio fragt das einmal pro Sitzung.",
      "Den Wert eingeben und die {btn:priority} wählen; üblich ist 8 (manueller Bediener). {btn:writeValue} drücken.",
      "Wenn du fertig bist, den Wert freigeben: mit dem Freigabe-Button deiner Priorität in den Details oder mit {btn:release} unter {btn:tab.log}.",
      "Bevor du die Anlage verlässt, bietet {btn:leave} unter {btn:tab.log} an, alles freizugeben, was du noch hältst.",
    ],
    causes: [
      "bacnetio fragt vor dem Schreiben nach: Eine höhere Priorität hält einen anderen Wert, deiner würde nicht wirken. Die Prioritäten in den Details zeigen, wer was hält.",
      "Das Gerät hat abgelehnt: bacnetio zeigt seinen Grund, zum Beispiel kein Schreibrecht oder ein falsches Passwort, und hält ihn im Protokoll fest.",
      "Der Wert springt zurück: Ein anderes System, etwa das Leitsystem, schreibt ihn mit gleicher oder höherer Priorität wieder.",
      "Der Punkt ist übersteuert: Vor Ort wirkt ein Handschalter. Die Prioritäten wirken erst wieder, wenn er zurückgestellt ist.",
    ],
  },
  points: {
    title: "Datenpunkte eines Geräts lesen",
    intro: "Die Datenpunkte eines Geräts stehen in einer Liste, nach Anlageteil gruppiert, wo die Namen Pfade sind, sonst nach Art. Der gewählte Punkt steht daneben, mit seinem Wert live, wer ihn hält und allen Eigenschaften.",
    steps: [
      "Das Gerät in der Liste wählen und {btn:dev.openPoints} drücken. bacnetio liest seine Punkte und zeigt, wie weit es ist.",
      "Punkte mit {btn:filter} finden oder mit den Ansichten nur einige zeigen, etwa {btn:view.faults} oder {btn:view.overridden}.",
      "Einen Punkt wählen: Sein Wert folgt live, seine Prioritäten zeigen, wer ihn hält, und darunter stehen alle Eigenschaften des Objekts.",
      "Unter {btn:more} steht, was man ab und zu braucht: die Exporte, {btn:compare.open}, {btn:print.open}, {btn:grouping.open} und {btn:ref.save}.",
    ],
    causes: [
      "Das Lesen dauert lange: Ein Gerät ohne ReadPropertyMultiple oder eines am MS/TP wird Wert für Wert gefragt.",
      "Ein Punkt zeigt einen Fehler: Das Gerät hat diese Eigenschaft verweigert. Die Details nennen seinen Grund.",
      "Die Gruppen passen nicht zur Anlage: {btn:grouping.open} gruppiert die Punkte der Anlage nach einem eigenen Muster.",
    ],
  },
  bbmd: {
    title: "BBMD in einem anderen Subnetz",
    intro: "Ein BBMD gibt Broadcasts zwischen Subnetzen weiter. bacnetio meldet sich bei ihm als Fremdgerät (Foreign Device) an, und der BBMD gibt das Who-Is an die Anlage weiter.",
    steps: [
      "{btn:side.searchSettings} öffnen.",
      "Die Adresse des BBMD unter {btn:bbmd} eintragen, zum Beispiel 10.20.0.10, mit :Port, wenn er nicht 47808 nutzt.",
      "{btn:searchWithSettings} drücken.",
      "Um zu sehen, wie die BBMDs eingerichtet sind, die Gerätedetails eines BBMD öffnen und {btn:bbmd.read} drücken. bacnetio liest seine Tabellen nur und ändert sie nie.",
    ],
    causes: [
      "Der BBMD hat die Anmeldung abgelehnt: Vielleicht nimmt er keine Fremdgeräte an, oder seine Tabelle dafür ist voll.",
      "Der BBMD antwortet nicht: seine Adresse und den Port prüfen, und ob eine Firewall UDP durchlässt.",
      "Nur ein Teil der Anlage antwortet: Den BBMDs der anderen Subnetze fehlen die Einträge der anderen. {btn:bbmd.read} zeigt sie.",
    ],
  },
  mstp: {
    title: "MS/TP über einen USB-RS-485-Adapter",
    intro: "Mit einem USB-RS-485-Adapter arbeitet bacnetio direkt an einem MS/TP-Strang (Windows und Linux). Es hört zuerst mit, ohne etwas zu senden, und nimmt dann am Token-Ring teil.",
    steps: [
      "Den Adapter an den Strang anschliessen (A an A, B an B) und in diesen Computer stecken.",
      "{btn:side.searchSettings} öffnen und unter {btn:link.title} {btn:link.mstp} wählen. Unter {btn:mstp.port} den Adapter wählen.",
      "{btn:mstp.speed} auf {btn:mstp.findSpeed} lassen und {btn:mstp.listen} drücken. bacnetio findet die Geschwindigkeit, die Stationen und eine freie Adresse für sich.",
      "{btn:mstp.join} drücken. Die Geräte am Strang werden gelesen, geschrieben und aufgezeichnet wie über IP.",
      "Wenn du fertig bist, in der Seitenleiste {btn:mstp.leave} drücken: Das gibt den Anschluss frei.",
    ],
    causes: [
      "Kein Anschluss gefunden: den Adapter einstecken und {btn:mstp.lookAgain} drücken. Unter Linux muss dein Benutzer einmal in die Gruppe dialout.",
      "Nichts gehört: die Verdrahtung (A und B), den Abschluss und den Sitz des Adapters am Strang prüfen.",
      "Nur beschädigte Telegramme: A und B vielleicht vertauscht, der Abschluss fehlt, oder der Strang läuft mit einer Geschwindigkeit, die bacnetio nicht probiert.",
      "Kein Master nimmt bacnetio in den Token-Ring auf: neu mithören und mit der angebotenen Adresse teilnehmen.",
    ],
  },
  watch: {
    title: "Werte im Blick behalten",
    intro: "Die Merkliste zeigt Punkte mehrerer Geräte zusammen, mit ihren Werten live.",
    steps: [
      "Im Arbeitsplatz einen Punkt wählen und {btn:watch.add} drücken, oder das Auge in seiner Zeile der Liste einschalten.",
      "In der Seitenleiste {btn:tab.watch} öffnen: Die Werte folgen live.",
      "Eine Zeile wählen, um den Punkt in seinem Gerät zu öffnen.",
    ],
    causes: [
      "Ein Wert bleibt alt: Das Gerät meldet Änderungen nicht selbst (COV) und wird in Abständen neu gefragt.",
    ],
  },
  record: {
    title: "Werte aufzeichnen",
    intro: "Eine Aufzeichnung misst gewählte Werte in einem Intervall oder bei jeder Änderung, als Kurven und als CSV oder Excel. Sie wird jede Minute gesichert und übersteht einen Neustart.",
    steps: [
      "Punkte mit dem Aufzeichnungs-Schalter in ihrer Zeile der Liste wählen oder mit {btn:rec.add} in den Details.",
      "{btn:tab.recording} öffnen, unter {btn:rec.interval} das Intervall wählen und {btn:rec.start} drücken.",
      "Die Kurven einzeln oder zusammen ansehen und in einen Zeitraum hineinzoomen.",
      "Die Werte mit {btn:rec.export} oder {btn:xlsx.recExport} speichern.",
    ],
    causes: [
      "Es wird nichts gemessen: Die Aufzeichnung läuft erst nach {btn:rec.start}.",
      "Ein Punkt wird abgefragt statt bei Änderung gemeldet: Das Gerät meldet ihn nicht selbst (COV abgelehnt).",
      "Lücken in der Kurve: Das Gerät hat zu dieser Zeit nicht geantwortet.",
    ],
  },
  reference: {
    title: "Änderungen sehen, zwei Geräte vergleichen",
    intro: "Eine Referenz merkt sich alle Werte eines Geräts, damit du später siehst, was sich geändert hat, etwa vor und nach der Inbetriebnahme. Ein Vergleich zeigt zwei Geräte nebeneinander.",
    steps: [
      "Das Gerät öffnen und unter {btn:more} {btn:ref.save} drücken.",
      "Später zeigt die Ansicht {btn:view.changed} jeden Punkt, dessen Wert sich seither geändert hat, mit dem Wert von damals.",
      "Für einen Vergleich mit einem anderen Gerät unter {btn:more} {btn:compare.open} drücken und das Gerät wählen. Die Punkte werden nach Namen zugeordnet.",
    ],
    causes: [
      "Alles gilt als geändert: Die Referenz gehört zu einem anderen Zustand der Anlage. Eine neue speichern.",
      "Ein Vergleich zeigt meist «nicht vorhanden»: Die beiden Geräte benennen ihre Punkte verschieden.",
    ],
  },
  exports: {
    title: "Excel, CSV, Drucken, EDE",
    intro: "Jede Liste verlässt bacnetio als Datei: CSV und Excel, eine Seite zum Drucken oder als PDF, und EDE-Dateien für Leitsysteme.",
    steps: [
      "Die Geräteliste und die ganze Anlage: {btn:export} über der Geräteliste, mit {btn:export.devices}, {btn:xlsx.devicesExport}, {btn:xlsx.site} und {btn:ede.site}. Noch nicht gelesene Geräte werden zuerst gelesen.",
      "Die Punkte eines Geräts: {btn:more} in seiner Punkteliste, mit CSV, Excel, auch mit Prioritäten, EDE und seinen Zustandstexten.",
      "Zum Drucken: {btn:print.open} unter {btn:more}, mit dem Filter der Liste. Der Druckdialog bietet auch «Als PDF speichern».",
      "Die gehaltenen Werte und die Aufzeichnung haben eigene Buttons für CSV und Excel.",
    ],
    causes: [
      "In der EDE-Datei fehlen Geräte: Was nicht gelesen werden konnte, fehlt, und bacnetio sagt, wie viele.",
      "Die Dateien der Anlage heissen nach der Anlage: ihr zuerst unter {btn:side.allSites} einen Namen geben.",
    ],
  },
  capture: {
    title: "Mitschnitt für Wireshark",
    intro: "bacnetio schneidet seinen eigenen BACnet-Verkehr als Datei mit, die Wireshark öffnet, ohne Installation und ohne Administratorrechte. Am MS/TP ist jedes Telegramm auf dem Strang darin.",
    steps: [
      "{btn:tab.settings} öffnen und in {btn:capture.title} {btn:capture.start} drücken.",
      "Tun, was du ansehen willst, zum Beispiel ein Gerät lesen.",
      "{btn:capture.stop} drücken, dann {btn:capture.save}, und die Datei in Wireshark öffnen.",
    ],
    causes: [
      "Verkehr anderer Programme fehlt: Der Mitschnitt enthält nur, was bacnetio sendet und empfängt.",
      "Der Mitschnitt war voll: Höchstens 100 000 Pakete werden behalten, die ältesten fallen weg.",
    ],
  },
  sites: {
    title: "Anlagen behalten und öffnen",
    intro: "Jede Suche behält ihre Anlage: die Geräte, was gelesen wurde, und die Gruppierung. Du öffnest sie später wieder, auch ohne die Anlage, oder gibst sie als Datei weiter.",
    steps: [
      "{btn:side.allSites} unter dem Namen der Anlage in der Seitenleiste öffnen.",
      "{btn:sites.rename} gibt einer Anlage einen eigenen Namen; ohne heisst sie nach dem Netz.",
      "{btn:sites.open} zeigt eine Anlage wieder; {btn:sites.file} speichert sie, {btn:sites.import} liest eine ein.",
    ],
    causes: [
      "Die Werte sind alt: Eine geöffnete Anlage zeigt, was damals gelesen wurde. Neu suchen, um die Anlage jetzt zu lesen.",
    ],
  },
  plugins: {
    title: "Erweiterungen",
    intro: "Eine Erweiterung bringt eine Prüfung, einen Bericht oder eine Ansicht, je eine Datei (….bacnetio-plugin.html). Sie liest, was bacnetio liest, kann aber nie in die Anlage schreiben.",
    steps: [
      "{btn:tab.settings} öffnen und in {btn:plugins.title} {btn:plugins.install} drücken. Die Datei wählen.",
      "Die Erweiterung bekommt eine Seite in der Seitenleiste und kann Geräte und Punkte in den Listen markieren.",
      "Am selben Ort wird sie ausgeschaltet oder entfernt.",
    ],
    causes: [
      "Nur Erweiterungen installieren, denen du vertraust: Sie sehen alles, was bacnetio liest.",
    ],
  },
  keys: {
    title: "Tasten",
    intro: "Die Tasten wirken überall ausser in einem Eingabefeld.",
  },
};
