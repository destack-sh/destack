import { type Message, t } from "@destack/locale";
import { Duration, present, schema } from "@destack/schema";
import { Corners } from "../box/index.ts";
import { Color } from "../color/index.ts";
import { FilterStack } from "../filter/index.ts";
import { Opacity } from "../opacity/index.ts";
import { ShadowStack } from "../shadow/index.ts";
import { Sizing } from "../size/index.ts";
import { Transform } from "../transform/index.ts";
import { Reference } from "../reference/index.ts";
import { Time, TimingFunction, Transition } from "../transition/index.ts";

/** The query motion plays inside, leaving it out for people who ask for less motion. */
const MOTION_QUERY = "@media (prefers-reduced-motion: no-preference)";

/** The selector of the element while it is hidden, which an exit moves it to. */
const HIDDEN = ":is([hidden], [data-closed])";

/** The custom property a parent's stagger delays each child by, as CSS registers it. */
const STAGGER = "--destack-motion-stagger";

/** The registration of a stagger's delay, so each child takes its own rather than its parent's. */
const STAGGER_PROPERTY = `@property ${STAGGER} { syntax: "<time>"; inherits: false; initial-value: 0ms; }`;

/** The children a stagger reaches where `sibling-index()` is missing, a screen of list rows. */
const STAGGERED_CHILDREN = 24;

/** The style values motion animates, as CSS properties a browser interpolates. */
export const MotionProperty = schema.enum([
    "opacity",
    "transform",
    "filters",
    "shadows",
    "corners",
    "background",
    "color",
    "width",
    "height",
]);
/** A style value motion animates. */
export type MotionProperty = schema.Infer<typeof MotionProperty>;

/** The schemas of the values motion animates, by property. */
const VALUES = {
    opacity: Opacity,
    transform: Transform,
    filters: FilterStack,
    shadows: ShadowStack.bindable,
    corners: Corners,
    background: Color.bindable,
    color: Color.bindable,
    width: Sizing,
    height: Sizing,
};

/** The style values motion animates, titled, each with its value's schema and CSS's initial one. */
export const MOTION_PROPERTIES = {
    opacity: {
        title: t.context("option")`Opacity`,
        schema: VALUES.opacity,
        initial: Opacity.initial,
        isComposited: true,
    },
    transform: {
        title: t.context("option")`Transform`,
        schema: VALUES.transform,
        initial: Transform.initial,
        isComposited: true,
    },
    filters: {
        title: t.context("option")`Filters`,
        schema: VALUES.filters,
        initial: [],
        isComposited: true,
    },
    shadows: {
        title: t.context("option")`Shadows`,
        schema: VALUES.shadows,
        initial: [],
        isComposited: false,
    },
    corners: {
        title: t.context("option")`Corners`,
        schema: VALUES.corners,
        initial: Corners.initial,
        isComposited: false,
    },
    background: {
        title: t.context("option")`Background`,
        schema: VALUES.background,
        initial: "#00000000",
        isComposited: false,
    },
    color: {
        title: t.context("option")`Text color`,
        schema: VALUES.color,
        initial: "#000000",
        isComposited: false,
    },
    width: {
        title: t.context("option")`Width`,
        schema: VALUES.width,
        initial: { kind: "auto" },
        isComposited: false,
    },
    height: {
        title: t.context("option")`Height`,
        schema: VALUES.height,
        initial: { kind: "auto" },
        isComposited: false,
    },
} satisfies Record<
    MotionProperty,
    {
        readonly title: Message;
        readonly schema: schema.Schema;
        readonly initial: unknown;
        readonly isComposited: boolean;
    }
>;

/** One style value of a frame: a property and its value. */
export const MotionValue = schema
    .discriminatedUnion("property", [
        schema.object({ property: schema.literal("opacity"), value: VALUES.opacity }),
        schema.object({ property: schema.literal("transform"), value: VALUES.transform }),
        schema.object({ property: schema.literal("filters"), value: VALUES.filters }),
        schema.object({ property: schema.literal("shadows"), value: VALUES.shadows }),
        schema.object({ property: schema.literal("corners"), value: VALUES.corners }),
        schema.object({ property: schema.literal("background"), value: VALUES.background }),
        schema.object({ property: schema.literal("color"), value: VALUES.color }),
        schema.object({ property: schema.literal("width"), value: VALUES.width }),
        schema.object({ property: schema.literal("height"), value: VALUES.height }),
    ])
    .meta({ id: "style.motionValue", title: "Motion value" });
/** One style value of a frame. */
export type MotionValue = schema.Infer<typeof MotionValue>;

