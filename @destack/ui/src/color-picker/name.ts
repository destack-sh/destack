import type { Color, ColorHueName, ColorName } from "@destack/color";
import { type Localization, type Message, t } from "@destack/locale";

/** What each hue is called. */
export const HUE_NAMES: Readonly<Record<ColorHueName, Message>> = {
    black: t`black`,
    white: t`white`,
    gray: t`gray`,
    pink: t`pink`,
    red: t`red`,
    orange: t`orange`,
    brown: t`brown`,
    yellow: t`yellow`,
    green: t`green`,
    cyan: t`cyan`,
    blue: t`blue`,
    purple: t`purple`,
    magenta: t`magenta`,
};

/** What each lightness is called. */
const LIGHTNESSES: Readonly<Record<NonNullable<ColorName["lightness"]>, Message>> = {
    "very dark": t`very dark`,
    dark: t`dark`,
    light: t`light`,
    "very light": t`very light`,
};

/** What each strength is called. */
const CHROMAS: Readonly<Record<NonNullable<ColorName["chroma"]>, Message>> = {
    grayish: t`grayish`,
    pale: t`pale`,
    vibrant: t`vibrant`,
};

/** Write what people call a color in the reader's language. */
export function nameOf(color: Color, locale: Localization): string {
    // write the hue, one or between two
    const { lightness, chroma, hues } = color.name();
    const [first = "gray", second] = hues;
    const only = locale.render(HUE_NAMES[first]);
    const hue =
        second === undefined ? only : locale.render(t`${only}-${locale.render(HUE_NAMES[second])}`);

    // add how light and how strong it is, as the language orders them
    const adjectives = [
        ...(lightness === undefined ? [] : [locale.render(LIGHTNESSES[lightness])]),
        ...(chroma === undefined ? [] : [locale.render(CHROMAS[chroma])]),
    ];
    const [one, other] = adjectives;
    const name =
        one === undefined
            ? hue
            : other === undefined
              ? locale.render(t`${one} ${hue}`)
              : locale.render(t`${one} ${other} ${hue}`);

    return color.alpha === 1
        ? name
        : locale.render(
              t`${name}, ${locale.number(1 - color.alpha, { style: "percent" })} transparent`,
          );
}
