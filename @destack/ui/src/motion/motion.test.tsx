import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import type { MotionTrack, TimingFunction } from "@destack/style";
import { render, stubPopovers } from "@destack/view/test";
import { preloadNamedInputs } from "../value-field/index.ts";
import { KeyframeTrack, TrackField } from "./index.ts";

await preloadNamedInputs();

test("move keyframes by arrows, further with Shift, within the track's time", () => {
    const [offsets, setOffsets] = createSignal<readonly number[]>([0, 0.5, 1]);
    const [selected, setSelected] = createSignal(0);
    const { container } = render(() => (
        <KeyframeTrack
            label="Opacity"
            offsets={offsets()}
            selected={selected()}
            disabled={false}
            onSelect={setSelected}
            onMove={(index, offset) => setOffsets((current) => current.with(index, offset))}
            onRemove={(index) => setOffsets((current) => current.toSpliced(index, 1))}
        />
    ));
    const [first, middle, last] = container.querySelectorAll<HTMLElement>(
        "[data-slot=keyframe-track-keyframe]",
    );
    press(middle, "ArrowRight");
    press(middle, "ArrowLeft", true);
    press(first, "ArrowLeft");
    press(last, "ArrowRight", true);

    // the middle moved a hundredth then back a tenth, the ends stayed within 0 and 1
    expect(offsets()).toEqual([0, 0.41, 1]);

    // the last key pressed selected its keyframe
    expect(selected()).toBe(2);
    expect(last?.getAttribute("aria-pressed")).toBe("true");

    // the delete key removes the keyframe it is pressed on, but never the only one
    press(first, "Delete");
    expect(offsets()).toEqual([0.41, 1]);
    press(
        container.querySelector<HTMLElement>("[data-slot=keyframe-track-keyframe]") ?? undefined,
        "Delete",
    );
    press(
        container.querySelector<HTMLElement>("[data-slot=keyframe-track-keyframe]") ?? undefined,
        "Delete",
    );
    expect(offsets()).toEqual([1]);
});

test("follow a keyframe as it moves past another, the last one without an easing", () => {
    stubPopovers();
    const linear: TimingFunction = [0, 0, 1, 1];
    const [track, setTrack] = createSignal<MotionTrack>({
        property: "opacity",
        keyframes: [
            { offset: 0, value: 0, easing: linear },
            { offset: 0.5, value: 0.5, easing: linear },
            { offset: 1, value: 1, easing: linear },
        ],
    });
    const { container } = render(() => (
        <TrackField value={track()} disabled={false} onValueChange={setTrack} onRemove={() => {}} />
    ));
    const keyframes = (): HTMLElement[] => [
        ...container.querySelectorAll<HTMLElement>("[data-slot=keyframe-track-keyframe]"),
    ];
    const legends = (): string[] =>
        [...container.querySelectorAll("legend")].map((legend) => legend.textContent.trim());

    // the first keyframe, moved past the middle one by arrows, keeps the focus in its new place
    keyframes()[0]?.focus();
    for (let step = 0; step < 7; step++) {
        press(focused(), "ArrowRight", true);
    }
    expect(track().keyframes.map((keyframe) => keyframe.value)).toEqual([0.5, 0, 1]);
    expect(keyframes().map((keyframe) => keyframe.getAttribute("aria-pressed"))).toEqual([
        "false",
        "true",
        "false",
    ]);
    expect(legends()).toEqual(["Opacity", "Value", "Easing"]);

    // the last keyframe eases into nothing, so it shows no easing
    keyframes()[2]?.click();
    flush();
    expect(legends()).toEqual(["Opacity", "Value"]);
});

/** Press a key on a keyframe, with Shift held where asked. */
function press(keyframe: HTMLElement | undefined, key: string, shiftKey = false): void {
    keyframe?.dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey, bubbles: true }));
    flush();
}

/** Read the focused element, none where the focus is on no element of the page. */
function focused(): HTMLElement | undefined {
    const active = document.activeElement;

    return active instanceof HTMLElement ? active : undefined;
}
