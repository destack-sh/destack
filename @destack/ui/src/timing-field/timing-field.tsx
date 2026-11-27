import bounceIcon from "@destack/icon/phosphor/wave-sine";
import durationIcon from "@destack/icon/phosphor/timer";
import stepsIcon from "@destack/icon/phosphor/stairs";
import { type Message, t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import {
    CubicBezier,
    Reference,
    type Spring,
    StepPosition,
    type Steps,
    TimingFunction,
} from "@destack/style";
import * as style from "@destack/style";
import { color, radius, size } from "@destack/theme/tokens.stylex";
import { type JSX, Show, useLocale } from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { CurveField } from "../curve-field/index.ts";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { NumberPart } from "../value-field/dimension.tsx";
import { choiceType, jsonType, ValuePart, ValueSet } from "../value-field/group.tsx";
import { TimePart } from "../value-field/transition.tsx";

/** The points a preview draws a timing function with, smooth at the size it shows. */
const PREVIEW_POINTS = 48;

/** Where a preview draws 0 and how tall the rise to 1, leaving room to overshoot. */
const PREVIEW_RISE = { start: 75, height: 50 } as const;

/** The kinds of timing function: a curve, steps or a spring. */
const TimingKind = schema.enum(["curve", "steps", "spring"]);
/** A kind of timing function. */
type TimingKind = schema.Infer<typeof TimingKind>;

/** The kinds of timing function, titled. */
const TIMING_KINDS = {
    curve: { title: t.context("option")`Curve` },
    steps: { title: t.context("option")`Steps` },
    spring: { title: t.context("option")`Spring` },
} satisfies Record<TimingKind, { readonly title: Message }>;

/** Where steps jump, titled as people read them. */
const STEP_POSITIONS = {
    "jump-start": { title: t.context("option")`At the start` },
    "jump-end": { title: t.context("option")`At the end` },
    "jump-none": { title: t.context("option")`Neither end` },
    "jump-both": { title: t.context("option")`Both ends` },
} satisfies Record<StepPosition, { readonly title: Message }>;

/** The timing each kind starts at, as a person switches to it. */
const STARTS = {
    curve: TimingFunction.initial,
    steps: { kind: "steps", count: 4, position: "jump-end" },
    spring: { kind: "spring", duration: { milliseconds: 400 }, bounce: 0.2 },
} satisfies Record<TimingKind, TimingFunction>;

/** The types of a timing function's parts. */
const TYPES = {
    kind: choiceType(TimingKind.options, TIMING_KINDS),
    curve: jsonType(CubicBezier.bindable),
    position: choiceType(StepPosition.options, STEP_POSITIONS),
};

/** The styles of a timing function's input. */
const styles = style.create({
    preview: {
        width: "100%",
        height: size[3],
        borderRadius: radius[2],
        backgroundColor: color.muted,
        overflow: "visible",
    },
    line: {
        fill: "none",
        stroke: color.foreground,
        strokeWidth: 2,
    },
    trigger: {
        display: "block",
        width: "100%",
        padding: 0,
        borderWidth: 0,
        borderRadius: radius[2],
        backgroundColor: "transparent",
        cursor: "pointer",
    },
    editor: {
        width: "14rem",
    },
});

/** The properties people change in a timing field's controls and inspector. */
const TIMING_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses changes where the timing cannot change now.`)
        .default(false),
};

/** The properties of a timing field besides its declared ones: its timing. */
export interface TimingFieldAttributes {
    /** What the timing is called, shown as its legend. */
    readonly label?: string | undefined;
    /** What the timing is called where no legend shows. */
    readonly "aria-label"?: string | undefined;
    /** The timing, CSS's ease where none is given. */
    readonly value: TimingFunction | undefined;
    /** Handle a person changing the timing. */
    readonly onValueChange: (value: TimingFunction) => void;
}

/** Render a timing function's field: its kind, its preview, then a curve, steps or a spring. */
export const TimingField = defineComponent({
    name: "timingField",
    properties: TIMING_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<TimingFieldAttributes, typeof TIMING_FIELD_PROPERTIES>,
    ): JSX.Element {
        return <TimingFieldBody {...properties} />;
    },
});

/** The properties of a timing field, its timing included. */
export type TimingFieldProperties = ComponentProperties<typeof TimingField>;

/** Render the parts of a timing field. */
function TimingFieldBody(
    properties: RenderProperties<TimingFieldAttributes, typeof TIMING_FIELD_PROPERTIES>,
): JSX.Element {
    // read the timing, CSS's ease where none is given, and its kind
    const locale = useLocale();
    const timing = (): TimingFunction => properties.value ?? TimingFunction.initial;
    const kind = (): TimingKind => kindOf(timing());
    const curve = () => {
        const current = timing();

        return Reference.is(current) || Array.isArray(current) ? current : undefined;
    };
    const own = (): CubicBezier | undefined => CubicBezier.safeParse(curve()).data;
    const steps = (): Steps | undefined => {
        const current = timing();

        return !Reference.is(current) && !Array.isArray(current) && current.kind === "steps"
            ? current
            : undefined;
    };
    const spring = (): Spring | undefined => {
        const current = timing();

        return !Reference.is(current) && !Array.isArray(current) && current.kind === "spring"
            ? current
            : undefined;
    };

    return (
        <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
            {/* Kind, then how it eases over its time, an own curve edited on a plot */}
            <FieldGroup columns={1}>
                <ValuePart
                    type={TYPES.kind}
                    schema={TimingKind}
                    label={t.context("label")`Kind`}
                    value={kind()}
                    disabled={properties.disabled}
                    onValueChange={(next) => properties.onValueChange(STARTS[next])}
                />
                <Show when={own()} fallback={<TimingPreview timing={timing()} />}>
                    {(current) => (
                        <Popover>
                            <PopoverTrigger
                                render={(attributes) => (
                                    <button
                                        {...attributes}
                                        type="button"
                                        aria-label={locale.render(t.context("action")`Edit curve`)}
                                        disabled={properties.disabled}
                                        {...style.attrs(styles.trigger)}
                                    >
                                        <TimingPreview timing={current()} />
                                    </button>
                                )}
                            />
                            <PopoverContent
                                aria-label={locale.render(t.context("action")`Edit curve`)}
                                xstyle={styles.editor}
                            >
                                <CurveField
                                    aria-label={locale.render(t.context("label")`Easing`)}
                                    disabled={properties.disabled}
                                    value={current()}
                                    onValueChange={(next) => properties.onValueChange(next)}
                                />
                            </PopoverContent>
                        </Popover>
                    )}
                </Show>
            </FieldGroup>

            {/* A curve, its keyword or its token */}
            <Show when={curve()}>
                {(current) => (
                    <ValuePart
                        type={TYPES.curve}
                        schema={CubicBezier.bindable}
                        label={t.context("label")`Easing`}
                        value={current()}
                        disabled={properties.disabled}
                        onValueChange={(next) => properties.onValueChange(next)}
                    />
                )}
            </Show>

            {/* Steps: how many, and where they jump */}
            <Show when={steps()}>
                {(current) => (
                    <FieldGroup columns={2}>
                        <NumberPart
                            label={locale.render(t.context("label")`Steps`)}
                            start={<Icon icon={stepsIcon} />}
                            min={1}
                            step={1}
                            disabled={properties.disabled}
                            value={current().count}
                            onValueChange={(count) =>
                                properties.onValueChange({ ...current(), count })
                            }
                        />
                        <ValuePart
                            type={TYPES.position}
                            schema={StepPosition}
                            label={t.context("label")`Jump`}
                            value={current().position}
                            disabled={properties.disabled}
                            onValueChange={(position) =>
                                properties.onValueChange({ ...current(), position })
                            }
                        />
                    </FieldGroup>
                )}
            </Show>

            {/* A spring: how long it takes to look settled, and how much it bounces */}
            <Show when={spring()}>
                {(current) => (
                    <FieldGroup columns={2}>
                        <TimePart
                            label={locale.render(t.context("label")`Duration`)}
                            icon={durationIcon}
                            disabled={properties.disabled}
                            value={current().duration}
                            onValueChange={(duration) => {
                                if (!Reference.is(duration)) {
                                    properties.onValueChange({ ...current(), duration });
                                }
                            }}
                        />
                        <NumberPart
                            label={locale.render(t.context("label")`Bounce`)}
                            start={<Icon icon={bounceIcon} />}
                            min={0}
                            max={1}
                            step={0.05}
                            format={{ style: "percent" }}
                            disabled={properties.disabled}
                            value={current().bounce}
                            onValueChange={(bounce) =>
                                properties.onValueChange({ ...current(), bounce })
                            }
                        />
                    </FieldGroup>
                )}
            </Show>
        </ValueSet>
    );
}

/** Render how a timing function eases over its time, a curve rising from 0 to 1. */
function TimingPreview(properties: {
    /** The timing function, none drawn for a token's curve. */
    readonly timing: TimingFunction;
}): JSX.Element {
    // sample the timing over its time, the plot's top at 1 with room to overshoot
    const points = (): string => {
        const { timing } = properties;
        if (Reference.is(timing)) {
            return "";
        }

        return Array.from({ length: PREVIEW_POINTS + 1 }, (_, index) => {
            const progress = index / PREVIEW_POINTS;
            const eased = TimingFunction.at(timing, progress);

            return `${String(progress * 100)},${String(PREVIEW_RISE.start - eased * PREVIEW_RISE.height)}`;
        }).join(" ");
    };

    return (
        <svg
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            aria-hidden="true"
            {...style.attrs(styles.preview)}
        >
            <polyline
                points={points()}
                vector-effect="non-scaling-stroke"
                {...style.attrs(styles.line)}
            />
        </svg>
    );
}

/** Read the kind of a timing function: a curve or its token, steps, or a spring. */
function kindOf(timing: TimingFunction): TimingKind {
    return Reference.is(timing) || Array.isArray(timing) ? "curve" : timing.kind;
}
