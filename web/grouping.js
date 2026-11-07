// Data points grouped by plant part. Many controllers name objects as paths
// ("/21/000/09/11/S337.01/5101"): the segments all names share are left out, the rest of the
// path up to the last segment becomes the group (the module), the last segment the name. Without
// paths the groups are the object kinds. A site can bring its own pattern instead
// (docs/concept-toolbox.md part 4).

const SEPARATOR = /[/\\']/;
/** Object kinds by type number, for devices whose names are no paths. */
const KINDS = [["inputs", [0, 3, 13]], ["outputs", [1, 4, 14]], ["values", [2, 5, 19]]];

const segments = (name) => (name ?? "").split(SEPARATOR).filter(Boolean);

function commonLength(lists) {
  let length = 0;
  while (lists.every((list) => list.length > length + 1 && list[length] === lists[0][length])) length++;
  return length;
}

const kindOf = (point) => KINDS.find(([, types]) => types.includes(point.type))?.[0] ?? "other";

/** Groups hold this many points on average at least, where the names allow it. */
const POINTS_PER_PART = 3;

/**
 * How many segments after the shared start name a plant part: the deepest that gives parts of
 * several points. Some vendors (K&P) put every point on a path of its own, which would make one
 * part per point; the rest of the path then stays in the row's name.
 */
function partDepth(paths, skip) {
  const deepest = Math.max(...paths.map((list) => list.length - skip - 1));
  for (let depth = deepest; depth >= 1; depth--) {
    const parts = new Set(paths.map((list) => list.slice(skip, Math.min(skip + depth, list.length - 1)).join("/")));
    if (paths.length / parts.size >= POINTS_PER_PART) return depth;
  }
  return deepest;
}

/** A site's own grouping pattern as a regular expression; null for none. Throws when it is no
 * valid regular expression, with the browser's reason. */
export const compilePattern = (text) => (text ? new RegExp(text) : null);

/** The pattern of the site open, or null: one that does not compile groups by itself. */
export function sitePattern(site) {
  try {
    return compilePattern(site?.grouping);
  } catch {
    return null;
  }
}

/** Where a name matches the pattern: its brackets (or the whole match without any) are the part,
 * the rest of the name is the point's name. */
export function splitByPattern(name, pattern) {
  const text = name ?? "";
  const match = pattern.exec(text);
  if (!match) return null;
  const part = match.length > 1 ? match.slice(1).filter(Boolean).join("/") : match[0];
  if (!part) return null;
  const rest = (text.slice(0, match.index) + text.slice(match.index + match[0].length)).replace(/^[\s_./\\-]+/, "");
  return { part, name: rest || text };
}

function groupByPattern(points, pattern) {
  const groups = new Map();
  const other = [];
  for (const point of points) {
    const split = splitByPattern(point.name, pattern);
    if (!split) {
      other.push({ point, name: point.name ?? "" });
      continue;
    }
    if (!groups.has(split.part)) groups.set(split.part, { key: `part:${split.part}`, title: split.part, kind: false, items: [] });
    groups.get(split.part).items.push({ point, name: split.name });
  }
  const result = [...groups.values()];
  if (other.length) result.push({ key: "part:", title: "other", kind: true, items: other });
  return result;
}

/**
 * Groups in the order of the object list: `{ key, title, kind, items: [{ point, name }] }`. `kind`
 * is true when `title` is a text key (inputs, outputs, …) rather than a name from the device.
 * With the site's own `pattern` the parts come from it.
 */
export function groupPoints(points, pattern = null) {
  if (pattern) return groupByPattern(points, pattern);
  const paths = points.map((point) => segments(point.name));
  const deep = paths.filter((list) => list.length > 1);
  // while a device is read, names come in batches: decide by the names known so far
  const named = paths.filter((list) => list.length > 0).length;
  if (!deep.length || deep.length * 2 < named) return groupByKind(points);
  const skip = commonLength(deep);
  const depth = partDepth(deep, skip);
  const groups = new Map();
  points.forEach((point, index) => {
    const list = paths[index];
    const inPart = list.length > skip + 1;
    const end = Math.min(skip + depth, list.length - 1);
    const title = inPart ? list.slice(skip, end).join("/") : "";
    const name = inPart ? list.slice(end).join("/") : list.at(-1) ?? "";
    if (!groups.has(title)) groups.set(title, { key: `part:${title}`, title, kind: false, items: [] });
    groups.get(title).items.push({ point, name });
  });
  const other = groups.get("");
  groups.delete("");
  const result = [...groups.values()];
  if (other) result.push({ ...other, title: "other", kind: true });
  return result;
}

/** Inputs, outputs and values together; everything else by its type, so that schedules, trend logs
 * and files do not end up in one long list. */
function groupByKind(points) {
  const groups = KINDS.map(([kind]) => ({ key: `kind:${kind}`, title: kind, kind: true, items: [] }));
  const byType = new Map();
  for (const point of points) {
    const kind = kindOf(point);
    if (kind !== "other") {
      groups.find((group) => group.title === kind).items.push({ point, name: point.name ?? "" });
      continue;
    }
    if (!byType.has(point.type)) byType.set(point.type, { key: `kind:type:${point.type}`, title: "other", kind: true, type: point.type, items: [] });
    byType.get(point.type).items.push({ point, name: point.name ?? "" });
  }
  const others = [...byType.values()].sort((first, second) => first.type - second.type);
  return [...groups, ...others].filter((group) => group.items.length);
}

/** Short names of object types, as technicians write them. */
const TYPE_SHORT = { 0: "AI", 1: "AO", 2: "AV", 3: "BI", 4: "BO", 5: "BV", 6: "CAL", 8: "DEV", 10: "FILE", 13: "MSI", 14: "MSO", 9: "EE", 12: "LOOP", 15: "NC", 16: "PRG", 17: "SCH", 19: "MSV", 20: "TL", 29: "SV", 56: "NP" };

export const typeShort = (type) => TYPE_SHORT[type] ?? String(type);
