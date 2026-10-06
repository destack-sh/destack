import { defineMarker } from "@destack/style";

/** The control of an input group, which the group observes for focus and invalidity. */
export const controlMarker = defineMarker();

/** A multiline control of an input group, which grows the group's height. */
export const multilineMarker = defineMarker();

/** An addon of an input group, which the group observes for its alignment. */
export const addonMarker = defineMarker();
