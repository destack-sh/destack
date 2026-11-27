import { Toggle } from "../toggle/index.ts";
import delayIcon from "@destack/icon/phosphor/hourglass";
import infinityIcon from "@destack/icon/phosphor/infinity";
import offsetIcon from "@destack/icon/phosphor/percent";
import repeatIcon from "@destack/icon/phosphor/repeat";
import durationIcon from "@destack/icon/phosphor/timer";
import { type Message, t } from "@destack/locale";
import { Duration, schema } from "@destack/schema";
import {
    MOTION_PROPERTIES,
    MotionAnimation,
    MotionProperty,
    MotionTrack,
    type RangeOffset,
    Reference,
    Scroller,
    type Timeline,
    TimelineAxis,
    ViewRange,
} from "@destack/style";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { For, type JSX, Show, useLocale } from "@destack/view";
import { Field, FieldLabel, FieldSeparator } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { NumberPart } from "../value-field/dimension.tsx";
import { choiceType, ValuePart, ValueSet } from "../value-field/group.tsx";
import { TimePart } from "../value-field/transition.tsx";
import { PropertyMenu } from "./frame.tsx";
import { TrackField } from "./track.tsx";

/** The timelines a person picks from, titled, each starting as it is first picked. */
export const TIMELINES = {
    time: { title: t.context("option")`Time`, start: { kind: "time" } },
    scroll: {
        title: t.context("option")`Scroll`,
        start: { kind: "scroll", axis: "block", scroller: "nearest" },
    },
    view: {
        title: t.context("option")`View`,
        start: {
            kind: "view",
            axis: "block",
            start: { range: "entry", percent: 0 },
            end: { range: "cover", percent: 50 },
        },
    },
} satisfies Record<Timeline["kind"], { readonly title: Message; readonly start: Timeline }>;

/** The kinds of timeline. */
const TimelineKind = schema.enum(["time", "scroll", "view"]);

/** Which way each iteration of an animation plays. */
const Direction = MotionAnimation.shape.direction;

/** Whether an animation holds its first frame before and its last after. */
const Fill = MotionAnimation.shape.fill;

/** The types of an animation's parts, titled as CSS's values. */
const TYPES = {
    timeline: choiceType(TimelineKind.options, TIMELINES),
    direction: choiceType(Direction.options, {
        normal: { title: t.context("option")`Normal` },
        reverse: { title: t.context("option")`Reverse` },
        alternate: { title: t.context("option")`Alternate` },
        "alternate-reverse": { title: t.context("option")`Alternate reverse` },
    }),
    fill: choiceType(Fill.options, {
        none: { title: t.context("option")`None` },
        forwards: { title: t.context("option")`Forwards` },
        backwards: { title: t.context("option")`Backwards` },
        both: { title: t.context("option")`Both` },
    }),
    axis: choiceType(TimelineAxis.options, {
        block: { title: t.context("option")`Vertical` },
        inline: { title: t.context("option")`Horizontal` },
    }),
    scroller: choiceType(Scroller.options, {
        nearest: { title: t.context("option")`Nearest` },
        root: { title: t.context("option")`Root` },
        self: { title: t.context("option")`Self` },
    }),
    range: choiceType(ViewRange.options, {
        cover: { title: t.context("option")`Cover` },
        contain: { title: t.context("option")`Contain` },
        entry: { title: t.context("option")`Entry` },
        exit: { title: t.context("option")`Exit` },
        "entry-crossing": { title: t.context("option")`Entry crossing` },
        "exit-crossing": { title: t.context("option")`Exit crossing` },
    }),
};

/** The styles of an animation's field. */
const styles = style.create({
    // a number beside the toggle that repeats it forever
    iterations: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
    // an edge of the view and the share of it a range starts or ends at
    range: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 3fr) minmax(0, 2fr)",
        gap: space[1],
    },
});

