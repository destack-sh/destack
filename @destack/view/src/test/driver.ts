import { onTestFinished } from "@destack/test";
import type { Driver } from "@destack/test";
import type { Given, SetStep } from "@destack/package/declare";
import type { Environment } from "../scenario/interaction.ts";
import type { JsonValue } from "@destack/schema";
import type { JSX } from "@solidjs/web";
import { computeAccessibleName, getRole, isDisabled, isInaccessible } from "dom-accessibility-api";
import { createSignal, flush } from "solid-js";
import { renderExample } from "../example/frame.ts";
import {
    type Locator,
    OBSERVED_STATES,
    type Observation,
    type Step,
    viewInteraction,
} from "../scenario/index.ts";
import { stubPopovers } from "./dom.ts";

/** The modifiers of Playwright's key syntax, with the keyboard event flag each sets. */
const MODIFIERS = new Map([
    ["Control", "ctrlKey"],
    ["Meta", "metaKey"],
    ["Alt", "altKey"],
    ["Shift", "shiftKey"],
]);

/** A popover the test DOM shows closed, which browsers hide with its contents. */
const CLOSED_POPOVER = "[popover]:not([data-popover-open])";

/** An example a driver rendered, with the properties a set step changes. */
interface Rendered {
    /** The properties the example declares, which a set step may change. */
    readonly declared: readonly string[];
    /** Replace some of the properties. */
    readonly set: (properties: Readonly<Record<string, JsonValue>>) => void;
}

/** A driver playing UI scenarios in the test DOM in process, as a browser driver plays them through Playwright. */
export class ViewDriver implements Driver<typeof viewInteraction> {
    /** The interaction the driver speaks. */
    static readonly interaction = viewInteraction;

    /** The element holding the rendered examples. */
    readonly container: HTMLElement;
    /** The rendered examples, by name. */
    readonly #examples: ReadonlyMap<string, Rendered>;

    /** Hold the examples rendered into a container. */
    constructor(container: HTMLElement, examples: ReadonlyMap<string, Rendered>) {
        this.container = container;
        this.#examples = examples;
    }

    /** Render a scenario's examples into the document in its environment for the test's duration, standing in for the Popover API the test DOM lacks. */
    static start(given: Given<JSX.Element, Environment>): ViewDriver {
        // mount into the document until the test ends
        stubPopovers();
        const container = document.createElement("div");
        document.body.append(container);
        onTestFinished(() => container.remove());

        // render each example over properties a set step changes
        const examples = new Map<string, Rendered>();
        for (const example of given.examples) {
            const declared = Object.keys(example.properties);
            const [values, setValues] = createSignal<Record<string, unknown>>({
                ...example.properties,
            });
            const properties = Object.defineProperties(
                {},
                Object.fromEntries(
                    declared.map((key) => [key, { enumerable: true, get: () => values()[key] }]),
                ),
            );
            const element = document.createElement("div");
            container.append(element);
            const environment =
                given.environment === undefined ? {} : { environment: given.environment };
            onTestFinished(renderExample(element, { example, properties, ...environment }));
            examples.set(example.name, {
                declared,
                set: (changed) => setValues((current) => ({ ...current, ...changed })),
            });
        }

        return new ViewDriver(container, examples);
    }

    /** Take one step, letting the rendered examples follow. */
    act(step: SetStep | Step): void {
        switch (step.action) {
            case "focus":
                this.locate(step.target).focus();
                break;
            case "click":
                click(this.locate(step.target));
                break;
            case "rightClick":
                rightClick(this.locate(step.target), step.position);
                break;
            case "press":
                if (step.target !== undefined) {
                    this.locate(step.target).focus();
                }
                press(step.key);
                break;
            case "fill":
                fill(this.locate(step.target), step.value);
                break;
            case "set":
                this.#set(step.properties, step.example);
                break;
        }
        flush();
    }

