import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { draw, press } from "@destack/view/test";
import {
    DateSegmentGroup,
    type DateSegmentFormat,
    type DateSegmentKind,
    type DateSegmentValues,
} from "./index.ts";

/** A clock of hours and minutes, which steps an empty segment from 9:30. */
const CLOCK: DateSegmentFormat = {
    layout: [
        { type: "hour", value: "" },
        { type: "literal", value: ":" },
        { type: "minute", value: "" },
    ],
    limits: (kind) => (kind === "hour" ? [0, 23] : [0, 59]),
    text: (_kind, value) => String(value).padStart(2, "0"),
    valueText: (kind, value) => `${String(value)} ${kind}`,
    name: (kind) => kind,
    now: (kind) => (kind === "hour" ? 9 : 30),
};

/** A step of a walk through the segments: the key, the values after it and the focused segment. */
interface Step {
    /** The key pressed. */
    readonly key: string;
    /** The values after the key. */
    readonly values: DateSegmentValues;
    /** Whether the focused segment still takes more digits. */
    readonly isTyping: boolean;
    /** The segment that holds the focus after the key. */
    readonly focused: string | undefined;
}

/** Render a clock from values, focus a segment and press keys in turn, reading each step. */
function walk(start: DateSegmentValues, kind: DateSegmentKind, keys: readonly string[]): Step[] {
    // render the clock over values it follows
    const [values, setValues] = createSignal(start);
    let isTyping = false;
    const container = draw(() => (
        <DateSegmentGroup
            aria-label="Time"
            format={CLOCK}
            values={values()}
            onValuesChange={(next, typing) => {
                setValues(next);
                isTyping = typing;
            }}
        />
    ));
    container.querySelector<HTMLElement>(`[data-segment=${kind}]`)?.focus();

    // press each key on the focused segment
    return keys.map((key) => {
        press(key);
        flush();
        const focused = document.activeElement;

        return {
            key,
            values: values(),
            isTyping,
            focused: focused instanceof HTMLElement ? focused.dataset["segment"] : undefined,
        };
    });
}

test("step a segment from now while empty and wrap it at its bounds", () => {
    expect(walk({ minute: 59 }, "hour", ["ArrowUp", "ArrowDown", "Tab"])).toEqual([
        { key: "ArrowUp", values: { minute: 59, hour: 9 }, isTyping: false, focused: "hour" },
        { key: "ArrowDown", values: { minute: 59, hour: 8 }, isTyping: false, focused: "hour" },
        { key: "Tab", values: { minute: 59, hour: 8 }, isTyping: false, focused: "hour" },
    ]);
    expect(walk({ hour: 23, minute: 0 }, "hour", ["ArrowUp", "ArrowRight", "ArrowDown"])).toEqual([
        { key: "ArrowUp", values: { minute: 0, hour: 0 }, isTyping: false, focused: "hour" },
        { key: "ArrowRight", values: { minute: 0, hour: 0 }, isTyping: false, focused: "minute" },
        { key: "ArrowDown", values: { hour: 0, minute: 59 }, isTyping: false, focused: "minute" },
    ]);
});

test("type digits into a segment, moving on once no further digit fits and starting over past the largest value", () => {
    expect(walk({}, "hour", ["1", "4", "7", "ArrowLeft", "2", "5"])).toEqual([
        { key: "1", values: { hour: 1 }, isTyping: true, focused: "hour" },
        { key: "4", values: { hour: 14 }, isTyping: false, focused: "minute" },
        { key: "7", values: { hour: 14, minute: 7 }, isTyping: false, focused: "minute" },
        { key: "ArrowLeft", values: { hour: 14, minute: 7 }, isTyping: false, focused: "hour" },
        { key: "2", values: { minute: 7, hour: 2 }, isTyping: true, focused: "hour" },
        { key: "5", values: { minute: 7, hour: 5 }, isTyping: false, focused: "minute" },
    ]);
});

test("clear a segment on Backspace and finish typing as the focus leaves", () => {
    expect(walk({ hour: 9, minute: 30 }, "minute", ["Backspace", "4", "ArrowLeft"])).toEqual([
        { key: "Backspace", values: { hour: 9 }, isTyping: false, focused: "minute" },
        { key: "4", values: { hour: 9, minute: 4 }, isTyping: true, focused: "minute" },
        { key: "ArrowLeft", values: { hour: 9, minute: 4 }, isTyping: false, focused: "hour" },
    ]);
});

test("show each segment's value or a placeholder and expose it as a spinbutton", () => {
    const container = draw(() => (
        <DateSegmentGroup
            aria-label="Time"
            format={CLOCK}
            values={{ hour: 7 }}
            onValuesChange={() => undefined}
        />
    ));
    const segments = [...container.querySelectorAll("[role=spinbutton]")].map((segment) => [
        segment.textContent,
        segment.getAttribute("aria-valuenow"),
        segment.getAttribute("aria-valuetext"),
        segment.getAttribute("aria-valuemax"),
    ]);

    expect(segments).toEqual([
        ["07", "7", "7 hour", "23"],
        ["––", null, "Empty", "59"],
    ]);
});
