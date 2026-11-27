import { t } from "@destack/locale";
import { Duration } from "@destack/schema";
import { CubicBezier, EASINGS, Reference, Transition } from "@destack/style";
import type { JSX } from "@destack/view";
import { jsonType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The types of a transition's parts. */
const TYPES = {
    cubicBezier: jsonType(CubicBezier.bindable),
    duration: jsonType(Reference.or(Duration, "duration")),
};

/** The transition an input starts at, a quick ease out. */
const START_TRANSITION: Transition = {
    duration: { milliseconds: 200 },
    delay: {},
    timingFunction: EASINGS["ease-out"].curve,
};

/** Render the input of a transition: how long it takes and waits, and how it eases. */
export function TransitionInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the transition, a quick ease out where none is given
    const transition = (): Transition =>
        Transition.safeParse(properties.value).data ?? START_TRANSITION;
    const change = (next: Partial<Transition>): void =>
        properties.onValueChange({ ...transition(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.row}>
            <ValuePart
                type={TYPES.duration}
                schema={Transition.shape.duration}
                label={t.context("label")`Duration`}
                value={transition().duration}
                disabled={properties.disabled}
                onValueChange={(duration) => change({ duration })}
            />
            <ValuePart
                type={TYPES.duration}
                schema={Transition.shape.delay}
                label={t.context("label")`Delay`}
                value={transition().delay}
                disabled={properties.disabled}
                onValueChange={(delay) => change({ delay })}
            />
            <ValuePart
                type={TYPES.cubicBezier}
                schema={CubicBezier.bindable}
                label={t.context("label")`Easing`}
                value={transition().timingFunction}
                disabled={properties.disabled}
                onValueChange={(timingFunction) => change({ timingFunction })}
            />
        </ValueGroup>
    );
}