    /** Read one observation. */
    observe(observation: Observation): JsonValue {
        switch (observation.kind) {
            case "focused":
                return focusedName();
            case "name":
                return computeAccessibleName(this.locate(observation.target));
            case "text":
                return textOf(this.locate(observation.target));
            case "texts":
                return this.locateAll(observation.target).map(textOf);
            case "value":
                return valueOf(this.locate(observation.target));
            case "attribute":
                return this.locate(observation.target).getAttribute(observation.name);
            case "state":
                return isInState(this.locate(observation.target), observation.state);
            case "visible":
                return this.locateAll(observation.target).some(isExposed);
            case "count":
                return this.locateAll(observation.target).length;
        }
    }

    /** Find the one element a locator matches, refusing none or several. */
    locate(locator: Locator): HTMLElement {
        const [element, ...others] = this.locateAll(locator);
        if (element === undefined || others.length > 0) {
            throw new TypeError(
                `${JSON.stringify(locator)} matches ${others.length + (element === undefined ? 0 : 1)} elements`,
            );
        }

        return element;
    }

    /** Find every element a locator matches in document order: role and label locators within the accessibility tree, the others anywhere. */
    locateAll(locator: Locator): HTMLElement[] {
        // search inside the located scope
        const scope = locator.within === undefined ? this.container : this.locate(locator.within);
        const elements = [...scope.querySelectorAll<HTMLElement>("*")];

        // match by role, label, text, test id or CSS
        let matched: HTMLElement[];
        if ("role" in locator) {
            matched = elements.filter((element) => isExposed(element) && isRole(element, locator));
        } else if ("label" in locator) {
            matched = elements.filter(
                (element) =>
                    isExposed(element) &&
                    isLabelled(element) &&
                    computeAccessibleName(element) === locator.label,
            );
        } else if ("text" in locator) {
            matched = elements.filter((element) => isInnermostText(element, locator.text));
        } else if ("testId" in locator) {
            matched = elements.filter((element) => element.dataset["testid"] === locator.testId);
        } else {
            matched = elements.filter((element) => element.matches(locator.css));
        }

        // take the nth match when asked
        if (locator.nth === undefined) {
            return matched;
        }
        const nth = matched[locator.nth];

        return nth === undefined ? [] : [nth];
    }

    /** Change properties an example declares. */
    #set(properties: Readonly<Record<string, JsonValue>>, name: string | undefined): void {
        // find the named example, or the only one
        const examples = [...this.#examples];
        const found = name === undefined ? examples[0] : examples.find(([key]) => key === name);
        if (found === undefined || (name === undefined && examples.length !== 1)) {
            throw new TypeError(`no one example ${name ?? ""} to set properties of`);
        }

        // refuse properties the example does not declare
        const [example, rendered] = found;
        const undeclared = Object.keys(properties).filter(
            (key) => !rendered.declared.includes(key),
        );
        if (undeclared.length > 0) {
            throw new TypeError(`example ${example} declares no ${undeclared.join(", ")}`);
        }
        rendered.set(properties);
    }
}

/** Press and release the main pointer button on an element, focusing it first as a pointer does. */
function click(element: HTMLElement): void {
    // press the button, focusing the element unless a handler prevents it
    element.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, cancelable: true }));
    const isFocusing = element.dispatchEvent(
        new MouseEvent("mousedown", { bubbles: true, cancelable: true }),
    );
    if (isFocusing) {
        element.focus();
    }

    // release the button and click
    element.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, cancelable: true }));
    element.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true }));
    element.click();
}

/** Open an element's context menu at a point relative to its top left corner, its center by default. */
function rightClick(
    element: HTMLElement,
    position: { readonly x: number; readonly y: number } | undefined,
): void {
    const box = element.getBoundingClientRect();
    const point = position ?? { x: box.width / 2, y: box.height / 2 };
    element.dispatchEvent(
        new MouseEvent("contextmenu", {
            button: 2,
            clientX: box.left + point.x,
            clientY: box.top + point.y,
            bubbles: true,
            cancelable: true,
        }),
    );
}

