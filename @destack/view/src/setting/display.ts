import { DeclarationReference } from "@destack/package/declare";
import type { SettingSelection } from "@destack/setting";
import { defineSetting, type SettingDefinition } from "@destack/setting/declare";
import type { DeviceSettingValue, SettingValue } from "@destack/setting/object";
import {
    Appearance,
    Contrast,
    DEFAULT_DISPLAY,
    Density,
    Motion,
    type Display,
    Seed,
    TextSize,
} from "@destack/theme";

/** Where a display preference lives, applied at once. */
const PREFERENCE = {
    scope: "user",
    overrides: ["package", "space", "installation", "device"],
    apply: "immediate",
} satisfies Pick<SettingDefinition, "scope" | "overrides" | "apply">;

/** The appearance a person selects. */
export const appearance = defineSetting({
    name: "appearance",
    title: "Appearance",
    description: "Use the system appearance or select a light or dark interface.",
    schema: Appearance,
    default: "system",
    ...PREFERENCE,
});

/** The text size a person reads at. */
export const textSize = defineSetting({
    name: "textSize",
    title: "Text size",
    description: "Read text smaller or larger.",
    schema: TextSize,
    default: DEFAULT_DISPLAY.textSize,
    ...PREFERENCE,
});

/** How tightly a person packs controls and content. */
export const density = defineSetting({
    name: "density",
    title: "Density",
    description:
        "Pack controls and content compactly or spaciously, or keep each app's own density.",
    schema: Density.nullable(),
    default: DEFAULT_DISPLAY.density,
    ...PREFERENCE,
});

/** The contrast a person reads at. */
export const contrast = defineSetting({
    name: "contrast",
    title: "Contrast",
    description: "Follow the device's contrast or set how far text, lines and graphics stand out.",
    schema: Contrast,
    default: DEFAULT_DISPLAY.contrast,
    ...PREFERENCE,
});

/** The motion a person sees. */
export const motion = defineSetting({
    name: "motion",
    title: "Motion",
    description: "Use the system motion preference, or show or remove transitions.",
    schema: Motion,
    default: DEFAULT_DISPLAY.motion,
    ...PREFERENCE,
});

/** The accent a person selects over every app's own. */
export const accent = defineSetting({
    name: "accent",
    title: "Accent color",
    description: "Use each app's own accent, or one accent color in every app.",
    schema: Seed.nullable(),
    default: DEFAULT_DISPLAY.accent,
    ...PREFERENCE,
});

/** The settings a person's display comes from. */
export const DISPLAY_SETTINGS: readonly DeclarationReference[] = [
    appearance,
    accent,
    contrast,
    density,
    motion,
    textSize,
].map((setting) => setting.reference);

/** Resolve the display of a selection from the values along its scope chain and on its device. */
export function resolveDisplay(
    selection: SettingSelection,
    values: readonly SettingValue[],
    chain: readonly string[],
    device: readonly DeviceSettingValue[],
): Display {
    return {
        appearance: appearance.resolve(selection, values, chain, device).value,
        accent: accent.resolve(selection, values, chain, device).value,
        contrast: contrast.resolve(selection, values, chain, device).value,
        density: density.resolve(selection, values, chain, device).value,
        motion: motion.resolve(selection, values, chain, device).value,
        textSize: textSize.resolve(selection, values, chain, device).value,
    };
}
