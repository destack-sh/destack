import { relative } from "node:path";
import type { Clock } from "@destack/environment";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import { OriginPath } from "@destack/package";
import type { Given, SetStep } from "@destack/package/declare";
import { type JsonValue, present, schema } from "@destack/schema";
import { type Driver, expect, inject } from "@destack/test";
import type { JSX } from "@solidjs/web";
import { type Browser, chromium, type Page } from "playwright";
import { STAGE } from "../example/stage.ts";
import {
    type Globals,
    type Locator,
    type Observation,
    type Step,
    viewInteraction,
} from "../scenario/index.ts";
import { testBrowser, testWorld } from "./browser.ts";
import {
    type BrowserPointer,
    openPointer,
    type PointerButton,
    type ViewportPoint,
} from "./pointer.ts";

/** The viewport of a scenario's page. */
const VIEWPORT = { width: 1280, height: 800 };

/** The milliseconds a page may take to load. */
const LOAD_TIME = 60_000;

/** The milliseconds a wheel that scrolls nothing is waited for, in real time. */
const WHEEL_TIME = 500;

/** The values a run's setup provides to its tests. */
declare module "vitest" {
    /** The values a run provides to its tests. */
    export interface ProvidedContext {
        /** The Chromium and the examples server of a run. */
        chromium: {
            /** The endpoint of the run's Chromium. */
            readonly endpoint: string;
            /** The origin of the package's examples server. */
            readonly origin: string;
            /** The package's directory. */
            readonly root: string;
        };
    }
}

/** A driver that plays UI scenarios in Chromium. */
export class BrowserDriver implements Driver<Step, Observation> {
    /** The interaction the driver speaks. */
    static readonly interaction = viewInteraction;
    /** The settings of the worlds: none change a view's numbers. */
    static readonly configuration = {};

    /** The clock of the test's world. */
    readonly clock: Clock;
    /** The page the examples render in. */
    readonly #page: Page;
    /** The driver's connection to the run's Chromium. */
    readonly #browser: Browser;
    /** The mouse, finger or pen the scenario points with. */
    readonly #pointer: BrowserPointer;
    /** The errors the page threw. */
    readonly #errors: string[] = [];
    /** Where the pointer is. */
    #point: ViewportPoint = { x: 0, y: 0 };

