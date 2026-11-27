import { expect, test } from "@destack/test";
import type { TimingFunction, Transition } from "../transition/index.ts";
import { Motion, MotionTrack, ViewTransitionName } from "./index.ts";

test("write a motion as CSS: its frames, entry, exit, tracks, stagger and view transition", () => {
    const quick: Transition = {
        duration: { milliseconds: 200 },
        delay: { milliseconds: 0 },
        timingFunction: [0.4, 0, 0.2, 1],
    };
    const slow: Transition = { ...quick, duration: { milliseconds: 400 } };
    const motion: Motion = {
        transition: quick,
        states: [
            { state: "hover", values: [{ property: "opacity", value: 0.9 }], transition: slow },
            {
                state: "press",
                values: [{ property: "background", value: "{color.muted}" }],
                transition: quick,
            },
        ],
        enter: { values: [{ property: "opacity", value: 0 }], transition: slow },
        exit: { values: [{ property: "opacity", value: 0 }], transition: quick },
        animations: [
            {
                tracks: [
                    {
                        property: "opacity",
                        keyframes: [
                            { offset: 0, value: 0, easing: [0, 0, 1, 1] },
                            { offset: 1, value: 1, easing: [0, 0, 1, 1] },
                        ],
                    },
                ],
                duration: { milliseconds: 1000 },
                delay: { milliseconds: 0 },
                iterations: 1,
                direction: "normal",
                fill: "both",
                timeline: { kind: "scroll", axis: "block", scroller: "nearest" },
            },
        ],
        stagger: { delay: { milliseconds: 50 }, from: "first" },
        viewTransition: "card",
    };

    // each child of the 24 the fallback reaches takes its place times the stagger
    const fallback = Array.from(
        { length: 24 },
        (_, place) =>
            `.card > :nth-child(${String(place + 1)}) { --destack-motion-stagger: calc(${String(place)} * 50ms); }`,
    ).join(" ");

    expect(Motion.css(motion, ".card", "card").split("\n")).toEqual([
        '@property --destack-motion-stagger { syntax: "<time>"; inherits: false; initial-value: 0ms; }',
        "@media (prefers-reduced-motion: no-preference) {",
        ".card { transition: opacity 200ms cubic-bezier(0.4, 0, 0.2, 1) 0ms, background-color 200ms cubic-bezier(0.4, 0, 0.2, 1) 0ms; }",
        ".card:hover { opacity: 0.9; transition: opacity 400ms cubic-bezier(0.4, 0, 0.2, 1) 0ms, background-color 400ms cubic-bezier(0.4, 0, 0.2, 1) 0ms; }",
        ".card:active { background-color: var(--destack-color-muted); transition: opacity 200ms cubic-bezier(0.4, 0, 0.2, 1) 0ms, background-color 200ms cubic-bezier(0.4, 0, 0.2, 1) 0ms; }",
        ".card:is([hidden], [data-closed]) { opacity: 0; transition: opacity 200ms cubic-bezier(0.4, 0, 0.2, 1) calc(0ms + var(--destack-motion-stagger, 0ms)), background-color 200ms cubic-bezier(0.4, 0, 0.2, 1) calc(0ms + var(--destack-motion-stagger, 0ms)), display 200ms cubic-bezier(0.4, 0, 0.2, 1) calc(0ms + var(--destack-motion-stagger, 0ms)) allow-discrete, overlay 200ms cubic-bezier(0.4, 0, 0.2, 1) calc(0ms + var(--destack-motion-stagger, 0ms)) allow-discrete; }",
        "@keyframes card-enter { from { opacity: 0; } }",
        "@keyframes card-0-0 { 0% { opacity: 0; animation-timing-function: linear; } 100% { opacity: 1; animation-timing-function: linear; } }",
        ".card { animation: card-enter 400ms cubic-bezier(0.4, 0, 0.2, 1) calc(0ms + var(--destack-motion-stagger, 0ms)) backwards, card-0-0 1000ms calc(0ms + var(--destack-motion-stagger, 0ms)) 1 normal both; animation-timeline: auto, scroll(nearest block); animation-range: normal, normal; }",
        "@supports (order: sibling-index()) { .card > * { --destack-motion-stagger: calc((sibling-index() - 1) * 50ms); } }",
        `@supports not (order: sibling-index()) { ${fallback} }`,
        ".card { view-transition-name: card; }",
        "::view-transition-group(card) { animation-duration: 200ms; animation-timing-function: cubic-bezier(0.4, 0, 0.2, 1); animation-delay: 0ms; }",
        "}",
    ]);
});

