# Field test checklist

What the maintainer checks on real plants, because the simulators cannot show it. Run each test side
by side with the K&P BACnet Browser or YABE and note the result here. Before a first release, every
line needs a result.

Reading is safe. **Sections 5, 8 and 8b write to a device**: do them on a point you may change, and
release what you wrote before you leave.

## 1. DDC4000 over VPN (HOOC)

| Check | Expected | Result |
| --- | --- | --- |
| Search on the plant subnet, with a stopwatch | Device shows within 1 s; devices and object count within 10 s, as fast as the K&P browser | |
| Compare the object types with the K&P browser | Same counts. **Done 1.11.2026 at the DDC4000: all 11 types and the sum of 170 match** | yes |
| Open the device, then *Show request log* in its details | No line says timeout; the first line after the limits says the list came from the search | |
| *Read again*, then the request log | The object list is read from the device, without a timeout | |
| A trend log: chart and CSV | Reads without "abort reason=10" | |
| Open every group | Names and values complete, grouped by plant part as in the K&P browser | |

Counts leave out the device object itself, so a device of 170 objects plus its device object shows 170 everywhere. If a count ever differs, keep the request log of *Read again* and a Wireshark capture.

**Adaptive timeouts are not built.** rusty-bacnet has one timeout per client, not per device or
request. All requests wait 2 s. If the request log shows that a VPN needs more, or that requests
routinely take longer than that, this has to be solved in the library first. A request that fails is
made smaller at once, which is built.

**A slow direct answer:** while searching, each address gets 1.5 s to answer the direct question. A
device that answers later, and does not answer Who-Is either, is not found. Note it here if that
happens over the VPN; *Device by IP* still finds it.

## 2. Windows with the K&P service on port 47808

| Check | Expected | Result |
| --- | --- | --- |
| Search while the K&P service runs | A note says that another program holds the port and that another one was used; bacnetio takes that port and does not stop | |
| Devices that only answer by broadcast | Named as missing in that note; devices found directly still work | |
| Stop the K&P service, search again | The note is gone; the devices also answer Who-Is | |
| Without administrator rights, firewall on | Devices are found by asking directly, and a note says broadcast answers are blocked | |
| Virtual adapters (VirtualBox, Hyper-V, VPN) | Not the default adapter; the list shows their description | |

## 3. Certified BACnet controller

| Check | Expected | Result |
| --- | --- | --- |
| Object count and names | Same as the K&P browser or YABE | |
| Values, units, state texts, status flags | Same as the K&P browser or YABE | |
| *Show technical details* | All properties, none shown as unreadable by mistake | |
| Active alarms (device details) | The same alarms as the K&P browser, or a clear sentence that the device gives none | |
| Live value | Says *Live*, changes when the plant changes; COV where the controller offers it | |
| Trend log | Same records as the K&P browser | |

## 4. MS/TP behind BACnet routers

| Check | Expected | Result |
| --- | --- | --- |
| Search | Routers and their networks are listed; devices show with network number and MAC | |
| Object lists during the search | Several slow devices do not stall the list; the count fills in device by device | |
| Time for a search with many MS/TP devices | Note the time; a device that stays silent shows *not read* and does not stop the search | |
| Open a device behind a router, read and search its points | Complete, without timeouts | |
| Traffic while the search reads object lists | The MS/TP trunk stays usable for the plant; note it if not | |

| *Device by IP …* with router address, network, MAC and device ID | The device opens and reads; this has only been tested on its input so far | |

## 4b. MS/TP directly over a USB RS-485 adapter

Listening sends nothing. **Joining takes part in the trunk's timing**: do it on a trunk where a
short disturbance does no harm, and leave the trunk when done (`docs/concept-mstp.md`). The
maintainer's adapter: WCH CH343 (`/dev/ttyACM0` on Linux, a COM port on Windows).

| Check | Expected | Result |
| --- | --- | --- |
| *Search settings → MS/TP*: the port list | The adapter first, with its name | |
| *Listen to the trunk*, speed found by listening | The trunk's speed within 10 s; the stations as YABE or the K&P browser show them | |
| Listen with A and B swapped, or the wrong speed chosen | "Only damaged frames …" or "Nothing heard …", nothing sent | |
| *Join the trunk and search* at the proposed address, with a stopwatch | In the ring within a few seconds; every device of the trunk found | |
| Open a device, read all its points | Complete, without timeouts in the request log | |
| Write and release a point you may change | As on IP; the value held while written, gone after the release | |
| *Leave the trunk*, then listen again | The port is free; the stations as before, bacnetio's address no longer heard | |
| On Windows: the same on a COM port | As on Linux, without administrator rights | |

