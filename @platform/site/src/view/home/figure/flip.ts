import { flush } from "@destack/view";
import type { Display } from "@destack/environment/browser";

/** A shared part on screen and its box. */
type Place = { element: HTMLElement; box: DOMRect };

/** Apply a switch, moving each part both states share on screen from its old place to its new. */
export function flip(update: () => void, display: Display): void {
    // apply the change at once where the reader asks for less motion
    if (display.media("(prefers-reduced-motion: reduce)").matches) {
        update();
        return;
    }

    // read where each shared part sits, and which parts that enter are already in
    const before = placesOf();
    const entered = new Set(awake("[data-flip-enter]"));

    // apply the change and render it at once
    update();
    flush();

    // move each shared part from its old box into its new one, reading boxes only when parts were on screen
    const places = before.size > 0 ? placesOf() : new Map<string, Place>();
    for (const [key, after] of places) {
        // set a part that was elsewhere where it was, sized by height so text keeps its shape
        const from = before.get(key);
        if (from !== undefined && from.element !== after.element) {
            const scale = from.box.height / after.box.height;
            const x = from.box.left - after.box.left;
            const y = from.box.top - after.box.top;
            after.element.style.setProperty(
                "--flip-from",
                `translate(${x}px, ${y}px) scale(${scale})`,
            );
            play(after.element, "move");
        }
    }

    // slide in each part that enters with the change
    for (const element of awake("[data-flip-enter]")) {
        if (!entered.has(element)) {
            play(element, "enter");
        }
    }
}

/** Read each shared part on screen and its box, keyed by its figure and its name. */
function placesOf(): Map<string, Place> {
    const places = new Map<string, Place>();
    for (const element of awake("[data-flip]")) {
        // key the part by its figure, so two figures never trade parts
        const figure = element.closest("section")?.querySelector("header[id]")?.id;
        const box = element.getBoundingClientRect();
        if (figure !== undefined && box.width > 0) {
            places.set(`${figure}:${element.dataset["flip"] ?? ""}`, { element, box });
        }
    }

    return places;
}

/** List the elements a selector finds in figures on screen, leaving sleeping figures out. */
function awake(selector: string): HTMLElement[] {
    return [...document.querySelectorAll<HTMLElement>(selector)].filter(
        (element) => element.closest("[data-asleep]") === null,
    );
}

/** Play a part's move or entrance once, clearing it when it ends. */
function play(element: HTMLElement, kind: "move" | "enter"): void {
    // start the part's keyframes
    element.dataset["flipping"] = kind;

    // clear them once its own keyframes end, leaving its children's to them
    const end = (event: AnimationEvent) => {
        if (event.target === element) {
            delete element.dataset["flipping"];
            element.removeEventListener("animationend", end);
        }
    };
    element.addEventListener("animationend", end);
}
