import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import type { TimingFunction } from "@destack/style";
import { createSignal } from "@destack/view";
import { TimingField } from "./timing-field.tsx";

/** A spring that bounces a little, its duration and bounce set apart from a curve. */
export const timingFieldSpring = defineExample({
    of: TimingField,
    name: "spring",
    description: t`A spring that bounces a little, its duration and bounce set apart from a curve.`,
    render: (properties) => {
        const [timing, setTiming] = createSignal<TimingFunction>({
            kind: "spring",
            duration: { milliseconds: 400 },
            bounce: 0.25,
        });

        return (
            <TimingField
                {...properties}
                label="Easing"
                value={timing()}
                onValueChange={setTiming}
            />
        );
    },
});

/** CSS's ease out as a curve, edited on its plot. */
export const timingFieldCurve = defineExample({
    of: TimingField,
    name: "curve",
    description: t`CSS's ease out as a curve, edited on its plot.`,
    render: (properties) => {
        const [timing, setTiming] = createSignal<TimingFunction>([0, 0, 0.58, 1]);

        return (
            <TimingField
                {...properties}
                label="Easing"
                value={timing()}
                onValueChange={setTiming}
            />
        );
    },
});

/** Four steps jumping at their ends, as a ticking clock moves. */
export const timingFieldSteps = defineExample({
    of: TimingField,
    name: "steps",
    description: t`Four steps jumping at their ends, as a ticking clock moves.`,
    render: (properties) => {
        const [timing, setTiming] = createSignal<TimingFunction>({
            kind: "steps",
            count: 4,
            position: "jump-end",
        });

        return (
            <TimingField
                {...properties}
                label="Easing"
                value={timing()}
                onValueChange={setTiming}
            />
        );
    },
});
