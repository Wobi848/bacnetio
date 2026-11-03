// The search of the point list. Plain words search name, description and object; a few prefixes
// ask for one thing exactly: `#2796214` the instance, `type:AO` the type, `value:>50` the value.

import { typeShort } from "./grouping.js";
import { objectText, valueWord } from "./i18n.js";

const NUMBER_CONDITION = /^(<=|>=|<|>|=)?(-?\d+(?:[.,]\d+)?)$/;
const COMPARE = {
  "<": (value, limit) => value < limit,
  "<=": (value, limit) => value <= limit,
  ">": (value, limit) => value > limit,
  ">=": (value, limit) => value >= limit,
  "=": (value, limit) => value === limit,
};

/** A value condition: a comparison with a number, else a piece of text the value holds. */
function valueCondition(text) {
  const match = NUMBER_CONDITION.exec(text);
  if (!match) return { text };
  return { operator: match[1] ?? "=", number: Number(match[2].replace(",", ".")) };
}

/** What a search text asks for: words to find, and the conditions of the prefixes. */
export function parseQuery(text) {
  const query = { words: [], instance: null, types: [], value: null };
  for (const part of text.toLowerCase().split(/\s+/).filter(Boolean)) {
    if (/^#\d+$/.test(part)) query.instance = Number(part.slice(1));
    else if (/^type:./.test(part)) query.types.push(...part.slice("type:".length).split(",").filter(Boolean));
    else if (/^value:./.test(part)) query.value = valueCondition(part.slice("value:".length));
    else query.words.push(part);
  }
  return query;
}

function valueMatches(point, condition) {
  const value = point.value ?? "";
  if (condition.text !== undefined) return `${value} ${valueWord(value)}`.toLowerCase().includes(condition.text);
  const number = Number.parseFloat(value);
  return !Number.isNaN(number) && COMPARE[condition.operator](number, condition.number);
}

export function matchesQuery(point, query) {
  if (query.instance !== null && point.instance !== query.instance) return false;
  if (query.types.length && !query.types.includes(typeShort(point.type).toLowerCase())) return false;
  if (query.value && !valueMatches(point, query.value)) return false;
  const text = [point.name, point.description, objectText(point), typeShort(point.type)].join(" ").toLowerCase();
  return query.words.every((word) => text.includes(word));
}