/** The states an element moves between, as CSS selects them. */
export const MotionState = schema.enum(["hover", "press", "focus", "open"]);
/** A state an element moves between. */
export type MotionState = schema.Infer<typeof MotionState>;

/** The states an element moves between, titled, with the selector of each. */
export const MOTION_STATES = {
    hover: { title: t.context("option")`Hover`, selector: ":hover" },
    press: { title: t.context("option")`Press`, selector: ":active" },
    focus: { title: t.context("option")`Focus`, selector: ":focus-visible" },
    open: { title: t.context("option")`Open`, selector: "[data-open]" },
} satisfies Record<MotionState, { readonly title: Message; readonly selector: string }>;

/** The keyframes of one property, each at an offset, easing into the next. */
const trackJson = schema.discriminatedUnion("property", [
    track("opacity", VALUES.opacity),
    track("transform", VALUES.transform),
    track("filters", VALUES.filters),
    track("shadows", VALUES.shadows),
    track("corners", VALUES.corners),
    track("background", VALUES.background),
    track("color", VALUES.color),
    track("width", VALUES.width),
    track("height", VALUES.height),
]);
/** The keyframes of one property. */
export type MotionTrack = schema.Infer<typeof trackJson>;

/** A keyframe of a track: where it falls, its value and how it eases into the next. */
export interface Keyframe {
    /** Where it falls in the iteration, from 0 to 1. */
    readonly offset: number;
    /** The value, of the track's property. */
    readonly value: unknown;
    /** How it eases into the next keyframe, unused on the last, as in the Web Animations API. */
    readonly easing: TimingFunction;
}

/** A track and the place of one keyframe in it, after a change that may reorder its keyframes. */
export interface TrackPlace {
    /** The track. */
    readonly track: MotionTrack;
    /** The place of the changed keyframe. */
    readonly index: number;
}

/** The keyframes of one property, and the changes that keep them in order of their offsets. */
export const MotionTrack = Object.assign(trackJson, {
    /** Start a track of a property, from CSS's initial value to itself over its time. */
    start(property: MotionProperty): MotionTrack {
        const value = MOTION_PROPERTIES[property].initial;

        return ordered(property, [
            { offset: 0, value, easing: TimingFunction.initial },
            { offset: 1, value, easing: TimingFunction.initial },
        ]).track;
    },

    /** Change a keyframe, moving it among the others where its offset passes theirs. */
    set(each: MotionTrack, index: number, change: Partial<Keyframe>): TrackPlace {
        const keyframes: readonly Keyframe[] = each.keyframes;
        const changed = { ...present(keyframes[index], "a keyframe of the track"), ...change };

        return ordered(each.property, keyframes.with(index, changed), changed);
    },

    /** Add a keyframe halfway to the next one, or to the previous one from the last, alike. */
    split(each: MotionTrack, index: number): TrackPlace {
        // read the keyframe and its neighbour, the previous one from the last keyframe
        const keyframes: readonly Keyframe[] = each.keyframes;
        const current = present(keyframes[index], "a keyframe of the track");
        const neighbour = keyframes[index + 1] ?? keyframes[index - 1] ?? current;
        const added = { ...current, offset: (current.offset + neighbour.offset) / 2 };

        return ordered(each.property, [...keyframes, added], added);
    },

    /** Remove a keyframe, refusing the only one a track has. */
    remove(each: MotionTrack, index: number): MotionTrack {
        // refuse to leave the track without keyframes
        const keyframes: readonly Keyframe[] = each.keyframes;
        if (keyframes.length === 1) {
            throw new RangeError("a track keeps at least one keyframe");
        }

        return ordered(each.property, keyframes.toSpliced(index, 1)).track;
    },
});

/** The edges of a view timeline's range, as CSS's `animation-range` names them. */
export const ViewRange = schema.enum([
    "cover",
    "contain",
    "entry",
    "exit",
    "entry-crossing",
    "exit-crossing",
]);
/** An edge of a view timeline's range. */
export type ViewRange = schema.Infer<typeof ViewRange>;

/** Where a view timeline's range starts or ends, as `entry 0%`. */
export const RangeOffset = schema.object({
    /** The range. */
    range: ViewRange,
    /** The share of the range, in percent. */
    percent: schema.number(),
});
/** Where a view timeline's range starts or ends. */
export type RangeOffset = schema.Infer<typeof RangeOffset>;

/** The scroller a scroll timeline follows: the nearest one around, the page's, or its own. */
export const Scroller = schema.enum(["nearest", "root", "self"]);
/** The scroller a scroll timeline follows. */
export type Scroller = schema.Infer<typeof Scroller>;

