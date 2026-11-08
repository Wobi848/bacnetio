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
  keys: {
    title: "Keys",
    intro: "The keys act everywhere except in an input field.",
  },
};
