// Las guías de la ayuda en español (docs/concept-help.md). Same guides and steps as en.js.
// Draft until someone from the trade has reviewed it (docs/concept-languages.md).

export default {
  find: {
    title: "Encontrar los dispositivos",
    intro: "bacnetio envía un Who-Is y además pregunta a las direcciones de la red una por una, así encuentra también los dispositivos que no responden a un Who-Is.",
    steps: [
      "Conecte este ordenador a la red de la instalación.",
      "Abra {btn:side.searchSettings} en la barra lateral y elija el {btn:adapter.title} que lleva a la instalación.",
      "Pulse {btn:searchWithSettings}. Los dispositivos aparecen en la lista a medida que responden.",
      "Elija un dispositivo: sus datos aparecen junto a la lista. {btn:dev.openPoints} lee sus puntos de datos.",
    ],
    causes: [
      "No se encontró ningún dispositivo: compruebe el adaptador de red. Un portátil con Wi-Fi y cable tiene dos.",
      "Un cortafuegos en este ordenador puede bloquear BACnet (puerto UDP 47808). En Windows, permita bacnetio cuando lo pregunte en el primer inicio.",
      "La instalación usa otro puerto: introdúzcalo en {btn:port}.",
      "La instalación está en otra subred: vea {guide:vpn}.",
      "Falta un solo dispositivo: {btn:locate.short} lo abre por su dirección.",
    ],
  },
  vpn: {
    title: "Llegar a una instalación detrás de una VPN",
    intro: "Una VPN no transmite los broadcasts, así que un Who-Is no llega a la instalación. bacnetio pregunta entonces directamente a cada dirección de la subred de la instalación, o se registra en el BBMD de la instalación.",
    steps: [
      "Conecte la VPN.",
      "Abra {btn:side.searchSettings} y elija el adaptador de la VPN como {btn:adapter.title}.",
      "Introduzca la subred de la instalación en {btn:subnet}, por ejemplo 192.168.10.0/24. Si la instalación tiene un BBMD, introduzca en su lugar su dirección en {btn:bbmd}.",
      "Pulse {btn:searchWithSettings}.",
    ],
    causes: [
      "No se encuentra nada: puede que la VPN solo deje pasar algunos puertos. BACnet necesita UDP 47808, o el puerto de la instalación indicado en {btn:port}.",
      "La subred a introducir es la de la instalación, no la de la VPN. Quien gestiona la red de la instalación la conoce.",
      "Una subred grande tarda: pregunte solo la parte donde están los controladores, por ejemplo una /24.",
      "Un solo dispositivo cuya dirección conoce: {btn:locate.short} lo abre directamente.",
    ],
  },
  write: {
    title: "Escribir y liberar un valor",
    intro: "Un valor escrito actúa de inmediato en la instalación en marcha. bacnetio lista cada valor que usted mantiene en {btn:tab.log}, también después de reiniciar, para que pueda liberarlo antes de irse.",
    steps: [
      "Abra el dispositivo y elija el punto de datos.",
      "En sus detalles, pulse {btn:changeValue} y después {btn:unlock.allow}. bacnetio lo pregunta una vez por sesión.",
      "Introduzca el valor y elija la {btn:priority}; 8 (operador manual) es la habitual. Pulse {btn:writeValue}.",
      "Cuando termine, libere el valor con el botón de liberar de su prioridad en los detalles, o con {btn:release} en {btn:tab.log}.",
      "Antes de dejar la instalación, {btn:leave} en {btn:tab.log} ofrece liberar todo lo que todavía mantiene.",
    ],
    causes: [
      "bacnetio pregunta antes de escribir: una prioridad más alta mantiene otro valor, así que el suyo no actuaría. Las prioridades en los detalles muestran quién mantiene qué.",
      "El dispositivo lo rechazó: bacnetio muestra el motivo, por ejemplo sin acceso de escritura o una contraseña incorrecta, y lo guarda en el registro.",
      "El valor vuelve atrás: otro sistema, como la gestión técnica del edificio, lo escribe de nuevo con la misma prioridad o una más alta.",
      "El punto está forzado: actúa un conmutador manual en campo, y las prioridades vuelven a actuar solo cuando se suelta.",
    ],
  },
  points: {
    title: "Leer los puntos de un dispositivo",
    intro: "Los puntos de datos de un dispositivo están en una lista, agrupados por parte de la instalación cuando los nombres son rutas, si no por tipo. El punto elegido aparece al lado con su valor en vivo, quién lo mantiene y todas sus propiedades.",
    steps: [
      "Elija el dispositivo en la lista y pulse {btn:dev.openPoints}. bacnetio lee sus puntos y muestra cuánto lleva.",
      "Encuentre puntos con {btn:filter}, o muestre solo algunos con las vistas, como {btn:view.faults} o {btn:view.overridden}.",
      "Elija un punto: su valor se actualiza en vivo, sus prioridades muestran quién lo mantiene, y debajo están todas las propiedades del objeto.",
      "{btn:more} contiene lo que se usa de vez en cuando: las exportaciones, {btn:compare.open}, {btn:print.open}, {btn:grouping.open} y {btn:ref.save}.",
    ],
    causes: [
      "La lectura tarda: a un dispositivo sin ReadPropertyMultiple, o en MS/TP, se le pregunta un valor cada vez.",
      "Un punto muestra un error: el dispositivo se negó a dar esa propiedad. Los detalles indican el motivo.",
      "Los grupos no encajan con la instalación: {btn:grouping.open} agrupa los puntos de la instalación según un patrón propio.",
    ],
  },
  bbmd: {
    title: "Un BBMD en otra subred",
    intro: "Un BBMD transmite los broadcasts entre subredes. bacnetio se registra en él como dispositivo externo (foreign device), y el BBMD transmite el Who-Is a la instalación.",
    steps: [
      "Abra {btn:side.searchSettings}.",
      "Introduzca la dirección del BBMD en {btn:bbmd}, por ejemplo 10.20.0.10, con :puerto si no usa el 47808.",
      "Pulse {btn:searchWithSettings}.",
      "Para ver cómo están configurados los BBMD, abra los detalles de un dispositivo BBMD y pulse {btn:bbmd.read}. bacnetio solo lee sus tablas y nunca las cambia.",
    ],
    causes: [
      "El BBMD rechazó el registro: puede que no acepte dispositivos externos, o que su tabla esté llena.",
      "El BBMD no responde: compruebe su dirección y su puerto, y que un cortafuegos deje pasar UDP.",
      "Solo responde una parte de la instalación: faltan los BBMD de las otras subredes en las tablas de los demás. {btn:bbmd.read} los muestra.",
    ],
  },
  mstp: {
    title: "MS/TP con un adaptador USB RS-485",
    intro: "Con un adaptador USB RS-485, bacnetio trabaja directamente en un bus MS/TP (Windows y Linux). Primero escucha sin enviar nada y después entra en el anillo de testigo.",
    steps: [
      "Conecte el adaptador al bus (A con A, B con B) y enchúfelo a este ordenador.",
      "Abra {btn:side.searchSettings} y elija {btn:link.mstp} en {btn:link.title}. Elija el adaptador en {btn:mstp.port}.",
      "Deje {btn:mstp.speed} en {btn:mstp.findSpeed} y pulse {btn:mstp.listen}. bacnetio encuentra la velocidad, las estaciones y una dirección libre para sí mismo.",
      "Pulse {btn:mstp.join}. Los dispositivos del bus se leen, se escriben y se registran como en IP.",
      "Cuando termine, pulse {btn:mstp.leave} en la barra lateral: libera el puerto.",
    ],
    causes: [
      "No se encuentra ningún puerto: enchufe el adaptador y pulse {btn:mstp.lookAgain}. En Linux, su usuario debe estar una vez en el grupo dialout.",
      "No se oye nada: compruebe el cableado (A y B), la terminación y que el adaptador está en el bus.",
      "Solo tramas dañadas: puede que A y B estén invertidos, falte la terminación, o el bus funcione a una velocidad que bacnetio no prueba.",
      "Ningún maestro acepta a bacnetio en el anillo de testigo: escuche de nuevo y entre con la dirección ofrecida.",
    ],
  },
  watch: {
    title: "Vigilar valores",
    intro: "La lista de seguimiento muestra juntos puntos de varios dispositivos, con sus valores en vivo.",
    steps: [
      "Elija un punto en el área de trabajo y pulse {btn:watch.add}, o active el ojo en su fila de la lista.",
      "Abra {btn:tab.watch} en la barra lateral: los valores se actualizan en vivo.",
      "Elija una fila para abrir el punto en su dispositivo.",
    ],
    causes: [
      "Un valor se queda viejo: el dispositivo no notifica los cambios (COV) y se le pregunta a intervalos.",
    ],
  },
  record: {
    title: "Registrar valores",
    intro: "Un registro mide los valores elegidos a intervalos o en cada cambio, como gráficos y como CSV o Excel. Se guarda cada minuto y sobrevive a un reinicio.",
    steps: [
      "Elija puntos con el interruptor de registro en su fila de la lista, o con {btn:rec.add} en los detalles.",
      "Abra {btn:tab.recording}, elija el intervalo en {btn:rec.interval} y pulse {btn:rec.start}.",
      "Mire los gráficos, uno a uno o juntos, y amplíe un periodo.",
      "Guarde los valores con {btn:rec.export} o {btn:xlsx.recExport}.",
    ],
    causes: [
      "No se mide nada: el registro solo funciona después de {btn:rec.start}.",
      "Un punto se pregunta en lugar de notificarse en cada cambio: el dispositivo no lo notifica por sí mismo (COV rechazado).",
      "Huecos en el gráfico: el dispositivo no respondió en ese momento.",
    ],
  },
  reference: {
    title: "Ver qué cambió, comparar dos dispositivos",
    intro: "Una referencia recuerda todos los valores de un dispositivo, para ver más tarde qué cambió, por ejemplo antes y después de la puesta en marcha. Una comparación muestra dos dispositivos uno al lado del otro.",
    steps: [
      "Abra el dispositivo y pulse {btn:ref.save} en {btn:more}.",
      "Más tarde, la vista {btn:view.changed} muestra cada punto cuyo valor cambió desde entonces, con el valor que tenía.",
      "Para comparar con otro dispositivo, pulse {btn:compare.open} en {btn:more} y elíjalo. Los puntos se emparejan por nombre.",
    ],
    causes: [
      "Todo cuenta como cambiado: la referencia pertenece a otro estado de la instalación. Guarde una nueva.",
      "Una comparación muestra sobre todo «no está»: los dos dispositivos nombran sus puntos de forma distinta.",
    ],
  },
  exports: {
    title: "Excel, CSV, imprimir, EDE",
    intro: "Cada lista sale de bacnetio como archivo: CSV y Excel, una página para imprimir o guardar como PDF, y archivos EDE para sistemas de gestión.",
    steps: [
      "La lista de dispositivos y toda la instalación: {btn:export} encima de la lista de dispositivos, con {btn:export.devices}, {btn:xlsx.devicesExport}, {btn:xlsx.site} y {btn:ede.site}. Los dispositivos aún no leídos se leen primero.",
      "Los puntos de un dispositivo: {btn:more} en su lista de puntos, con CSV, Excel, también con las prioridades, EDE y sus textos de estado.",
      "Para imprimir: {btn:print.open} en {btn:more}, con el filtro de la lista. El diálogo de impresión ofrece también Guardar como PDF.",
      "Los valores mantenidos y el registro tienen sus propios botones para CSV y Excel.",
    ],
    causes: [
      "Al archivo EDE le faltan dispositivos: se omiten los que no se pudieron leer, y bacnetio dice cuántos.",
      "Los archivos de la instalación llevan su nombre: dele primero un nombre en {btn:side.allSites}.",
    ],
  },
  capture: {
    title: "Captura para Wireshark",
    intro: "bacnetio graba su propio tráfico BACnet en un archivo que abre Wireshark, sin instalar nada y sin derechos de administrador. En MS/TP contiene cada trama del bus.",
    steps: [
      "Abra {btn:tab.settings} y, en {btn:capture.title}, pulse {btn:capture.start}.",
      "Haga lo que quiera observar, por ejemplo leer un dispositivo.",
      "Pulse {btn:capture.stop}, después {btn:capture.save}, y abra el archivo en Wireshark.",
    ],
    causes: [
      "Falta el tráfico de otros programas: la captura solo contiene lo que bacnetio envía y recibe.",
      "La captura estaba llena: se guardan como máximo 100 000 paquetes, los más antiguos se omiten.",
    ],
  },
  sites: {
    title: "Guardar y abrir instalaciones",
    intro: "Cada búsqueda guarda su instalación: los dispositivos, lo leído y la agrupación. La abre más tarde, también sin la instalación, o la pasa a otros como archivo.",
    steps: [
      "Abra {btn:side.allSites} bajo el nombre de la instalación en la barra lateral.",
      "{btn:sites.rename} da a una instalación un nombre propio; sin él lleva el nombre de la red.",
      "{btn:sites.open} muestra de nuevo una instalación; {btn:sites.file} la guarda, {btn:sites.import} lee una.",
    ],
    causes: [
      "Los valores son viejos: una instalación abierta muestra lo que se leyó entonces. Busque de nuevo para leer la instalación ahora.",
    ],
  },
  plugins: {
    title: "Extensiones",
    intro: "Una extensión añade una comprobación, un informe o una vista, un archivo cada una (….bacnetio-plugin.html). Lee lo que lee bacnetio, pero nunca puede escribir en la instalación.",
    steps: [
      "Abra {btn:tab.settings} y, en {btn:plugins.title}, pulse {btn:plugins.install}. Elija el archivo.",
      "La extensión recibe una página en la barra lateral y puede marcar dispositivos y puntos en las listas.",
      "Desactívela o elimínela en el mismo lugar.",
    ],
    causes: [
      "Instale solo extensiones de confianza: ven todo lo que lee bacnetio.",
    ],
  },
  keys: {
    title: "Teclas",
    intro: "Las teclas funcionan en todas partes, salvo en un campo de entrada.",
  },
};