/** The axis a scroll or view timeline follows. */
export const TimelineAxis = schema.enum(["block", "inline"]);
/** The axis a scroll or view timeline follows. */
export type TimelineAxis = schema.Infer<typeof TimelineAxis>;

/** What an animation's progress follows: time, a scroller's position, or the element's view. */
export const Timeline = schema.discriminatedUnion("kind", [
    schema.object({
        /** Time. */
        kind: schema.literal("time"),
    }),
    schema.object({
        /** The position of a scroller. */
        kind: schema.literal("scroll"),
        /** The axis it follows. */
        axis: TimelineAxis,
        /** The scroller it follows. */
        scroller: Scroller,
    }),
    schema.object({
        /** The element's view in its scroller. */
        kind: schema.literal("view"),
        /** The axis it follows. */
        axis: TimelineAxis,
        /** Where its range starts. */
        start: RangeOffset,
        /** Where its range ends. */
        end: RangeOffset,
    }),
]);
/** What an animation's progress follows. */
export type Timeline = schema.Infer<typeof Timeline>;

/** An animation: tracks of keyframes, played over a time or a scroll, as CSS animations. */
export const MotionAnimation = schema
    .object({
        /** The tracks, one per property. */
        tracks: schema.array(MotionTrack),
        /** How long one iteration takes on a time timeline. */
        duration: Time.forward,
        /** How long it waits before it starts, a negative delay starting it partway, as in CSS. */
        delay: Time.bindable,
        /** How many times it plays, any number of times where infinite. */
        iterations: schema.union([schema.number().positive(), schema.literal("infinite")]),
        /** Which way each iteration plays. */
        direction: schema.enum(["normal", "reverse", "alternate", "alternate-reverse"]),
        /** Whether it holds its first frame before and its last after. */
        fill: schema.enum(["none", "forwards", "backwards", "both"]),
        /** What its progress follows. */
        timeline: Timeline,
    })
    .meta({ id: "style.motionAnimation", title: "Animation" });
/** An animation. */
export type MotionAnimation = schema.Infer<typeof MotionAnimation>;

/** A frame: the values an element moves to, and the transition it moves there with. */
export const MotionFrame = schema.object({
    /** The values, none for a frame that moves nothing. */
    values: schema.array(MotionValue),
    /** How it moves to them, as CSS times a transition by the style it moves to. */
    transition: Transition,
});
/** A frame: the values an element moves to, and how. */
export type MotionFrame = schema.Infer<typeof MotionFrame>;

/** How a parent delays each child's entry, exit and animations, one after another. */
export const Stagger = schema.object({
    /** The delay between one child and the next, none for all at once. */
    delay: Time.bindable,
    /** The child that starts first. */
    from: schema.enum(["first", "last"]),
});
/** How a parent delays each child, one after another. */
export type Stagger = schema.Infer<typeof Stagger>;

/** A name an element morphs under between views, as CSS's `view-transition-name` takes one. */
export const ViewTransitionName = schema.string().regex(/^(?!(?:none|auto)$)-?[_a-z][\w-]*$/iu, {
    message: "a view transition name is a CSS identifier other than none and auto",
});

/** How an element moves: between its states, as it enters and exits, and in its animations. */
const motionJson = schema
    .object({
        /** How it moves back to rest from a state. */
        transition: Transition,
        /** The frame of each state. */
        states: schema.array(MotionFrame.extend({ state: MotionState })),
        /** The frame it enters from, no values to appear at once. */
        enter: MotionFrame,
        /** The frame it exits to as it hides, no values to vanish at once. */
        exit: MotionFrame,
        /** Its animations. */
        animations: schema.array(MotionAnimation),
        /** How it delays its children's entries, exits and animations. */
        stagger: Stagger,
        /** The name it morphs under between views, none to cross-fade with the page. */
        viewTransition: schema.union([schema.literal("none"), ViewTransitionName]),
    })
    .meta({ id: "style.motion", title: "Motion" });
/** How an element moves. */
export type Motion = schema.Infer<typeof motionJson>;

