import type { BrowserEnvironment } from "@destack/environment/browser";
import type { Given } from "@destack/package/declare";
import type { JsonObject, JsonValue } from "@destack/schema";
import type { JSX } from "@solidjs/web";
import { computeAccessibleName, getRole, isDisabled, isInaccessible } from "dom-accessibility-api";
import { createSignal } from "solid-js";
import {
    type Globals,
    type Locator,
    OBSERVED_STATES,
    type Observation,
} from "../scenario/index.ts";
import { renderExample } from "./frame.ts";

/** The global a page exposes its stage as. */
export const STAGE = "destackStage";

/** A popover the test DOM marks as shown. */
const SHOWN_POPOVER = "[data-popover-open]";

/** A rendered example of a scenario. */
interface Rendered {
    /** The properties the example declares. */
    readonly declared: readonly string[];
    /** Replace some of the properties. */
    readonly set: (properties: JsonObject) => void;
}

/** The rendered examples of a UI scenario. */
export class Stage {
    /** The element around the rendered examples. */
    readonly container: HTMLElement;
    /** The rendered examples, by name. */
    readonly #examples: ReadonlyMap<string, Rendered>;
    /** The unmounting of the examples, last mounted first. */
    readonly #mounted: DisposableStack;

    /** Create a stage of rendered examples. */
    private constructor(
        container: HTMLElement,
        examples: ReadonlyMap<string, Rendered>,
        mounted: DisposableStack,
    ) {
        this.container = container;
        this.#examples = examples;
        this.#mounted = mounted;
    }

    /** Render the examples of a scenario into a container. */
    static render(
        container: HTMLElement,
        given: Given<JSX.Element, Globals>,
        environment: BrowserEnvironment,
    ): Stage {
        // render each example in order
        const mounted = new DisposableStack();
        const examples = new Map<string, Rendered>();
        for (const example of given.examples) {
            // render the example over properties a set step changes
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

            // mount it in its own element with the scenario's globals
            const element = container.ownerDocument.createElement("div");
            container.append(element);
            const globals = given.globals === undefined ? {} : { globals: given.globals };
            mounted.defer(renderExample(element, { example, properties, ...globals }, environment));
            examples.set(example.name, {
                declared,
                set: (changed) => setValues((current) => ({ ...current, ...changed })),
            });
        }

        return new Stage(container, examples, mounted);
    }

    /** Load a scenario from a module and render its examples. */
    static async load(
        container: HTMLElement,
        module: string,
        name: string,
        environment: BrowserEnvironment,
    ): Promise<Stage> {
        // find the scenario among the module's exports
        const exports: unknown = await import(/* @vite-ignore */ module);
        const scenario = Object.values(exports ?? {}).find(
            (each): each is { readonly given: Given<JSX.Element, Globals> } =>
                typeof each === "object" &&
                each !== null &&
                "given" in each &&
                Reflect.get(each, "name") === name,
        );
        if (scenario === undefined) {
            throw new TypeError(`module '${module}' exports no scenario '${name}'`);
        }

        return Stage.render(container, scenario.given, environment);
    }

    /** Unmount the examples. */
    dispose(): void {
        this.#mounted.dispose();
    }