/** Render an animation's field: its time, its timeline and its tracks. */
export function AnimationField(properties: {
    /** What the animation is called where no legend shows. */
    readonly "aria-label": string | undefined;
    /** The animation. */
    readonly value: MotionAnimation;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the animation changing. */
    readonly onValueChange: (value: MotionAnimation) => void;
}): JSX.Element {
    // change the animation, and read its timeline by kind
    const locale = useLocale();
    const animation = (): MotionAnimation => properties.value;
    const change = (next: Partial<MotionAnimation>): void =>
        properties.onValueChange({ ...animation(), ...next });
    const scroll = () => {
        const timeline = animation().timeline;

        return timeline.kind === "scroll" ? timeline : undefined;
    };
    const view = () => {
        const timeline = animation().timeline;

        return timeline.kind === "view" ? timeline : undefined;
    };

    // read a finite number of iterations, none where it repeats forever
    const iterations = (): number | undefined => {
        const count = animation().iterations;

        return count === "infinite" ? undefined : count;
    };

    // offer the properties no track animates yet
    const free = (): MotionProperty[] =>
        MotionProperty.options.filter(
            (property) => !animation().tracks.some((track) => track.property === property),
        );

    return (
        <ValueSet aria-label={properties["aria-label"]}>
            {/* Its time: how long, after what wait, how often and which way, as property rows */}
            <Field orientation="horizontal">
                <FieldLabel>{locale.render(t.context("label")`Duration`)}</FieldLabel>
                <TimePart
                    label={locale.render(t.context("label")`Duration`)}
                    icon={durationIcon}
                    disabled={properties.disabled}
                    value={animation().duration}
                    onValueChange={(duration) => change({ duration })}
                />
            </Field>
            <Field orientation="horizontal">
                <FieldLabel>{locale.render(t.context("label")`Delay`)}</FieldLabel>
                <TimePart
                    label={locale.render(t.context("label")`Delay`)}
                    icon={delayIcon}
                    disabled={properties.disabled}
                    value={animation().delay}
                    onValueChange={(delay) => change({ delay })}
                />
            </Field>
            <Field orientation="horizontal">
                <FieldLabel>{locale.render(t.context("label")`Iterations`)}</FieldLabel>
                <div {...style.attrs(styles.iterations)}>
                    <NumberPart
                        label={locale.render(t.context("label")`Iterations`)}
                        start={<Icon icon={repeatIcon} />}
                        min={1}
                        step={1}
                        disabled={
                            properties.disabled === true || animation().iterations === "infinite"
                        }
                        value={iterations() ?? 1}
                        onValueChange={(count) => change({ iterations: count })}
                    />
                    <Toggle
                        size="sm"
                        isIconOnly
                        aria-label={locale.render(t.context("action")`Repeat forever`)}
                        pressed={animation().iterations === "infinite"}
                        disabled={properties.disabled}
                        onPressedChange={(isForever) =>
                            change({ iterations: isForever ? "infinite" : 1 })
                        }
                    >
                        <Icon icon={infinityIcon} />
                    </Toggle>
                </div>
            </Field>
            <ValuePart
                type={TYPES.direction}
                schema={Direction}
                label={t.context("label")`Direction`}
                isLabelShown
                orientation="horizontal"
                value={animation().direction}
                disabled={properties.disabled}
                onValueChange={(direction) => change({ direction })}
            />
            <ValuePart
                type={TYPES.fill}
                schema={Fill}
                label={t.context("label")`Fill`}
                isLabelShown
                orientation="horizontal"
                value={animation().fill}
                disabled={properties.disabled}
                onValueChange={(fill) => change({ fill })}
            />

            <FieldSeparator />

            {/* What its progress follows: time, a scroll, or the element's view */}
            <ValuePart
                type={TYPES.timeline}
                schema={TimelineKind}
                label={t.context("label")`Timeline`}
                isLabelShown
                orientation="horizontal"
                value={animation().timeline.kind}
                disabled={properties.disabled}
                onValueChange={(kind) => change({ timeline: TIMELINES[kind].start })}
            />
            <Show when={scroll()}>
                {(timeline) => (
                    <>
                        <ValuePart
                            type={TYPES.axis}
                            schema={TimelineAxis}
                            label={t.context("label")`Axis`}
                            isLabelShown
                            orientation="horizontal"
                            value={timeline().axis}
                            disabled={properties.disabled}
                            onValueChange={(axis) => change({ timeline: { ...timeline(), axis } })}
                        />
                        <ValuePart
                            type={TYPES.scroller}
                            schema={Scroller}
                            label={t.context("label")`Scroller`}
                            isLabelShown
                            orientation="horizontal"
                            value={timeline().scroller}
                            disabled={properties.disabled}
                            onValueChange={(scroller) =>
                                change({ timeline: { ...timeline(), scroller } })
                            }
                        />
                    </>
                )}
            </Show>
            <Show when={view()}>
                {(timeline) => (
                    <>
                        <RangeOffsetPart
                            label={t.context("label")`Range start`}
                            offsetLabel={t.context("label")`Range start offset`}
                            value={timeline().start}
                            disabled={properties.disabled}
                            onValueChange={(start) =>
                                change({ timeline: { ...timeline(), start } })
                            }
                        />
                        <RangeOffsetPart
                            label={t.context("label")`Range end`}
                            offsetLabel={t.context("label")`Range end offset`}
                            value={timeline().end}
                            disabled={properties.disabled}
                            onValueChange={(end) => change({ timeline: { ...timeline(), end } })}
                        />
                    </>
                )}
            </Show>

            <FieldSeparator />

            {/* Its tracks, one per property, a menu adding the others */}
            <ValueSet
                label={locale.render(t.context("label")`Tracks`)}
                actions={
                    <PropertyMenu
                        label={locale.render(t.context("action")`Add track`)}
                        properties={free()}
                        disabled={properties.disabled}
                        onSelect={(property) =>
                            change({ tracks: [...animation().tracks, MotionTrack.start(property)] })
                        }
                    />
                }
            >
                <For each={animation().tracks}>
                    {(track, index) => (
                        <TrackField
                            value={track}
                            disabled={properties.disabled}
                            onValueChange={(next) =>
                                change({ tracks: animation().tracks.with(index(), next) })
                            }
                            onRemove={() =>
                                change({ tracks: animation().tracks.toSpliced(index(), 1) })
                            }
                        />
                    )}
                </For>
            </ValueSet>
        </ValueSet>
    );
}

