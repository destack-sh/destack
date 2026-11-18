import {
    appearance,
    contrast,
    density,
    motion,
    shortcut,
    start,
    textSize,
} from "@destack/view/setting";

/** The settings the person sets in their own scope, which the settings view edits. */
export const PERSON_SETTINGS = [start, shortcut, appearance, textSize, density, contrast, motion];