/** Press a key in Playwright's key syntax, such as `Control+r`, on the focused element or the body. */
function press(shortcut: string): void {
    // read the modifiers before the key
    const [key, ...held] = shortcut.split("+").toReversed();
    if (key === undefined || key === "") {
        throw new TypeError(`no key in ${shortcut}`);
    }
    const flags: Record<string, boolean> = {};
    for (const modifier of held) {
        const flag = MODIFIERS.get(modifier);
        if (flag === undefined) {
            throw new TypeError(`unknown modifier ${modifier} in ${shortcut}`);
        }
        flags[flag] = true;
    }

    // dispatch the key down and up on the focused element
    const target = document.activeElement ?? document.body;
    for (const type of ["keydown", "keyup"]) {
        target.dispatchEvent(
            new KeyboardEvent(type, {
                key,
                code: codeOf(key),
                ...flags,
                bubbles: true,
                cancelable: true,
            }),
        );
    }
}

/** Read the physical key a key value sits on in a US layout, such as `KeyT` for `t`. */
function codeOf(key: string): string {
    if (/^[a-z]$/iu.test(key)) {
        return `Key${key.toUpperCase()}`;
    } else if (/^[0-9]$/u.test(key)) {
        return `Digit${key}`;
    }

    return key === " " ? "Space" : key;
}

/** Replace a field's value as a person typing it would, focusing it first. */
function fill(element: HTMLElement, value: string): void {
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) {
        throw new TypeError("only a field can be filled");
    }
    element.focus();
    element.value = value;
    element.dispatchEvent(new InputEvent("input", { bubbles: true, data: value }));
}

/** Read the accessible name of the focused element, none when nothing has the focus. */
function focusedName(): string | null {
    const element = document.activeElement;

    return element === null || element === document.body ? null : computeAccessibleName(element);
}

/** Read an element's text with its whitespace collapsed. */
function textOf(element: Element): string {
    return (element.textContent ?? "").replaceAll(/\s+/gu, " ").trim();
}

/** Read a field's value, refusing an element that is no field. */
function valueOf(element: HTMLElement): string {
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) {
        throw new TypeError("only a field has a value");
    }

    return element.value;
}

/** Report whether an element is in the accessibility tree, a closed popover hiding its contents as browsers do. */
function isExposed(element: HTMLElement): boolean {
    return !isInaccessible(element) && element.closest(CLOSED_POPOVER) === null;
}

/** Report whether an element has a role, its accessible name and the states a role locator asks for. */
function isRole(element: HTMLElement, locator: Extract<Locator, { role: string }>): boolean {
    return (
        getRole(element) === locator.role &&
        (locator.name === undefined || computeAccessibleName(element) === locator.name) &&
        OBSERVED_STATES.every(
            (state) => locator[state] === undefined || isInState(element, state) === locator[state],
        )
    );
}

/** Report whether an element is in an ARIA state, reading native states where the element has them. */
function isInState(element: HTMLElement, state: (typeof OBSERVED_STATES)[number]): boolean {
    if (state === "disabled") {
        return isDisabled(element);
    } else if (state === "checked" && element instanceof HTMLInputElement) {
        return element.checked;
    }

    return element.getAttribute(`aria-${state}`) === "true";
}

/** Report whether a control takes its name from a label, aria-label or aria-labelledby. */
function isLabelled(element: HTMLElement): boolean {
    const labels = "labels" in element && element.labels instanceof NodeList ? element.labels : [];

    return (
        element.hasAttribute("aria-label") ||
        element.hasAttribute("aria-labelledby") ||
        labels.length > 0
    );
}

/** Report whether an element holds exactly a text and none of its children does. */
function isInnermostText(element: HTMLElement, text: string): boolean {
    return (
        textOf(element) === text && [...element.children].every((child) => textOf(child) !== text)
    );
}
