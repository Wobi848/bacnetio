// The help's guides in English (docs/concept-help.md). {btn:key} names a button or field by the
// interface's own text, {guide:id} another guide by its title; the order and targets are in help.js.

export default {
  find: {
    title: "Find the devices",
    intro: "bacnetio sends a Who-Is and also asks the addresses of the network one by one, so devices that do not answer a Who-Is are found too.",
    steps: [
      "Connect this computer to the plant's network.",
      "Open {btn:side.searchSettings} in the sidebar and choose the {btn:adapter.title} that leads to the plant.",
      "Press {btn:searchWithSettings}. The devices appear in the list as they answer.",
      "Choose a device: its facts stand beside the list. {btn:dev.openPoints} reads its data points.",
    ],
    causes: [
      "No device found: check the network adapter. A laptop with Wi-Fi and a cable has two.",
      "A firewall on this computer can block BACnet (UDP port 47808). On Windows, allow bacnetio when it asks at the first start.",
      "The plant uses another port: enter it under {btn:port}.",
      "The plant lies in another subnet: see {guide:vpn}.",
      "A single device is missing: {btn:locate.short} opens it by its address.",
    ],
  },
  vpn: {
    title: "Reach a plant behind a VPN",
    intro: "A VPN does not pass on broadcasts, so a Who-Is does not reach the plant. bacnetio then asks every address of the plant's subnet directly, or registers at the plant's BBMD.",
    steps: [
      "Connect the VPN.",
      "Open {btn:side.searchSettings} and choose the VPN's adapter as {btn:adapter.title}.",
      "Enter the plant's subnet under {btn:subnet}, for example 192.168.10.0/24. If the plant has a BBMD, enter its address under {btn:bbmd} instead.",
      "Press {btn:searchWithSettings}.",
    ],
    causes: [
      "Nothing found: the VPN may pass only some ports. BACnet needs UDP 47808, or the port the plant uses under {btn:port}.",
      "The subnet to enter is the plant's, not the VPN's own. Whoever runs the plant's network knows it.",
      "A large subnet takes long: ask only the part where the controllers are, for example a /24.",
      "A single device known by its address: {btn:locate.short} opens it directly.",
    ],
  },
  write: {
    title: "Write and release a value",
    intro: "A value you write acts in the running plant at once. bacnetio lists every value you hold under {btn:tab.log}, also after a restart, so you can release it before you leave.",
    steps: [
      "Open the device and choose the data point.",
      "In its details, press {btn:changeValue}, then {btn:unlock.allow}. bacnetio asks this once per session.",
      "Enter the value and choose the {btn:priority}; 8 (manual operator) is the usual one. Press {btn:writeValue}.",
      "When you are done, release the value with the release button of your priority in the details, or with {btn:release} under {btn:tab.log}.",
      "Before you leave the site, {btn:leave} under {btn:tab.log} offers to release everything you still hold.",
    ],
    causes: [
      "bacnetio asks before writing: a higher priority holds another value, so yours would not act. The priorities in the details show who holds what.",
      "The device refused: bacnetio shows its reason, for example no write access or a wrong password, and keeps it in the log.",
      "The value goes back: another system, such as the building management, writes it again at the same or a higher priority.",
      "The point is overridden: a hand switch on site acts, and the priorities act again only once it ends.",
    ],
  },
  points: {
    title: "Read a device's points",
    intro: "A device's data points stand in a list, grouped by plant part where the names are paths, otherwise by kind. The chosen point stands beside it with its value live, who holds it and every property.",
    steps: [
      "Choose the device in the list and press {btn:dev.openPoints}. bacnetio reads its points and shows how far it is.",
      "Find points with {btn:filter}, or show only some with the views, such as {btn:view.faults} or {btn:view.overridden}.",
      "Choose a point: its value follows live, its priorities show who holds it, and every property of the object stands below.",
      "{btn:more} holds what is used now and then: the exports, {btn:compare.open}, {btn:print.open}, {btn:grouping.open} and {btn:ref.save}.",
    ],
    causes: [
      "Reading takes long: a device without ReadPropertyMultiple, or one on MS/TP, is asked one value at a time.",
      "A point shows an error: the device refused to give that property. The details name its reason.",
      "The groups do not fit the plant: {btn:grouping.open} groups the site's points by a pattern of your own.",
    ],
  },
  bbmd: {
    title: "A BBMD in another subnet",
    intro: "A BBMD passes broadcasts between subnets. bacnetio registers at it as a foreign device, and the BBMD passes the Who-Is on to the plant.",
    steps: [
      "Open {btn:side.searchSettings}.",
      "Enter the BBMD's address under {btn:bbmd}, for example 10.20.0.10, with :port if it does not use 47808.",
      "Press {btn:searchWithSettings}.",
      "To see how the BBMDs are set up, open a BBMD's device details and press {btn:bbmd.read}. bacnetio only reads its tables and never changes them.",
    ],
    causes: [
      "The BBMD refused the registration: it may not take foreign devices, or its table of them is full.",
      "The BBMD does not answer: check its address and port, and that a firewall lets UDP through.",
      "Only part of the plant answers: the BBMDs of the other subnets are missing from each other's tables. {btn:bbmd.read} shows them.",
    ],
  },
  mstp: {
    title: "MS/TP over a USB RS-485 adapter",
    intro: "With a USB RS-485 adapter, bacnetio works directly on an MS/TP trunk (Windows and Linux). It first listens without sending anything, then joins the token ring.",
    steps: [
      "Connect the adapter to the trunk (A to A, B to B) and plug it into this computer.",
      "Open {btn:side.searchSettings} and choose {btn:link.mstp} under {btn:link.title}. Choose the adapter under {btn:mstp.port}.",
      "Leave {btn:mstp.speed} at {btn:mstp.findSpeed} and press {btn:mstp.listen}. bacnetio finds the speed, the stations and a free address for itself.",
      "Press {btn:mstp.join}. The devices on the trunk are read, written and recorded as on IP.",
      "When you are done, press {btn:mstp.leave} in the sidebar: it frees the port.",
    ],
    causes: [
      "No port found: plug in the adapter and press {btn:mstp.lookAgain}. On Linux your user must be in the group dialout once.",
      "Nothing heard: check the wiring (A and B), the termination and that the adapter sits on the trunk.",
      "Only damaged frames: A and B are possibly swapped, the termination is missing, or the trunk runs at a speed bacnetio does not try.",
      "No master takes bacnetio into the token ring: listen again and join with the address offered.",
    ],
  },
  watch: {
    title: "Keep an eye on values",
    intro: "The watch list shows points of several devices together, with their values live.",
    steps: [
      "Choose a point in the workspace and press {btn:watch.add}, or switch on the eye in its row of the list.",
      "Open {btn:tab.watch} in the sidebar: the values follow live.",
      "Choose a row to open the point in its device.",
    ],
    causes: [
      "A value stays old: the device does not report changes (COV) and is asked again at intervals.",
    ],
  },
  record: {
    title: "Record values",
    intro: "A recording measures chosen values at an interval or on every change, as charts and as CSV or Excel. It is backed up every minute and survives a restart.",
    steps: [
      "Choose points with the record switch in their row of the list, or with {btn:rec.add} in the details.",
      "Open {btn:tab.recording}, choose the interval under {btn:rec.interval} and press {btn:rec.start}.",
      "Look at the charts, one by one or together, and zoom into a time.",
      "Save the values with {btn:rec.export} or {btn:xlsx.recExport}.",
    ],
    causes: [
      "Nothing is measured: the recording only runs after {btn:rec.start}.",
      "A point is polled instead of on change: the device does not report it by itself (COV refused).",
      "Gaps in the chart: the device did not answer at that time.",
    ],
  },
  reference: {
    title: "See what changed, compare two devices",
    intro: "A reference remembers all values of a device, so you see later what changed, for example before and after commissioning. A comparison shows two devices side by side.",
    steps: [
      "Open the device and press {btn:ref.save} under {btn:more}.",
      "Later, the view {btn:view.changed} shows every point whose value changed since then, with the value it had.",
      "To compare with another device, press {btn:compare.open} under {btn:more} and choose it. The points are matched by name.",
    ],
    causes: [
      "Everything counts as changed: the reference belongs to another state of the plant. Save a new one.",
      "A comparison shows mostly «not there»: the two devices name their points differently.",
    ],
  },
  exports: {
    title: "Excel, CSV, print, EDE",
    intro: "Every list leaves bacnetio as a file: CSV and Excel, a page to print or save as PDF, and EDE files for management systems.",
    steps: [
      "The device list and the whole site: {btn:export} above the device list, with {btn:export.devices}, {btn:xlsx.devicesExport}, {btn:xlsx.site} and {btn:ede.site}. Devices not read yet are read first.",
      "A device's points: {btn:more} in its point list, with CSV, Excel, also with priorities, EDE and its state texts.",
      "To print: {btn:print.open} under {btn:more}, with the list's filter. The print dialog also offers Save as PDF.",
      "The values held and the recording have their own buttons for CSV and Excel.",
    ],
    causes: [
      "The EDE file lacks devices: those that could not be read are left out, and bacnetio says how many.",
      "The site's files are named after the site: give it a name under {btn:side.allSites} first.",
    ],
  },
  capture: {
    title: "Capture for Wireshark",
    intro: "bacnetio records its own BACnet traffic as a file that Wireshark opens, without installing anything and without administrator rights. On MS/TP every frame on the trunk is in it.",
    steps: [
      "Open {btn:tab.settings} and, in {btn:capture.title}, press {btn:capture.start}.",
      "Do what you want to look at, for example read a device.",
      "Press {btn:capture.stop}, then {btn:capture.save}, and open the file in Wireshark.",
    ],
    causes: [
      "Traffic of other programs is missing: the capture holds only what bacnetio sends and receives.",
      "The capture was full: at most 100 000 packets are kept, the oldest are left out.",
    ],
  },
  sites: {
    title: "Keep and open sites",
    intro: "Every search keeps its site: the devices, what was read and the grouping. You open it again later, also without the plant, or hand it on as a file.",
    steps: [
      "Open {btn:side.allSites} under the site's name in the sidebar.",
      "{btn:sites.rename} gives a site your own name; without one it is named after the network.",
      "{btn:sites.open} shows a site again; {btn:sites.file} saves it, {btn:sites.import} reads one.",
    ],
    causes: [
      "Values are old: an opened site shows what was read then. Search again to read the plant now.",
    ],
  },
  plugins: {
    title: "Extensions",
    intro: "An extension adds a check, a report or a view, one file each (….bacnetio-plugin.html). It reads what bacnetio reads, but it can never write to the plant.",
    steps: [
      "Open {btn:tab.settings} and, in {btn:plugins.title}, press {btn:plugins.install}. Choose the file.",
      "The extension gets a page in the sidebar and can mark devices and points in the lists.",
      "Switch it off or remove it in the same place.",
    ],
    causes: [
      "Install only extensions you trust: they see everything bacnetio reads.",
    ],
  },
  keys: {
    title: "Keys",
    intro: "The keys act everywhere except in an input field.",
  },
};
