import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";

/** An effect's background, each effect giving its default. */
export const COLOR_BACK = field
    .color()
    .labeled(t.context("label")`Background`)
    .described(t`The color behind the effect.`);

/** An effect's foreground, each effect giving its default. */
export const COLOR_FRONT = field
    .color()
    .labeled(t.context("label")`Foreground`)
    .described(t`The color the effect draws in.`);

/** The size of an effect's dots, each effect giving its default. */
export const DOT_SIZE = field
    .number(schema.number().min(0))
    .labeled(t.context("label")`Size`)
    .described(t`The size of each dot, in pixels.`);

/** An effect's highlight, each effect giving its default. */
export const COLOR_HIGHLIGHT = field
    .color()
    .labeled(t.context("label")`Highlight`)
    .described(t`The color of the highlights.`);

/** An effect's edge grain, each effect giving its default. */
export const GRAIN_MIXER = field
    .number(schema.number().min(0).max(1))
    .labeled(t.context("label")`Edge grain`)
    .described(t`The grain that roughens the edges of shapes.`);

/** An effect's grain overlay, each effect giving its default. */
export const GRAIN_OVERLAY = field
    .number(schema.number().min(0).max(1))
    .labeled(t.context("label")`Grain overlay`)
    .described(t`The black and white grain over the result.`);

/** An effect's softness, each effect giving its default. */
export const SOFTNESS = field
    .number(schema.number().min(0).max(1))
    .labeled(t.context("label")`Softness`)
    .described(t`How smoothly the colors blend, from a hard edge to a smooth gradient.`);

/** An effect's grain size, each effect giving its default. */
export const GRAIN_SIZE = field
    .number(schema.number().min(0).max(1))
    .labeled(t.context("label")`Grain size`)
    .described(t`The size of the grain.`);

/** An effect's original colors, each effect giving its default. */
export const ORIGINAL_COLORS = field
    .boolean()
    .labeled(t.context("label")`Original colors`)
    .described(t`Draws with the image's own colors in place of the foreground.`);

/** An effect's inverted, each effect giving its default. */
export const INVERTED = field
    .boolean()
    .labeled(t.context("label")`Inverted`)
    .described(t`Inverts the image's lightness.`);

/** An effect's proportion, each effect giving its default. */
export const PROPORTION = field
    .number(schema.number().min(0).max(1))
    .labeled(t.context("label")`Proportion`)
    .described(t`Where the colors meet, an even split in the middle.`);

/** An effect's mixing, each effect giving its default. */
export const MIXING = field
    .number(schema.number().min(0).max(1))
    .labeled(t.context("label")`Mixing`)
    .described(t`How the colors blend, from hard stripes to a smooth gradient.`);

/** The space between an effect and each edge of its canvas, none by default. */
export const MARGIN_PROPERTIES = {
    marginLeft: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Left margin`)
        .described(t`The space between the left edge and the effect.`)
        .default(0),
    marginRight: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Right margin`)
        .described(t`The space between the right edge and the effect.`)
        .default(0),
    marginTop: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Top margin`)
        .described(t`The space between the top edge and the effect.`)
        .default(0),
    marginBottom: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Bottom margin`)
        .described(t`The space between the bottom edge and the effect.`)
        .default(0),
};