/** How an element moves, and the CSS a browser plays it as. */
export const Motion = Object.assign(motionJson, {
    /** Write a motion as CSS rules for a selector, its keyframes named after a prefix. */
    css(motion: Motion, selector: string, name: string): string {
        // transition every property a frame moves, each frame timing the moves into it
        const frames = [...motion.states, motion.enter, motion.exit];
        const moved = frames.flatMap((frame) => frame.values);
        const properties = [...new Set(moved.flatMap((value) => Object.keys(valueCss(value))))];
        const isSized = moved.some(
            (value) => value.property === "width" || value.property === "height",
        );
        const rest = [
            properties.length === 0
                ? ""
                : `transition: ${Transition.css(motion.transition, properties)};`,
            isSized ? "interpolate-size: allow-keywords;" : "",
        ]
            .join(" ")
            .trim();

        // write the rest, each state's frame, and the exit's frame with its discrete display
        const exitTiming = Transition.timing(
            motion.exit.transition,
            staggered(motion.exit.transition),
        );
        const exit = [
            ...properties.map((property) => `${property} ${exitTiming}`),
            ...["display", "overlay"].map((property) => `${property} ${exitTiming} allow-discrete`),
        ].join(", ");
        const rules = [
            rest === "" ? "" : `${selector} { ${rest} }`,
            ...motion.states.map(
                (each) =>
                    `${selector}${MOTION_STATES[each.state].selector} { ${block(each.values)} transition: ${Transition.css(each.transition, properties)}; }`,
            ),
            motion.exit.values.length === 0
                ? ""
                : `${selector}${HIDDEN} { ${block(motion.exit.values)} transition: ${exit}; }`,
        ];

        // play the entry from its frame, then each track, all delayed by a parent's stagger
        const entry =
            motion.enter.values.length === 0
                ? []
                : [
                      {
                          css: `@keyframes ${name}-enter { from { ${block(motion.enter.values)} } }`,
                          animation: enterCss(motion.enter.transition, `${name}-enter`),
                          timeline: { kind: "time" } as const,
                      },
                  ];
        const tracks = motion.animations.flatMap((animation, index) =>
            animation.tracks.map((keyed, place) => {
                const named = `${name}-${String(index)}-${String(place)}`;

                return {
                    css: keyframesCss(keyed, named),
                    animation: animationCss(animation, named),
                    timeline: animation.timeline,
                };
            }),
        );
        const played = [...entry, ...tracks];
        const animations =
            played.length === 0
                ? ""
                : `${selector} { ${[
                      `animation: ${played.map((each) => each.animation).join(", ")}`,
                      `animation-timeline: ${played.map((each) => timelineCss(each.timeline)).join(", ")}`,
                      `animation-range: ${played.map((each) => rangeCss(each.timeline)).join(", ")}`,
                  ].join("; ")}; }`;

        // stagger the children, and morph between views under the motion's name
        const body = [
            ...rules,
            ...played.map((each) => each.css),
            animations,
            staggerCss(motion.stagger, selector),
            viewTransitionCss(motion, selector),
        ]
            .filter((rule) => rule !== "")
            .join("\n");

        // play it all only where a person allows motion, the stagger's delay registered on top
        const registered = isStaggered(motion.stagger) ? `${STAGGER_PROPERTY}\n` : "";

        return body === "" ? "" : `${registered}${MOTION_QUERY} {\n${body}\n}`;
    },
});

/** Describe one property's track: its property and keyframes of its value. */
function track<Property extends MotionProperty, Value extends schema.Schema>(
    property: Property,
    value: Value,
) {
    return schema.object({
        /** The property it animates. */
        property: schema.literal(property),
        /** Its keyframes, in order of their offsets. */
        keyframes: schema.array(
            schema.object({
                /** Where it falls in the iteration, from 0 to 1. */
                offset: schema.number().min(0).max(1),
                /** The value. */
                value,
                /** How it eases into the next keyframe. */
                easing: TimingFunction,
            }),
        ),
    });
}

/** Read keyframes as a property's track in order of their offsets, with one keyframe's place. */
function ordered(
    property: MotionProperty,
    keyframes: readonly Keyframe[],
    changed?: Keyframe,
): TrackPlace {
    const sorted = keyframes.toSorted((left, right) => left.offset - right.offset);
    const each = trackJson.parse({ property, keyframes: sorted });

    return { track: each, index: changed === undefined ? 0 : sorted.indexOf(changed) };
}

/** Write one value as the CSS properties it sets. */
function valueCss(value: MotionValue): Readonly<Record<string, string>> {
    switch (value.property) {
        case "opacity":
            return { opacity: String(value.value) };
        case "transform":
            return { ...Transform.css(value.value) };
        case "filters":
            return { filter: FilterStack.css(value.value) };
        case "shadows":
            return { "box-shadow": ShadowStack.css(value.value) };
        case "corners":
            return { "border-radius": Corners.css(value.value) };
        case "background":
            return { "background-color": Color.css(value.value) };
        case "color":
            return { color: Color.css(value.value) };
        case "width":
            return { width: Sizing.css(value.value) };
        case "height":
            return { height: Sizing.css(value.value) };
    }
}

