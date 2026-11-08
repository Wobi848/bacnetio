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
  keys: {
    title: "Tasten",
    intro: "Die Tasten wirken überall ausser in einem Eingabefeld.",
  },
};
