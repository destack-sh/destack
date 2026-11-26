import type { Color, ColorHueName, ColorName } from "@destack/color";
import { type Localization, type Message, t } from "@destack/locale";

/** What each hue is called. */
export const HUE_NAMES: Readonly<Record<ColorHueName, Message>> = {
    black: t.context("option")`black`,
    white: t.context("option")`white`,
    gray: t.context("option")`gray`,
    pink: t.context("option")`pink`,
    red: t.context("option")`red`,
    orange: t.context("option")`orange`,
    brown: t.context("option")`brown`,
    yellow: t.context("option")`yellow`,
    green: t.context("option")`green`,
    cyan: t.context("option")`cyan`,
    blue: t.context("option")`blue`,
    purple: t.context("option")`purple`,
    magenta: t.context("option")`magenta`,
};

/** What each lightness is called. */
const LIGHTNESSES: Readonly<Record<NonNullable<ColorName["lightness"]>, Message>> = {
    "very dark": t.context("option")`very dark`,
    dark: t.context("option")`dark`,
    light: t.context("option")`light`,
    "very light": t.context("option")`very light`,
};

/** What each strength is called. */
const CHROMAS: Readonly<Record<NonNullable<ColorName["chroma"]>, Message>> = {
    grayish: t.context("option")`grayish`,
    pale: t.context("option")`pale`,
    vibrant: t.context("option")`vibrant`,
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
