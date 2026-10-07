import type { SettingReference, SettingSelection } from "@destack/setting";
import type { SettingValue } from "@destack/setting/object";
import type { Appearance, Preferences } from "@destack/theme";
import { accent } from "./accent.ts";
import { appearance } from "./appearance.ts";
import { contrast } from "./contrast.ts";
import { density } from "./density.ts";
import { motion } from "./motion.ts";
import { textSize } from "./text.ts";

/** The appearance and preferences a person sees an interface in. */
export interface Display {
    /** The appearance. */
    readonly appearance: Appearance;
    /** The display preferences. */
    readonly preferences: Preferences;
}

/** The settings a person's display comes from. */
export const DISPLAY_SETTINGS: readonly SettingReference[] = [
    appearance,
    accent,
    contrast,
    density,
    motion,
    textSize,
].map((setting) => setting.reference);

/** Resolve the display of a selection from the values placed along its scope chain, nearest scope first. */
export function resolveDisplay(
    selection: SettingSelection,
    values: readonly SettingValue[],
    chain: readonly string[],
): Display {
    return {
        appearance: appearance.resolve(selection, values, chain).value,
        preferences: {
            accent: accent.resolve(selection, values, chain).value,
            contrast: contrast.resolve(selection, values, chain).value,
            density: density.resolve(selection, values, chain).value,
            motion: motion.resolve(selection, values, chain).value,
            textSize: textSize.resolve(selection, values, chain).value,
        },
    };
}
