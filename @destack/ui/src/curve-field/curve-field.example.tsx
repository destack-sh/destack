import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { CurveField } from "./curve-field.tsx";

/** CSS's ease out, its control points to drag or move by arrows. */
export const curveFieldDefault = defineExample({
    of: CurveField,
    name: "default",
    description: t`CSS's ease out, its control points to drag or move by arrows.`,
    render: (properties) => (
        <CurveField {...properties} aria-label="Easing" defaultValue={[0, 0, 0.58, 1]} />
    ),
});

/** A curve that overshoots its end and comes back, as a playful ease does. */
export const curveFieldOvershoot = defineExample({
    of: CurveField,
    name: "overshoot",
    description: t`A curve that overshoots its end and comes back, as a playful ease does.`,
    render: (properties) => (
        <CurveField {...properties} aria-label="Easing" defaultValue={[0.34, 1.56, 0.64, 1]} />
    ),
});
