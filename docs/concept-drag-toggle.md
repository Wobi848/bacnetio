# Concept: switch many at once by dragging

Asked for on 4 November 2026, with Blender as the model: press on a switch and drag over others,
and they all take the same state.

## What the user does

- In the point list each row has two switches: *watch* (eye) and *record* (chart). Press on one of
  them and, keeping the button or finger down, move up or down over the same switch in other rows.
- The first switch flips (on becomes off, off becomes on). Every switch of the same kind the
  pointer passes takes that same new state; it does not flip back and forth. Releasing ends it.
- Moving back over a row already passed leaves it as it is; there is no undo while dragging.
- Watch and record never mix: a drag that starts on an eye only changes eyes.
- A plain click stays a plain click.

## Where it applies

1. The watch and record switches in the point list (first step).
2. Later and only if asked: the object type checkboxes and other lists of switches.

Selecting rows by dragging (to act on all of them at once, as Blender's box select) is a separate
idea and not part of this.

## How it works

- With the mouse only (decided 4 November: "nur auf PC, nicht für Android"). Pointer events on the
  list (`pointerdown`, `pointermove` over the rows, `pointerup`) that start with `pointerType`
  `mouse`; on a touch screen a drag scrolls as before and a tap is a plain click.
- Each row passed is switched at once through the same functions as a click (watch list,
  recording), so nothing new is stored and the rules stay the same, such as which types can be
  recorded.
- With the keyboard nothing changes: each switch is still a button.

## Decided

- 4 November: built for the point list first; other places only when asked.
