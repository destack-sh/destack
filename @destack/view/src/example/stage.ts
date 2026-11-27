import type { BrowserEnvironment } from "@destack/environment/browser";
import {
    type Example,
    type Given,
    Operation,
    Reference,
    type Scenario,
    SetStep,
} from "@destack/package/declare";
import { type JsonObject, type JsonValue, present, schema } from "@destack/schema";
import { ActionInput, type ActionInputDescription } from "@destack/object";
import { Component } from "../declare/component.ts";
import { Catalog } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { CONTEXT_ELEMENT, ViewContext } from "../declare/context.ts";
import type { JSX } from "@solidjs/web";
import { computeAccessibleName, getRole, isDisabled, isInaccessible } from "dom-accessibility-api";
import { createSignal, flush } from "solid-js";
import { Globals, type Locator, OBSERVED_STATES, Observation, Step } from "../scenario/index.ts";
import { Keyboard } from "./keyboard.ts";
import { Pointer } from "./pointer.ts";
import { renderExample } from "./frame.ts";
import { Matrix } from "./matrix.ts";
import { type CheckResult, ExampleRequest, MatrixAxes, ExampleEvent } from "../page/message.ts";
import type { IndexEntry } from "../page/example.ts";

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
    /** The pointer the steps point with. */
    readonly #pointer: Pointer;
    /** The keyboard the steps type with. */
    readonly #keyboard: Keyboard;

    /** Create a stage of rendered examples. */
    private constructor(
        container: HTMLElement,
        examples: ReadonlyMap<string, Rendered>,
        mounted: DisposableStack,
        pointer: Pointer,
    ) {
        // store the container, the examples, their unmounting, the pointer and the keyboard
        this.container = container;
        this.#examples = examples;
        this.#mounted = mounted;
        this.#pointer = pointer;
        this.#keyboard = new Keyboard(container.ownerDocument);
    }

    /** Render the examples of a scenario into a container, translated by the catalogs. */
    static render(
        container: HTMLElement,
        given: Given<JSX.Element, Globals>,
        environment: BrowserEnvironment,
        page: {
            readonly catalogs?: readonly Catalog[];
            readonly index?: readonly IndexEntry[];
        } = {},
    ): Stage {
        // render each example in order
        const mounted = new DisposableStack();
        const examples = new Map<string, Rendered>();
        for (const example of given.examples) {
            // render the example over the properties a set step or a host changes: its own and its component's
            const definitions = Component.is(example.of) ? example.of.properties : {};
            const declared = [
                ...new Set([...Object.keys(example.properties), ...Object.keys(definitions)]),
            ];
            const defaults = Component.is(example.of) ? example.of.defaults : {};
            const [values, setValues] = createSignal<Record<string, unknown>>({
                ...defaults,
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
            mounted.defer(
                renderExample(element, { example, properties, ...page, ...globals }, environment),
            );
            examples.set(example.name, {
                declared,
                set: (changed) => setValues((current) => ({ ...current, ...changed })),
            });
        }

        return new Stage(
            container,
            examples,
            mounted,
            new Pointer(given.globals?.pointer ?? "mouse"),
        );
    }

    /** Host a page's examples: show the one its address gives, then each one its host asks for. */
    static async host(
        element: HTMLElement,
        library: Library,
        owner: PackageId,
        environment: BrowserEnvironment,
    ): Promise<void> {
        // fetch every catalog the page's builds serve, as its context lists them
        const catalogs = await fetchCatalogs(owner, environment);

        // show what the address gives, talking to a host of the page's own origin
        const url = new URL(environment.navigation.currentEntry.url);
        const { origin } = url;

        // keep a look-only page, such as a matrix's cell, out of the focus and the pointer
        const isInert = url.searchParams.has("inert");
        element.inert = isInert;

        // let go of the focus a modal dialog takes, which the platform exempts from inertness
        if (isInert) {
            element.ownerDocument.addEventListener("focusin", release, { capture: true });
        }
        let current = await show(element, library, catalogs, environment, origin, {
            name: present(url.searchParams.get("name"), "the shown name"),
            globals: schema.json().parse(JSON.parse(url.searchParams.get("globals") ?? "{}")),
            properties: schema
                .jsonObject()
                .parse(JSON.parse(url.searchParams.get("properties") ?? "{}")),
            matrix: MatrixAxes.nullable().parse(
                JSON.parse(url.searchParams.get("matrix") ?? "null"),
            ),
        });

        // show what the host asks for next in place, passing every other request to what is shown
        let showing = Promise.resolve();
        window.addEventListener("message", (event: MessageEvent<unknown>) => {
            // take only the parent's requests of this origin
            const request =
                event.source === window.parent ? ExampleRequest.read(event, origin) : undefined;
            if (request === undefined) {
                return;
            }
            // show another example or scenario, one at a time in the order asked, taking the last down once the next is drawn
            if (request.destack === "show") {
                showing = showing.then(async () => {
                    const next = await show(
                        element,
                        library,
                        catalogs,
                        environment,
                        origin,
                        request,
                    );
                    current.dispose();
                    current = next;
                });
            }
            // pass it on, after what is being shown
            else {
                void showing.then(() => current.receive(request));
            }
        });
    }

    /** Act one step of a scenario, returning the result of each of its checks. */
    async play(
        steps: readonly { readonly step: SetStep | Step; readonly checks: readonly unknown[] }[],
        index: number,
        pause: (milliseconds: number) => Promise<void>,
    ): Promise<CheckResult[]> {
        // act the step, waiting out a pause by the page's clock
        const { step, checks } = present(steps[index], `step ${String(index)} of the scenario`);
        await this.act(step, pause);

        // observe each check, comparing what it saw with what it expects
        const results = checks.flatMap((check) => {
            // skip a check that reads telemetry, which a page does not record
            const parsed = ObservedCheck.safeParse(check);
            if (!parsed.success) {
                return [];
            }

            // compare what the examples show with what the check expects
            const seen = this.observe(parsed.data.observe);

            return [{ seen, isPassed: JSON.stringify(seen) === JSON.stringify(parsed.data.is) }];
        });

        return results;
    }

    /** Take one step on the examples as a person would, waiting out a pause by a clock. */
    async act(step: SetStep | Step, pause: (milliseconds: number) => Promise<void>): Promise<void> {
        switch (step.action) {
            case "focus":
                this.locate(step.target).focus();
                break;
            case "click":
                this.#pointer.move(this.locate(step.target), step.position);
                this.#pointer.press(step.button);
                this.#pointer.release();
                break;
            case "pointerMove":
                this.#pointer.move(this.locate(step.target), step.position);
                break;
            case "pointerDown":
                this.#pointer.press(step.button);
                break;
            case "pointerUp":
                this.#pointer.release();
                break;
            case "wheel":
                this.#pointer.wheel(step.deltaX ?? 0, step.deltaY ?? 0);
                break;
            case "pause":
                await pause(step.milliseconds);
                break;
            case "press":
                // focus the target and let the focus settle, as a browser does before the key
                if (step.target !== undefined) {
                    this.locate(step.target).focus();
                    flush();
                }
                this.#keyboard.press(step.key);
                break;
            case "fill":
                this.#keyboard.fill(this.locate(step.target), step.value);
                break;
            case "set":
                this.set(step.properties, step.example);
                break;
        }
        flush();
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
            case "names":
                return this.locateAll(observation.target).map((element) =>
                    computeAccessibleName(element),
                );
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

/** Report whether a module's export is an example or a scenario a page shows, by its kind. */
function isShowable(
    value: unknown,
): value is Example<object, JSX.Element> | Scenario<unknown, unknown, Globals, JSX.Element> {
    return (
        typeof value === "object" &&
        value !== null &&
        "kind" in value &&
        (value.kind === "example" || value.kind === "scenario")
    );
}

/** A page's examples and scenarios by name, each with its kind and how to load it. */
type Library = Readonly<
    Record<string, Omit<IndexEntry, "name"> & { readonly load: () => Promise<unknown> }>
>;

/** What a page shows: an example or scenario by name, its globals, properties and matrix axes. */
type Address = Omit<Extract<ExampleRequest, { readonly destack: "show" }>, "destack">;

/** What a page shows now: how it takes its host's requests, and how to take it down. */
interface Presentation {
    /** Take a request of the host. */
    readonly receive: (request: ExampleRequest) => void;
    /** Take it down, leaving the element empty. */
    readonly dispose: () => void;
}

/** Show an example alone, laid out as a matrix, or a scenario, telling the host what it may do. */
async function show(
    container: HTMLElement,
    library: Library,
    catalogs: readonly Catalog[],
    environment: BrowserEnvironment,
    origin: string,
    address: Address,
): Promise<Presentation> {
    // load the declaration by its name, refusing one that is no example or scenario
    const { name } = address;
    const entry = present(library[name], `the example or scenario '${name}' of the page`);
    const value = await entry.load();
    if (!isShowable(value)) {
        throw new TypeError(`'${name}' is invalid: it is no example or scenario`);
    }

    // show it in its own element, removed with it
    const element = container.ownerDocument.createElement("div");
    container.append(element);
    const globals = Globals.parse(address.globals);
    const page = { element, library, catalogs, environment, origin, globals };

    // lay an example out along its axes, else show it alone, else play a scenario
    if (value.kind === "example" && address.matrix !== null) {
        return showMatrix(value, name, address.properties, address.matrix, page);
    } else if (value.kind === "example") {
        return showExample(value, address.properties, page);
    } else {
        return showScenario(value, page);
    }
}

/** The page an example or scenario shows on: its element, library, catalogs and globals. */
interface Page {
    /** The element it shows in, which it removes once taken down. */
    readonly element: HTMLElement;
    /** The page's examples and scenarios. */
    readonly library: Library;
    /** The catalogs translating them. */
    readonly catalogs: readonly Catalog[];
    /** The page's environment. */
    readonly environment: BrowserEnvironment;
    /** The globals it renders with. */
    readonly globals: Globals;
    /** The origin of the page and its host. */
    readonly origin: string;
}

/** Lay an example out along its axes, laying the cells out again as its values or axes change. */
function showMatrix(
    example: Example<object, JSX.Element>,
    name: string,
    given: JsonObject,
    axes: MatrixAxes,
    page: Page,
): Presentation {
    // draw every cell of the axes with the example's values, telling the host the cell picked
    const { element, catalogs, environment, globals, origin } = page;
    const values = valuesOf(example, given);
    const matrix = new Matrix(
        element,
        name,
        { example, globals, catalogs },
        values,
        axes,
        environment,
        {
            choose: (chosen) => ExampleEvent.send({ destack: "cell", values: chosen }, origin),
            open: (opened) => ExampleEvent.send({ destack: "open", values: opened }, origin),
        },
    );
    tellExample(example, values, origin);

    return {
        receive: (request) => {
            // take changed values or other axes
            if (request.destack === "set") {
                matrix.set(request.properties);
            } else if (request.destack === "axes") {
                matrix.arrange(request.axes);
            }
        },
        dispose: () => {
            matrix.dispose();
            element.remove();
        },
    };
}

/** Show an example alone, from the properties given, telling the host what it may change. */
function showExample(
    example: Example<object, JSX.Element>,
    given: JsonObject,
    page: Page,
): Presentation {
    // stage the example, starting from the properties given
    const stage = stageOf({ examples: [example], globals: page.globals }, page);
    if (Object.keys(given).length > 0) {
        stage.set(given, undefined);
    }
    tellExample(example, valuesOf(example, given), page.origin);

    return {
        receive: (request) => {
            if (request.destack === "set") {
                stage.set(request.properties, undefined);
            }
        },
        dispose: () => {
            stage.dispose();
            page.element.remove();
        },
    };
}

/** Show a scenario's examples, telling the host its steps, and act each one the host asks for. */
function showScenario(
    scenario: Scenario<unknown, unknown, Globals, JSX.Element>,
    page: Page,
): Presentation {
    // stage its examples and read its steps once, each with its checks
    const stage = stageOf(scenario.given, page);
    const entries = scenario.when.map((each) => Reference.resolve(each, scenario.parameters));
    const steps = entries.map((each) => ({
        step: schema.union([SetStep, Step]).parse(Operation.step(each)),
        checks: Operation.is(each) ? (each.then ?? []) : [],
    }));

    // tell the host its steps
    const description = scenario.description;
    ExampleEvent.send(
        {
            destack: "scenario",
            name: scenario.name,
            description,
            steps: steps.map(({ step }) => step),
        },
        page.origin,
    );

    // wait out a step's pauses by the page's clock
    const pause = async (milliseconds: number) =>
        new Promise<void>((resolve) => {
            page.environment.clock.setTimeout(resolve, milliseconds);
        });

    return {
        receive: (request) => {
            // act the step asked for, then tell the host what its checks saw
            if (request.destack === "act") {
                const { index } = request;
                void stage.play(steps, index, pause).then((results) => {
                    ExampleEvent.send({ destack: "step", index, results }, page.origin);
                });
            }
        },
        dispose: () => {
            stage.dispose();
            page.element.remove();
        },
    };
}

/** Stage examples on a page, exposing the stage to a test's driver. */
function stageOf(given: Given<JSX.Element, Globals>, page: Page): Stage {
    // render the examples with the page's index, and expose the stage
    const index = Object.entries(page.library).map(([name, { kind }]) => ({ name, kind }));
    const stage = Stage.render(page.element, given, page.environment, {
        catalogs: page.catalogs,
        index,
    });
    Reflect.set(globalThis, STAGE, stage);

    return stage;
}

/** Read an example's values: its component's defaults, its own properties, then those given. */
function valuesOf(example: Example<object, JSX.Element>, given: JsonObject): JsonObject {
    const defaults = Component.is(example.of) ? example.of.defaults : {};

    return schema.jsonObject().parse({ ...defaults, ...example.properties, ...given });
}

/** Tell the host an example is shown: its description, its inputs and their values. */
function tellExample(
    example: Example<object, JSX.Element>,
    values: JsonObject,
    origin: string,
): void {
    ExampleEvent.send(
        {
            destack: "example",
            name: example.name,
            description: example.description,
            inputs: inputsOf(example),
            values,
        },
        origin,
    );
}

/** Fetch every catalog the page's builds serve, as the page's context lists them below its base. */
async function fetchCatalogs(
    owner: PackageId,
    environment: BrowserEnvironment,
): Promise<Catalog[]> {
    // read the page's context, which lists the catalogs
    const element = document.getElementById(CONTEXT_ELEMENT);
    if (element === null) {
        throw new TypeError(`the page has no '${CONTEXT_ELEMENT}' context`);
    }
    const context = ViewContext.parse(JSON.parse(element.textContent));
    const root = `${new URL(environment.navigation.currentEntry.url).origin}${context.base}`;

    return Promise.all(
        context.catalogs.map(async (path) => {
            // fetch each at its build path, refusing one that does not load
            const response = await environment.network.fetch(new Request(`${root}/${path}`));
            if (!response.ok) {
                throw new TypeError(
                    `could not read the catalog at '${path}': status '${String(response.status)}'`,
                );
            }

            return Catalog.of(path, await response.json(), owner);
        }),
    );
}

/** Describe the inputs a host changes an example's component by, objects aside. */
function inputsOf(example: Example): ActionInputDescription[] {
    // describe each property of the component the example shows, none for anything else
    const definitions = Component.is(example.of) ? example.of.properties : {};

    return Object.entries(definitions).flatMap(([name, definition]) => {
        const input = definition === undefined ? undefined : ActionInput.of(name, definition);

        return input === undefined || input.value.kind === "object" ? [] : [input.describe()];
    });
}

/** A check of a step that observes the examples, and the value it expects. */
const ObservedCheck = schema.object({
    /** The observation. */
    observe: Observation,
    /** The value it expects. */
    is: schema.json(),
});

/** Let go of the focus that landed on an element of a look-only page. */
function release(event: FocusEvent): void {
    if (event.target instanceof HTMLElement) {
        event.target.blur();
    }
}