    /** Create a driver of a page playing a scenario. */
    private constructor(page: Page, browser: Browser, pointer: BrowserPointer) {
        // set the page and collect the errors it throws
        this.clock = testBrowser().clock;
        this.#page = page;
        this.#browser = browser;
        this.#pointer = pointer;
        page.on("pageerror", (error) => this.#errors.push(error.message));
    }

    /** Open a page that renders the examples of the running scenario. */
    static async start(given: Given<JSX.Element, Globals>): Promise<BrowserDriver> {
        // open a page in the run's Chromium
        const { endpoint, origin, root } = inject("chromium");
        const browser = await chromium.connect(endpoint);
        const kind = given.globals?.pointer ?? "mouse";
        const context = await browser.newContext({
            viewport: VIEWPORT,
            hasTouch: kind === "touch",
        });
        const page = await context.newPage();
        const pointer = await openPointer(kind, context, page);

        // render the scenario of the running test
        const state = expect.getState();
        const path = present(state.testPath, "the running test's file");
        const title = present(state.currentTestName, "the running test's name");
        const name = present(title.split(" > ").at(-1), "the running scenario's name");
        const query = new URLSearchParams({ scenario: `/${relative(root, path)}`, name });
        const now = testBrowser().clock.now();
        await page.clock.install({ time: now });
        await page.goto(`${origin}${OriginPath.examples}?${query.toString()}`);
        await page.waitForFunction((exposed) => exposed in globalThis, STAGE);

        // stop the page's time until the scenario pauses
        await page.clock.pauseAt(now + LOAD_TIME);

        // refuse a page that loads again during the scenario
        const driver = new BrowserDriver(page, browser, pointer);
        await driver.#settle();
        page.on("framenavigated", (frame) => {
            if (frame === page.mainFrame()) {
                driver.#errors.push("it loaded again while the scenario played");
            }
        });

        return driver;
    }

    /** Close the page and its connection to the run's Chromium. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#browser.close();
    }

    /** Take one step with real input. */
    async act(step: SetStep | Step): Promise<void> {
        // send the step's input to the page
        const { keyboard } = this.#page;
        switch (step.action) {
            case "focus":
                await this.#call("focus", step.target, false);
                break;
            case "click":
                await this.#pointerMove(step.target, step.position);
                await this.#pointerDown(step.button);
                await this.#pointerUp();
                break;
            case "pointerMove":
                await this.#pointerMove(step.target, step.position);
                break;
            case "pointerDown":
                await this.#pointerDown(step.button);
                break;
            case "pointerUp":
                await this.#pointerUp();
                break;
            case "wheel":
                await this.#wheel(step.deltaX ?? 0, step.deltaY ?? 0);
                break;
            case "pause":
                await this.#page.clock.runFor(step.milliseconds);
                break;
            case "press":
                if (step.target !== undefined) {
                    await this.#call("focus", step.target, false);
                }
                await keyboard.press(
                    step.key.endsWith(" ") ? `${step.key.slice(0, -1)}Space` : step.key,
                );
                break;
            case "fill":
                await this.#call("focus", step.target, true);
                await (step.value === ""
                    ? keyboard.press("Delete")
                    : keyboard.insertText(step.value));
                break;
            case "set":
                await this.#call("set", step.properties, step.example);
                break;
        }

        // let the page settle and fail on its errors
        await this.#settle();
        if (this.#errors.length > 0) {
            throw new TypeError(`the page threw '${this.#errors.join("', '")}'`);
        }
    }

    /** Read one observation from the page. */
    async observe(observation: Observation): Promise<JsonValue> {
        return await this.#call("observe", observation);
    }

    /** Read the fingerprint of the test's world. */
    fingerprint(): number {
        return testWorld().simulation.fingerprint;
    }

    /** The telemetry of the test's world. */
    get telemetry(): SimulatedTelemetry {
        return testWorld().simulation.telemetry;
    }

    /** Turn the wheel where the pointer is and wait for the scroll it starts to end. */
    async #wheel(deltaX: number, deltaY: number): Promise<void> {
        // watch for the end of a scroll, which the browser applies after the wheel returns
        const scrolled = this.#page.evaluate(
            async () =>
                new Promise<void>((resolve) => {
                    const options = { capture: true, once: true };
                    document.addEventListener("scrollend", () => resolve(), options);
                }),
        );
        await this.#page.mouse.wheel(deltaX, deltaY);

        // go on once it ended, or once a wheel that scrolls nothing had its time
        await Promise.race([scrolled, this.#page.waitForTimeout(WHEEL_TIME)]);
    }

    /** Wait for the page's transitions and animations to end. */
    async #settle(): Promise<void> {
        await this.#page.evaluate(async () => {
            // wait for every animation that ends
            const ending = document
                .getAnimations()
                .filter((each) => Number.isFinite(each.effect?.getComputedTiming().endTime));
            await Promise.allSettled(ending.map(async (each) => each.finished));
        });
    }

    /** Move the pointer onto an element in steps. */
    async #pointerMove(
        target: Locator,
        position: { readonly x: number; readonly y: number } | undefined,
    ): Promise<void> {
        // read the point on the element and move there
        const from = this.#point;
        this.#point = Point.parse(await this.#call("point", target, position));
        await this.#pointer.move(from, this.#point);
    }

    /** Press a button of the pointer where it is. */
    async #pointerDown(button: PointerButton = "primary"): Promise<void> {
        await this.#pointer.down(this.#point, button);
    }

    /** Release the button the pointer pressed, where it is. */
    async #pointerUp(): Promise<void> {
        await this.#pointer.up(this.#point);
    }

    /** Call a verb of the page's stage. */
    async #call(verb: string, ...values: unknown[]): Promise<JsonValue> {
        const result: unknown = await this.#page.evaluate(
            ([exposed, called, given]) => {
                // find the verb on the stage the page exposes
                const stage: unknown = Reflect.get(globalThis, exposed);
                const found: unknown =
                    typeof stage === "object" && stage !== null
                        ? Reflect.get(stage, called)
                        : undefined;
                if (typeof found !== "function") {
                    throw new TypeError(`the page has no stage with a verb '${called}'`);
                }
                const answered: unknown = Reflect.apply(found, stage, given);

                return answered ?? null;
            },
            [STAGE, verb, values] as const,
        );

        return schema.json().parse(result);
    }
}

/** A point in the page's viewport. */
const Point = schema.object({
    /** The horizontal position. */
    x: schema.number(),
    /** The vertical position. */
    y: schema.number(),
});