/** Write values as a block of CSS declarations. */
function block(values: readonly MotionValue[]): string {
    return values
        .flatMap((value) => Object.entries(valueCss(value)))
        .map(([property, css]) => `${property}: ${css};`)
        .join(" ");
}

/** Write a track as CSS keyframes, each easing into the next. */
function keyframesCss(each: MotionTrack, name: string): string {
    const frames = each.keyframes.map((keyframe) => {
        // the keyframes of a track hold that track's kind of value
        const value = MotionValue.parse({ property: each.property, value: keyframe.value });
        const percent = `${String(Math.round(keyframe.offset * 10_000) / 100)}%`;

        return `${percent} { ${block([value])} animation-timing-function: ${TimingFunction.css(keyframe.easing)}; }`;
    });

    return `@keyframes ${name} { ${frames.join(" ")} }`;
}

/** Write an animation's timing for one of its tracks, as CSS's `animation` shorthand. */
function animationCss(animation: MotionAnimation, name: string): string {
    return [
        name,
        Time.css(animation.duration),
        `calc(${Time.css(animation.delay)} + var(${STAGGER}, 0ms))`,
        String(animation.iterations),
        animation.direction,
        animation.fill,
    ].join(" ");
}

/** Write the entry's animation from its frame, held at its start through its delay. */
function enterCss(transition: Transition, name: string): string {
    return `${name} ${Transition.timing(transition, staggered(transition))} backwards`;
}

/** Write a transition's delay with a parent's stagger added, as an entry or exit waits its turn. */
function staggered(transition: Transition): string {
    return `calc(${Time.css(transition.delay)} + var(${STAGGER}, 0ms))`;
}

/** Tell whether a stagger delays the children at all, a token always counting as one. */
function isStaggered(stagger: Stagger): boolean {
    return Reference.is(stagger.delay) || Duration.milliseconds(stagger.delay) !== 0;
}

/** Write how a parent staggers its children: an index from `sibling-index()`, else by position. */
function staggerCss(stagger: Stagger, selector: string): string {
    // leave a stagger of nothing unwritten
    const delay = Time.css(stagger.delay);
    if (!isStaggered(stagger)) {
        return "";
    }

    // count each child from the first or the last, by `sibling-index()` where the browser has it
    const index =
        stagger.from === "first" ? "(sibling-index() - 1)" : "(sibling-count() - sibling-index())";
    const position = stagger.from === "first" ? "nth-child" : "nth-last-child";
    const fallback = Array.from(
        { length: STAGGERED_CHILDREN },
        (_, place) =>
            `${selector} > :${position}(${String(place + 1)}) { ${STAGGER}: calc(${String(place)} * ${delay}); }`,
    );

    return [
        `@supports (order: sibling-index()) { ${selector} > * { ${STAGGER}: calc(${index} * ${delay}); } }`,
        `@supports not (order: sibling-index()) { ${fallback.join(" ")} }`,
    ].join("\n");
}

/** Write the name an element morphs under between views, timed as it moves back to rest. */
function viewTransitionCss(motion: Motion, selector: string): string {
    // leave an element that cross-fades with the page unnamed
    if (motion.viewTransition === "none") {
        return "";
    }
    const { transition } = motion;
    const timing = [
        `animation-duration: ${Time.css(TimingFunction.duration(transition.timingFunction, transition.duration))}`,
        `animation-timing-function: ${TimingFunction.css(transition.timingFunction)}`,
        `animation-delay: ${Time.css(transition.delay)}`,
    ].join("; ");

    return [
        `${selector} { view-transition-name: ${motion.viewTransition}; }`,
        `::view-transition-group(${motion.viewTransition}) { ${timing}; }`,
    ].join("\n");
}

/** Write what an animation's progress follows, as CSS's `animation-timeline`. */
function timelineCss(timeline: Timeline): string {
    switch (timeline.kind) {
        case "time":
            return "auto";
        case "scroll":
            return `scroll(${timeline.scroller} ${timeline.axis})`;
        case "view":
            return `view(${timeline.axis})`;
    }
}

/** Write the part of its view an animation plays over, as CSS's `animation-range`. */
function rangeCss(timeline: Timeline): string {
    return timeline.kind === "view"
        ? `${timeline.start.range} ${String(timeline.start.percent)}% ${timeline.end.range} ${String(timeline.end.percent)}%`
        : "normal";
}
