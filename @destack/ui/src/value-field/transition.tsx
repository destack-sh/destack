import durationIcon from "@destack/icon/phosphor/timer";
import delayIcon from "@destack/icon/phosphor/hourglass";
import { t } from "@destack/locale";
import { Duration, present } from "@destack/schema";
import { type Bindable, EASINGS, TimingFunction, Transition } from "@destack/style";
import type { IconBodies } from "@destack/icon";
import { type JSX, Show, useLocale } from "@destack/view";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { InputGroup, InputGroupAddon } from "../input-group/index.ts";
import { NumberField, NumberFieldInput, NumberFieldPrefix } from "../number-field/index.ts";
import { TokenBinding } from "./bindable.tsx";
import { jsonType, ValueNone, ValuePart, ValueSet } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** The types of a transition's parts. */
const TYPES = {
    timingFunction: jsonType(TimingFunction),
};

/** The milliseconds of a second, as a person types seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The transition a person adds, a quick ease out. */
const NEW_TRANSITION: Transition = {
    duration: { milliseconds: 200 },
    delay: {},
    timingFunction: EASINGS["ease-out"].curve,
};

/** Render the input of a transition: how long it takes and waits, and how it eases. */
export function TransitionInput(properties: NamedValueInputProperties): JSX.Element {
    // read the transition, a quick ease out where none is given
    const transition = (): Transition | undefined => readValue(properties.value, Transition);
    const change = (next: Partial<Transition>): void =>
        properties.onValueChange({ ...present(transition(), "the value changed"), ...next });
    const locale = useLocale();

    return (
        <Show
            when={transition()}
            fallback={
                <ValueNone
                    label={properties.label}
                    aria-label={properties["aria-label"]}
                    disabled={properties.disabled}
                    onAdd={() => properties.onValueChange(NEW_TRANSITION)}
                />
            }
        >
            {(current) => (
                <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
                    {/* How long it takes and waits */}
                    <FieldGroup columns={2}>
                        <TimePart
                            label={locale.render(t.context("label")`Duration`)}
                            icon={durationIcon}
                            disabled={properties.disabled}
                            value={current().duration}
                            onValueChange={(duration) => change({ duration })}
                        />
                        <TimePart
                            label={locale.render(t.context("label")`Delay`)}
                            icon={delayIcon}
                            disabled={properties.disabled}
                            value={current().delay}
                            onValueChange={(delay) => change({ delay })}
                        />
                    </FieldGroup>

                    {/* How it eases */}
                    <FieldGroup columns={1}>
                        <ValuePart
                            type={TYPES.timingFunction}
                            schema={TimingFunction}
                            label={t.context("label")`Easing`}
                            value={current().timingFunction}
                            disabled={properties.disabled}
                            onValueChange={(timingFunction) => change({ timingFunction })}
                        />
                    </FieldGroup>
                </ValueSet>
            )}
        </Show>
    );
}

/** Render a time of a transition as one field typed in milliseconds or seconds, such as `200ms`. */
export function TimePart(properties: {
    /** What the time is called. */
    readonly label: string;
    /** The icon at the field's start. */
    readonly icon: IconBodies;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The time, or a reference to a duration token. */
    readonly value: Bindable<Duration>;
    /** Handle the time or its token changing. */
    readonly onValueChange: (value: Bindable<Duration>) => void;
}): JSX.Element {
    // write the time in milliseconds, reading seconds where a person types them
    let typed: string | undefined;
    const give = (value: number): void => {
        const scale = typed === "s" ? MILLISECONDS_PER_SECOND : 1;
        properties.onValueChange(Duration.of(Math.round(value * scale), "milliseconds"));
    };

    return (
        <TokenBinding
            type="duration"
            label={properties.label}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={properties.onValueChange}
            detach={(probe, reference) => probe.duration(reference)}
        >
            {(time, bind) => (
                <NumberField
                    aria-label={properties.label}
                    min={0}
                    units={["ms", "s"]}
                    unit="ms"
                    disabled={properties.disabled}
                    value={Duration.milliseconds(time() ?? {})}
                    onUnitChange={(unit) => (typed = unit)}
                    onValueChange={(next) => next !== undefined && give(next)}
                    onValueCommitted={() => (typed = undefined)}
                >
                    <InputGroup>
                        <NumberFieldPrefix label={properties.label}>
                            <Icon icon={properties.icon} />
                        </NumberFieldPrefix>
                        <NumberFieldInput />
                        <InputGroupAddon align="inline-end">{bind}</InputGroupAddon>
                    </InputGroup>
                </NumberField>
            )}
        </TokenBinding>
    );
}
