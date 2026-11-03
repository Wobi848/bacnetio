// The plant view a device carries itself: its Structured View objects (type 29) with their
// Subordinate_List, flattened into tree nodes. A device without them keeps the grouping by object
// names (grouping.js), as docs/concept-explorer.md section 2 asks.

import { pointKey } from "./format.js";

const VIEW_TYPE = 29;
/** Nodes below this depth keep the indentation of the last step (list.css has level-2 to level-5). */
const DEEPEST = 3;

export const viewKey = (instance) => `view:${instance}`;
/** The node for the objects no view holds. */
export const OUTSIDE = "view:outside";
export const isViewNode = (node) => Boolean(node?.startsWith("view:"));

const onThisDevice = (item) => item.device === null || item.device === undefined;

/** A view's own name, else what the view above it called it, else "Structured view 2". */
const viewTitle = (view, annotation) => view.name || annotation || view.object;

/**
 * The views in the order the device lists them, each
 * `{ key, title, depth, objects: [pointKey], elsewhere, error }`. `objects` holds everything below
 * the node, through its nested views, so a branch counts what hangs under it. `elsewhere` counts
 * subordinates on another device, which this device's table cannot show.
 */
export function plantNodes(views = []) {
  const byInstance = new Map(views.map((view) => [view.instance, view]));
  const held = new Set();
  for (const view of views) {
    for (const item of view.subordinates ?? []) {
      if (item.type === VIEW_TYPE && onThisDevice(item) && byInstance.has(item.instance)) held.add(item.instance);
    }
  }
  const nodes = [];
  // `above` keeps a view that holds itself, directly or around a corner, from being walked twice
  const walk = (view, depth, title, above) => {
    const node = {
      key: viewKey(view.instance),
      title,
      depth: Math.min(depth, DEEPEST),
      objects: [],
      elsewhere: 0,
      error: view.error ?? null,
    };
    nodes.push(node);
    const below = [];
    for (const item of view.subordinates ?? []) {
      if (!onThisDevice(item)) { node.elsewhere += 1; continue; }
      const child = item.type === VIEW_TYPE ? byInstance.get(item.instance) : null;
      if (!child) { node.objects.push(pointKey(item)); continue; }
      if (!above.has(child.instance)) below.push([child, item.annotation]);
    }
    for (const [child, annotation] of below) {
      node.objects.push(...walk(child, depth + 1, viewTitle(child, annotation), new Set([...above, child.instance])));
    }
    return node.objects;
  };
  // all views are held only when they hold each other; then each one starts a branch of its own
  const roots = views.filter((view) => !held.has(view.instance));
  for (const view of roots.length ? roots : views) {
    walk(view, 0, viewTitle(view, null), new Set([view.instance]));
  }
  return nodes;
}

/**
 * The device's points that no view holds, so nothing is lost. The views themselves are left out:
 * they are the branches of the tree, not rows of a plant part.
 */
export function outsidePoints(nodes, points) {
  const inside = new Set(nodes.flatMap((node) => node.objects));
  return points.filter((point) => point.type !== VIEW_TYPE && !inside.has(pointKey(point)));
}

/** The points of one plant node, in the order the views name them. */
export function nodePoints(nodes, node, points) {
  if (node === OUTSIDE) return outsidePoints(nodes, points);
  const found = nodes.find((candidate) => candidate.key === node);
  if (!found) return [];
  const byKey = new Map(points.map((point) => [pointKey(point), point]));
  // a view may name an object the device does not list; only what was read can be shown
  return found.objects.map((key) => byKey.get(key)).filter(Boolean);
}