    /** Change properties of an example. */
    set(properties: JsonObject, name: string | undefined): void {
        // find the example by name, or the only one
        const examples = [...this.#examples];
        const found = name === undefined ? examples[0] : examples.find(([key]) => key === name);
        if (found === undefined || (name === undefined && examples.length !== 1)) {
            throw new TypeError(`no single example '${name ?? ""}' to set properties of`);
        }

        // refuse undeclared properties
        const [example, rendered] = found;
        const undeclared = Object.keys(properties).filter(
            (key) => !rendered.declared.includes(key),
        );
        if (undeclared.length > 0) {
            throw new TypeError(`example '${example}' does not declare '${undeclared.join(", ")}'`);
        }
        rendered.set(properties);
    }

    /** Read the viewport point on the element a locator matches. */
    point(
        locator: Locator,
        position?: { readonly x: number; readonly y: number },
    ): { readonly x: number; readonly y: number } {
        const box = this.locate(locator).getBoundingClientRect();

        return {
            x: box.left + (position?.x ?? box.width / 2),
            y: box.top + (position?.y ?? box.height / 2),
        };
    }

    /** Move the focus to the element a locator matches. */
    focus(locator: Locator, isSelecting: boolean): void {
        // focus the element and select a field's text
        const element = this.locate(locator);
        element.focus();
        const isField =
            element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement;
        if (isSelecting && isField) {
            element.select();
        }
    }

    /** Read one observation. */
    observe(observation: Observation): JsonValue {
        switch (observation.kind) {
            case "focused":
                return focusedName();
            case "name":
                return computeAccessibleName(this.locate(observation.target));
            case "text":
                return readText(this.locate(observation.target));
            case "texts":
                return this.locateAll(observation.target).map(readText);
            case "value":
                return readValue(this.locate(observation.target));
            case "attribute":
                return this.locate(observation.target).getAttribute(observation.name);
            case "state":
                return isInState(this.locate(observation.target), observation.state);
            case "visible":
                return this.locateAll(observation.target).some(isVisible);
            case "count":
                return this.locateAll(observation.target).length;
        }
    }

    /** Find the one element a locator matches. */
    locate(locator: Locator): HTMLElement {
        const [element, ...others] = this.locateAll(locator);
        if (element === undefined || others.length > 0) {
            throw new TypeError(
                `${JSON.stringify(locator)} matches '${others.length + (element === undefined ? 0 : 1)}' elements`,
            );
        }

        return element;
    }

    /** Find every element a locator matches. */
    locateAll(locator: Locator): HTMLElement[] {
        // search inside the located scope
        const scope = locator.within === undefined ? this.container : this.locate(locator.within);
        const elements = [...scope.querySelectorAll<HTMLElement>("*")];

        // match by role, label, text, test id or CSS
        let matched: HTMLElement[];
        if ("role" in locator) {
            matched = elements.filter((element) => isRole(element, locator) && isExposed(element));
        } else if ("label" in locator) {
            matched = elements.filter(
                (element) =>
                    isLabelled(element) &&
                    computeAccessibleName(element) === locator.label &&
                    isExposed(element),
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
}

/** Render a scenario's examples in a page and expose their stage. */
export async function hostScenario(
    element: HTMLElement,
    module: string,
    name: string,
    environment: BrowserEnvironment,
): Promise<void> {
    Reflect.set(globalThis, STAGE, await Stage.load(element, module, name, environment));
}

/** Read the accessible name of the focused element. */
function focusedName(): string | null {
    const element = document.activeElement;

    return element === null || element === document.body ? null : computeAccessibleName(element);
}

/** Read an element's text with its whitespace collapsed. */
function readText(element: Element): string {
    return (element.textContent ?? "").replaceAll(/\s+/gu, " ").trim();
}

/** Read the value of a field. */
function readValue(element: HTMLElement): string {
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) {
        throw new TypeError("only a field has a value");
    }

    return element.value;
}

/** Report whether an element is in the accessibility tree. */
function isExposed(element: HTMLElement): boolean {
    const popover = element.closest("[popover]");

    return !isInaccessible(element) && (popover === null || isShown(popover));
}

/** Report whether a popover is shown. */
function isShown(popover: Element): boolean {
    const isMatched = typeof CSS !== "undefined" && CSS.supports("selector(:popover-open)");

    return popover.matches(SHOWN_POPOVER) || (isMatched && popover.matches(":popover-open"));
}

/** Report whether a person can see an element. */
function isVisible(element: HTMLElement): boolean {
    // count an element without a box as seen
    const box = element.getBoundingClientRect();
    if (!isRendered(element)) {
        return false;
    } else if (box.width === 0 && box.height === 0) {
        return true;
    }

    // count an element in the top layer as seen
    const layer = element.closest("[popover], dialog");
    if (layer !== null && (layer.hasAttribute("popover") ? isShown(layer) : isModal(layer))) {
        return true;
    }

    // refuse an element that lies outside an ancestor clipping its overflow
    for (let each = element.parentElement; each !== null; each = each.parentElement) {
        const clip = each.getBoundingClientRect();
        const isClipping = getComputedStyle(each).overflow !== "visible";
        const isOutside =
            box.right <= clip.left ||
            box.left >= clip.right ||
            box.bottom <= clip.top ||
            box.top >= clip.bottom;
        if (isClipping && isOutside) {
            return false;
        }
    }

    return true;
}

/** Report whether an element matches a role locator. */
function isRole(element: HTMLElement, locator: Extract<Locator, { role: string }>): boolean {
    return (
        getRole(element) === locator.role &&
        (locator.name === undefined || computeAccessibleName(element) === locator.name) &&
        OBSERVED_STATES.every(
            (state) => locator[state] === undefined || isInState(element, state) === locator[state],
        )
    );
}

/** Report whether an element is in an ARIA state. */
function isInState(element: HTMLElement, state: (typeof OBSERVED_STATES)[number]): boolean {
    if (state === "disabled") {
        return isDisabled(element);
    } else if (state === "checked" && element instanceof HTMLInputElement) {
        return element.checked;
    }

    return element.getAttribute(`aria-${state}`) === "true";
}

/** Report whether a control has a label. */
function isLabelled(element: HTMLElement): boolean {
    const labels = "labels" in element && element.labels instanceof NodeList ? element.labels : [];

    return (
        element.hasAttribute("aria-label") ||
        element.hasAttribute("aria-labelledby") ||
        labels.length > 0
    );
}

/** Report whether an element is the innermost with a text. */
function isInnermostText(element: HTMLElement, text: string): boolean {
    return (
        readText(element) === text &&
        [...element.children].every((child) => readText(child) !== text)
    );
}

/** Report whether an element is drawn. */
function isRendered(element: HTMLElement): boolean {
    // refuse an element hidden by its style
    const { visibility } = getComputedStyle(element);
    if (visibility === "hidden" || visibility === "collapse") {
        return false;
    }

    // refuse an element inside a hidden element or a closed popover
    for (let each: HTMLElement | null = element; each !== null; each = each.parentElement) {
        const isClosed = each.hasAttribute("popover") && !isShown(each);
        if (each.hidden !== false || getComputedStyle(each).display === "none" || isClosed) {
            return false;
        }
    }

    return true;
}

/** Report whether a dialog is modal. */
function isModal(dialog: Element): boolean {
    const isMatched = typeof CSS !== "undefined" && CSS.supports("selector(:modal)");

    return isMatched && dialog.matches(":modal");
}
