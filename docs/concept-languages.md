# Concept: the same languages as mbusio

Date: 2026-11-01 · Status: **built**; French, Italian and Spanish are translated, not reviewed and
not in the menu

Goal: English, German, French, Italian and Spanish, as in mbusio
(`../mbusio/docs/concept-languages.md`). A further language is a matter of translating one file.
German and English behave exactly as before.

## 1. Where texts live

| Part | How it works |
| --- | --- |
| Interface | One file per language in `web/lang/` with the same keys: interface texts, notes from the server by code, known server errors by pattern, object type names, locale. `web/i18n.js` picks the language and falls back to English, then to the key. |
| Server notes | The server sends a `code` and English `text` (`docs/api.md`); the interface translates by code. A code without a translation shows the server's English text. |
| Server errors | English from the server; the interface translates the known ones by pattern, the rest stay English. |
| Exports (CSV) | English column heads and a decimal point, unchanged: the files feed Excel macros and scripts that read them by name. A later step may add `?lang=`, then for every language at once. |
| Command line | English only. |
| Android | `res/values` (English), `values-de`, `values-fr`, `values-it`, `values-es`; Android picks by the phone's language. |

## 2. Formats

One place per language in its file: the locale for dates and times, and whether numbers take a
decimal comma.

| Language | Locale | Date | Decimal mark |
| --- | --- | --- | --- |
| en | en-GB | 01/11/2026 | `.` |
| de | de-CH | 01.11.2026 | `,` |
| fr | fr-CH | 01.11.2026 | `,` |
| it | it-CH | 01.11.2026 | `,` |
| es | es-ES | 01/11/2026 | `,` |

Values typed by the user accept both marks (`write.rs` already reads `21,5`).

## 3. Choosing the language

- Without a choice the system's language is taken if it is in the menu, otherwise English.
- The menu (Settings) shows English and German. French, Italian and Spanish join it only after
  review by someone from the trade (AGENTS.md).
- Reviewers open a language with `?lang=fr`, `?lang=it` or `?lang=es`; it is kept like a choice
  from the menu until another language is chosen.

## 4. Terms

Translators work from one glossary so the same word stays the same word. BACnet terms keep the
standard's wording where the trade uses it (object and property names, COV, BBMD, priority array).
**Draft, needs review by someone from the trade.**

| English | German | French | Italian | Spanish |
| --- | --- | --- | --- | --- |
| Device | Gerät | Appareil | Dispositivo | Dispositivo |
| Data point | Datenpunkt | Point de données | Punto dati | Punto de datos |
| Present value | Aktueller Wert | Valeur actuelle | Valore attuale | Valor actual |
| Analog input / output / value | Analogeingang / -ausgang / -wert | Entrée / sortie / valeur analogique | Ingresso / uscita / valore analogico | Entrada / salida / valor analógico |
| Binary … | Binär… | … binaire | … binario | … binario |
| Multi-state … | Mehrstufen… | … multi-états | … multistato | … multiestado |
| Trend log | Trendaufzeichnung | Journal de tendance | Registro di tendenza | Registro de tendencia |
| Notification class | Meldeklasse | Classe de notification | Classe di notifica | Clase de notificación |
| Priority array | Prioritätenliste | Tableau de priorités | Matrice delle priorità | Matriz de prioridades |
| Relinquish default | Standardwert | Valeur par défaut | Valore predefinito | Valor por defecto |
| Release | Freigeben | Libérer | Rilasciare | Liberar |
| Overridden | Übersteuert | Forcé | Forzato | Forzado |
| Out of service | Ausser Betrieb | Hors service | Fuori servizio | Fuera de servicio |
| Fault | Störung | Défaut | Guasto | Fallo |
| Alarm | Alarm | Alarme | Allarme | Alarma |
| Search (for devices) | Suchen | Rechercher | Cercare | Buscar |
| Adapter | Adapter | Adaptateur | Scheda di rete | Adaptador |
| Subnet | Subnetz | Sous-réseau | Sottorete | Subred |
| Recording | Aufzeichnung | Enregistrement | Registrazione | Registro |
| Capture (Wireshark) | Mitschnitt | Capture | Cattura | Captura |
| Reference | Referenz | Référence | Riferimento | Referencia |
| Log | Protokoll | Journal | Registro | Registro |
| Plant | Anlage | Installation | Impianto | Instalación |

Protocol words (BACnet, BBMD, COV, Who-Is, I-Am, MS/TP, APDU) stay as they are.

## 5. Tests

- `tests/web/ui.test.js`: every language has exactly the keys of English, every note code and
  object type of English, and every key the interface uses exists.
- `tests/ui/27-layout.test.mjs`: every page in every language at 390, 1000 and 1250 px, light and
  dark. French and Spanish are longer than English.
- `tests/ui/07-settings.test.mjs`: the menu holds English and German only; `?lang=fr` opens French.

## 6. Open

- Who reviews French, Italian and Spanish?
