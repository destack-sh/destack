import { Motion, type Transition } from "@destack/style";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { MotionField } from "./index.ts";

/** CSS's ease over 200 milliseconds. */
const EASE: Transition = {
    duration: { milliseconds: 200 },
    delay: { milliseconds: 0 },
    timingFunction: [0.25, 0.1, 0.25, 1],
};

/** A motion with its transition alone. */
const QUICK: Motion = {
    transition: EASE,
    states: [],
    enter: { values: [], transition: EASE },
    exit: { values: [], transition: EASE },
    animations: [],
    stagger: { delay: { milliseconds: 0 }, from: "first" },
    viewTransition: "none",
};

test("add a state and an animation, the stage styled by the motion", () => {
    stubPopovers();
    const [motion, setMotion] = createSignal<Motion | undefined>(QUICK);
    const { container } = render(() => (
        <MotionField label="Motion" value={motion()} onValueChange={setMotion} />
    ));
    press(container, "Add state");
    press(container, "Add animation");

    // the first free state and the fade-in animation join the motion
    expect(motion()?.states).toEqual([{ state: "hover", values: [], transition: EASE }]);
    expect(
        motion()?.animations.map((animation) => animation.tracks.map((track) => track.property)),
    ).toEqual([["opacity"]]);

    // the stage styles its sample by the motion
    const name = container.querySelector("[data-motion]")?.getAttribute("data-motion") ?? "";
    expect(container.querySelector("style")?.textContent).toBe(
        Motion.css(Motion.parse(motion()), `[data-motion="${name}"]`, name),
    );
});

test("add a motion where none is set", () => {
    stubPopovers();
    const [motion, setMotion] = createSignal<Motion | undefined>(undefined);
    const { container } = render(() => (
        <MotionField label="Motion" value={motion()} onValueChange={setMotion} />
    ));
    press(container, "Add Motion");

    expect(motion()).toEqual(QUICK);
});

test("name the view transition once the text is a CSS identifier, and clear it to none", () => {
    stubPopovers();
    const given: Motion["viewTransition"][] = [];
    const { container } = render(() => (
        <MotionField
            label="Motion"
            value={QUICK}
            onValueChange={(next) => given.push(next.viewTransition)}
        />
    ));
    const input = [...container.querySelectorAll("input")].find(
        (element) => element.placeholder === "None",
    );
    const type = (text: string): string | undefined => {
        if (input !== undefined) {
            input.value = text;
            input.dispatchEvent(new Event("input", { bubbles: true }));
        }
        flush();

        return container.querySelector("[data-slot=field-error]")?.textContent ?? undefined;
    };
    const errors = ["1card", "card", ""].map(type);

    // only the number-led text shows what a name takes
    expect(errors).toEqual([
        "Start with a letter, then use letters, digits, - and _",
        undefined,
        undefined,
    ]);

    // the identifier and the cleared field reach the motion, the number-led text does not
    expect(given).toEqual(["card", "none"]);
});

/** Press a button by its accessible name or its text. */
function press(container: HTMLElement, name: string): void {
    const button = [...container.querySelectorAll<HTMLElement>("button")].find(
        (element) =>
            read(element.getAttribute("aria-label")) === name || read(element.textContent) === name,
    );
    expect(button, name).toBeDefined();
    button?.click();
    flush();
}

/** Read a text without the bidi isolates the locale wraps around interpolations. */
function read(text: string | null): string | undefined {
    return text?.replaceAll(/[\u2068\u2069]/gu, "").trim();
}
