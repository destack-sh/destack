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
    ThemeDefinition,
    Typeface,
} from "@destack/theme";
import { t } from "@destack/locale";

/** Where a person's look lives, the same in every app and applied at once. */
const LOOK = {
    scope: "user",
    overrides: ["space", "device"],
    apply: "immediate",
} satisfies Pick<SettingDefinition, "scope" | "overrides" | "apply">;

/** Where a display preference lives, applied at once. */
const PREFERENCE = {
    scope: "user",
    overrides: ["package", "space", "installation", "device"],
    apply: "immediate",
} satisfies Pick<SettingDefinition, "scope" | "overrides" | "apply">;

/** The appearance a person selects. */
export const appearance = defineSetting({
    name: "appearance",
    title: t`Appearance`,
    description: t`Use the system appearance or select a light or dark interface.`,
    schema: Appearance,
    default: "system",
    ...PREFERENCE,
});

/** The text size a person reads at. */
export const textSize = defineSetting({
    name: "textSize",
    title: t`Text size`,
    description: t`Read text smaller or larger.`,
    schema: TextSize,
    default: DEFAULT_DISPLAY.textSize,
    ...PREFERENCE,
});

/** How tightly a person packs controls and content. */
export const density = defineSetting({
    name: "density",
    title: t`Density`,
    description: t`Pack controls and content compactly or spaciously, or use the theme's density.`,
    schema: Density.nullable(),
    default: DEFAULT_DISPLAY.density,
    ...PREFERENCE,
});

/** The contrast a person reads at. */
export const contrast = defineSetting({
    name: "contrast",
    title: t`Contrast`,
    description: t`Follow the device's contrast or set how far text, lines and graphics stand out.`,
    schema: Contrast,
    default: DEFAULT_DISPLAY.contrast,
    ...PREFERENCE,
});

/** The motion a person sees. */
export const motion = defineSetting({
    name: "motion",
    title: t`Motion`,
    description: t`Use the system motion preference, or show or remove transitions.`,
    schema: Motion,
    default: DEFAULT_DISPLAY.motion,
    ...PREFERENCE,
});

/** The theme a person sees in every app, whole, or null for Destack's. */
export const theme = defineSetting({
    name: "theme",
    title: t`Theme`,
    description: t`The colors of every app.`,
    schema: ThemeDefinition.nullable(),
    default: null,
    ...LOOK,
});

/** The accent a person selects over their theme's. */
export const accent = defineSetting({
    name: "accent",
    title: t`Accent color`,
    description: t`Use the theme's accent, or another accent color.`,
    schema: Seed.nullable(),
    default: DEFAULT_DISPLAY.accent,
    ...LOOK,
});

/** The interface typeface a person reads in over their theme's. */
export const typeface = defineSetting({
    name: "typeface",
    title: t`Font`,
    description: t`Use the theme's font, or another font.`,
    schema: Typeface.nullable(),
    default: DEFAULT_DISPLAY.typeface,
    ...LOOK,
});

/** The settings a person's display comes from. */
export const DISPLAY_SETTINGS: readonly DeclarationReference[] = [
    appearance,
    accent,
    contrast,
    density,
    motion,
    textSize,
    theme,
    typeface,
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
        typeface: typeface.resolve(selection, values, chain, device).value,
    };
}
