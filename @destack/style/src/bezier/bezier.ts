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

/** The steps Newton's method takes to solve a curve's x, more than CSS's precision needs. */
const NEWTON_STEPS = 8;

/** The slope below which Newton's method stops, as a flat curve's x barely moves. */
const NEWTON_FLAT = 1e-6;

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

    /** Read how far a curve has eased at a share of its time, solving its x for its y. */
    at(curve: CubicBezier, progress: number): number {
        // find the curve's parameter whose x is the progress by Newton's steps, as browsers do
        const [x1, y1, x2, y2] = curve;
        let parameter = progress;
        for (let step = 0; step < NEWTON_STEPS; step += 1) {
            const gradient = slopeAt(parameter, x1, x2);
            if (Math.abs(gradient) < NEWTON_FLAT) {
                break;
            }
            const next = parameter - (pointAt(parameter, x1, x2) - progress) / gradient;
            parameter = Math.min(Math.max(next, 0), 1);
        }

        return pointAt(parameter, y1, y2);
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

/** Read one coordinate of a curve from 0 to 1 at a parameter, between its two control points. */
function pointAt(parameter: number, first: number, second: number): number {
    const rest = 1 - parameter;

    return (
        3 * rest * rest * parameter * first +
        3 * rest * parameter * parameter * second +
        parameter ** 3
    );
}

/** Read the slope of one coordinate of a curve at a parameter. */
function slopeAt(parameter: number, first: number, second: number): number {
    const rest = 1 - parameter;

    return (
        3 * rest * rest * first +
        6 * rest * parameter * (second - first) +
        3 * parameter * parameter * (1 - second)
    );
}