test("write nothing for a motion that moves nothing", () => {
    const still: Transition = {
        duration: { milliseconds: 200 },
        delay: { milliseconds: 0 },
        timingFunction: { kind: "spring", duration: { milliseconds: 400 }, bounce: 0.2 },
    };

    expect(
        Motion.css(
            {
                transition: still,
                states: [],
                enter: { values: [], transition: still },
                exit: { values: [], transition: still },
                animations: [],
                stagger: { delay: { milliseconds: 0 }, from: "first" },
                viewTransition: "none",
            },
            ".card",
            "card",
        ),
    ).toBe("");
});

test("stagger children from the last by a token, the last one first", () => {
    const still: Transition = {
        duration: { milliseconds: 200 },
        delay: { milliseconds: 0 },
        timingFunction: [0.25, 0.1, 0.25, 1],
    };
    const lines = Motion.css(
        {
            transition: still,
            states: [],
            enter: { values: [], transition: still },
            exit: { values: [], transition: still },
            animations: [],
            stagger: { delay: "{motion.duration.short}", from: "last" },
            viewTransition: "none",
        },
        ".list",
        "list",
    ).split("\n");

    // count each child back from the last, by sibling-index() or by its place from the end
    const token = "var(--destack-motion-duration-short)";
    const fallback = Array.from(
        { length: 24 },
        (_, place) =>
            `.list > :nth-last-child(${String(place + 1)}) { --destack-motion-stagger: calc(${String(place)} * ${token}); }`,
    ).join(" ");
    expect(lines.slice(2, 4)).toEqual([
        `@supports (order: sibling-index()) { .list > * { --destack-motion-stagger: calc((sibling-count() - sibling-index()) * ${token}); } }`,
        `@supports not (order: sibling-index()) { ${fallback} }`,
    ]);
});

test("take a view transition name only where CSS takes it", () => {
    expect(
        ["card", "-hero_1", "none", "auto", "1card", "two words", ""].map(
            (name) => ViewTransitionName.safeParse(name).success,
        ),
    ).toEqual([true, true, false, false, false, false, false]);
});

test("keep a track's keyframes in order of their offsets as they move, split and go", () => {
    const linear: TimingFunction = [0, 0, 1, 1];
    const fade: MotionTrack = {
        property: "opacity",
        keyframes: [
            { offset: 0, value: 0, easing: linear },
            { offset: 0.5, value: 0.5, easing: linear },
            { offset: 1, value: 1, easing: linear },
        ],
    };

    // moving the first keyframe past the middle one puts it second, its place following it
    const moved = MotionTrack.set(fade, 0, { offset: 0.75 });
    expect(moved.index).toBe(1);
    expect(moved.track.keyframes.map((keyframe) => [keyframe.offset, keyframe.value])).toEqual([
        [0.5, 0.5],
        [0.75, 0],
        [1, 1],
    ]);

    // splitting the first adds a copy halfway to the next, splitting the last halfway back
    const first = MotionTrack.split(fade, 0);
    const last = MotionTrack.split(fade, 2);
    expect([first.index, first.track.keyframes[1]]).toEqual([
        1,
        { offset: 0.25, value: 0, easing: linear },
    ]);
    expect([last.index, last.track.keyframes[2]]).toEqual([
        2,
        { offset: 0.75, value: 1, easing: linear },
    ]);

    // removing refuses the only keyframe a track has
    const single = MotionTrack.remove(MotionTrack.remove(fade, 0), 0);
    expect(single.keyframes).toEqual([{ offset: 1, value: 1, easing: linear }]);
    expect(() => MotionTrack.remove(single, 0)).toThrow(
        new RangeError("a track keeps at least one keyframe"),
    );
});

test("start a track at CSS's initial value", () => {
    expect(MotionTrack.start("opacity").keyframes.map((keyframe) => keyframe.value)).toEqual([
        1, 1,
    ]);
});