## 5. Writing, on a point you may change

| Check | Expected | Result |
| --- | --- | --- |
| Write without the unlock | Not possible; the unlock explains what happens | |
| An Integer Value, Large Analog Value or CharacterString Value object, if the plant has one | Takes the value and releases it; only tested on its encoding so far | |
| Write at priority 8 | The priority array, the details and the list show the new value at once; the log has an entry | |
| Close the page while holding a value | A warning appears | |
| Restart bacnetio | The value is still listed as held and can be released | |
| Release | The value returns to what the plant sets; the log has an entry | |
| *CSV with priorities* | The winning priority and every set slot match the K&P browser | |
| *Read priorities* | The same winners as the K&P browser | |
| Change out of service in the technical details, and back | The device takes it; the log has both entries | |
| A device that reports COV unasked | Its value says *Live* without a subscription | |

## 6. Exports

| Check | Expected | Result |
| --- | --- | --- |
| *EDE* for one device | Opens in the management system or tool you use for EDE files | |
| *EDE state texts* with the EDE file | The tool shows the multi-state texts | |
| *EDE (all devices)* | Contains every device; names a number of devices if some could not be read | |
| CSV in Excel | Umlauts and units read correctly, in your Excel's language | |

## 6b. Workspace

| Check | Expected | Result |
| --- | --- | --- |
| Tree on a real site: plant parts from the names | Grouped as in the K&P browser's name view | |
| Write at a priority below one that is set | The warning names the priority that wins | |
| *Site problems* after *Read all devices* | The same faults and overrides as the K&P browser | |
| *Read all devices* on a large site, MS/TP too | Note the time; the plant stays usable | |
| Search across the site, watch list over a shift | Finds what you look for; values stay live | |

## 7. Site memory

| Check | Expected | Result |
| --- | --- | --- |
| Search and open a device, close bacnetio, start it again | Devices and points show at once, marked as remembered | |
| *Search again* | The mark disappears, values are live | |

| A second subnet, then *Settings → Sites* | Both listed; opening one shows it as remembered | |
| Save a site to a file, load it on another computer | The same devices and points, marked as remembered | |

## 8. Device clock, on a device whose clock you may set

| Check | Expected | Result |
| --- | --- | --- |
| Send the time | Device time matches this computer; the log has an entry. The device does not answer this request, so check its time | |

## 8b. Device ID, on a device whose ID you may change

| Check | Expected | Result |
| --- | --- | --- |
| Change the ID in the device details | The device takes it, the search runs, the device shows under the new ID, the log has an entry. Or it refuses, and the reason shows | |
| Change it back | Same, and the management system still finds the device | |

## 8c. Commands, on a device you may restart

| Check | Expected | Result |
| --- | --- | --- |
| Warm start, cold start, with the device's password | The device restarts; a wrong password shows the refusal | |
| Stop communication for 2 minutes, resume | The device goes silent and comes back; a device after ASHRAE 135-2020 refuses and says to stop its sending instead | |
| *Stop its sending* for 2 minutes, resume | The device still answers reads; no COV or alarms come from it until resumed | |
| Acknowledge an alarm | It no longer says not acknowledged, in the K&P browser too | |
| A device with this computer in a notification class | Its alarm messages show in *Site problems* | |
| A schedule and a calendar | Week, exceptions and dates as in the K&P browser | |
| Change a day of a schedule, and back | The device runs by it; the K&P browser shows the same | |
| Who-Has by a name on a device not read yet | The device answers; a click opens the point | |
| Change a property such as COV increment | Taken or refused with the reason; logged | |

## 9. Usability

| Check | Expected | Result |
| --- | --- | --- |
| A colleague without help: find a device, read a value, write and release it (concept section 3.1) | Done without asking | |

## 10. Other things only real use can show

| Check | Expected | Result |
| --- | --- | --- |
| Android: phone, USB or Ethernet adapter (concept section 9) | Same interface; finds a device | |
| French, Italian and Spanish: someone from the trade reviews `?lang=fr`, `?lang=it`, `?lang=es` | Terms are right; only then do they enter the menu | |
