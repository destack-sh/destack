import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { type Bindable, Reference } from "../reference/index.ts";

/** The x of a control point, from 0 to 1, as CSS bounds it. */
const CONTROL_X = schema.number().min(0).max(1);

/** An easing as CSS's `cubic-bezier()` writes it, with its four numbers. */
const CUBIC_BEZIER =
    /^cubic-bezier\(\s*([\d.e+-]+)\s*,\s*([\d.e+-]+)\s*,\s*([\d.e+-]+)\s*,\s*([\d.e+-]+)\s*\)$/u;

/** An easing CSS names with a keyword. */
export const Easing = schema.enum(["linear", "ease", "ease-in", "ease-out", "ease-in-out"]);
/** An easing CSS names with a keyword. */
export type Easing = schema.Infer<typeof Easing>;

/** How a change eases, as the design token format's cubic Bézier: two control points' x and y. */
const cubicBezierJson = schema
    .tuple([CONTROL_X, schema.number(), CONTROL_X, schema.number()])
    .meta({ id: "style.cubicBezier", title: "Easing" });
/** How a change eases. */
export type CubicBezier = schema.Infer<typeof cubicBezierJson>;

/** The easings CSS names, as their curves, titled. */
export const EASINGS = {
    linear: { curve: [0, 0, 1, 1], title: t.context("option")`Linear` },
    ease: { curve: [0.25, 0.1, 0.25, 1], title: t.context("option")`Ease` },
    "ease-in": { curve: [0.42, 0, 1, 1], title: t.context("option")`Ease in` },
    "ease-out": { curve: [0, 0, 0.58, 1], title: t.context("option")`Ease out` },
    "ease-in-out": { curve: [0.42, 0, 0.58, 1], title: t.context("option")`Ease in and out` },
} satisfies Record<Easing, { readonly curve: CubicBezier; readonly title: Message }>;

/** How a change eases, as the design token format's cubic Bézier: two control points' x and y. */
export const CubicBezier = Object.assign(cubicBezierJson, {
    /** Find the easing CSS names a curve with, if any. */
    easing(curve: CubicBezier): Easing | undefined {
        return Easing.options.find((easing) =>
            EASINGS[easing].curve.every((point, index) => point === curve[index]),
        );
    },

    /** Read an easing as CSS writes one, a keyword or `cubic-bezier()`, none for other text. */
    read(text: string): CubicBezier | undefined {
        // take a keyword's curve, or the four numbers of a function
        const easing = Easing.safeParse(text.trim()).data;
        const numbers = CUBIC_BEZIER.exec(text.trim())?.slice(1).map(Number);

        return easing === undefined
            ? cubicBezierJson.safeParse(numbers).data
            : EASINGS[easing].curve;
    },

    /** A curve, or a reference to a cubic Bézier token in its place. */
    bindable: Reference.or(cubicBezierJson, "cubicBezier"),

    /** Write a curve as CSS's easing function, by its keyword where CSS names it. */
    css(curve: Bindable<CubicBezier>): string {
        return Reference.write(
            curve,
            (written) => CubicBezier.easing(written) ?? `cubic-bezier(${written.join(", ")})`,
        );
    },
});