/** Render where a view timeline's range starts or ends: a range and a share of it, in one row. */
function RangeOffsetPart(properties: {
    /** What the edge is called. */
    readonly label: Message;
    /** What its share is called. */
    readonly offsetLabel: Message;
    /** The edge. */
    readonly value: RangeOffset;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the edge changing. */
    readonly onValueChange: (value: RangeOffset) => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Field orientation="horizontal">
            <FieldLabel>{locale.render(properties.label)}</FieldLabel>
            <ValueSet xstyle={styles.range}>
                <ValuePart
                    type={TYPES.range}
                    schema={ViewRange}
                    label={properties.label}
                    value={properties.value.range}
                    disabled={properties.disabled}
                    onValueChange={(range) =>
                        properties.onValueChange({ ...properties.value, range })
                    }
                />
                <NumberPart
                    label={locale.render(properties.offsetLabel)}
                    start={<Icon icon={offsetIcon} />}
                    disabled={properties.disabled}
                    value={properties.value.percent}
                    onValueChange={(percent) =>
                        properties.onValueChange({ ...properties.value, percent })
                    }
                />
            </ValueSet>
        </Field>
    );
}

/** Render a summary of an animation: its properties, then its duration or its timeline. */
export function AnimationSummary(properties: {
    /** The animation. */
    readonly value: MotionAnimation;
}): JSX.Element {
    // name its properties, then its duration or token on time, else its timeline
    const locale = useLocale();
    const titles = (): string =>
        properties.value.tracks
            .map((track) => locale.render(MOTION_PROPERTIES[track.property].title))
            .join(", ");
    const timing = (): string => {
        const { duration, timeline } = properties.value;

        return timeline.kind !== "time"
            ? locale.render(TIMELINES[timeline.kind].title)
            : Reference.is(duration)
              ? Reference.path(duration).join(".")
              : locale.number(Duration.seconds(duration), {
                    style: "unit",
                    unit: "second",
                    unitDisplay: "narrow",
                });
    };

    return <>{[titles(), timing()].filter((part) => part !== "").join(" · ")}</>;
}
