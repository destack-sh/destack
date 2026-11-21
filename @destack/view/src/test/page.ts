import { relative } from "node:path";
import type { Clock } from "@destack/environment";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import type { Given, SetStep } from "@destack/package/declare";
import { type JsonValue, present, schema } from "@destack/schema";
import { type Driver, expect, inject } from "@destack/test";
import type { JSX } from "@solidjs/web";
import { type Browser, type CDPSession, chromium, type Page } from "playwright";
import { STAGE } from "../example/stage.ts";
import {
    type Globals,
    type Locator,
    type Observation,
    type Step,
    viewInteraction,
} from "../scenario/index.ts";
import { testBrowser, testWorld } from "./browser.ts";

/** The viewport of a scenario's page. */
const VIEWPORT = { width: 1280, height: 800 };

/** The milliseconds a page may take to load. */
const LOAD_TIME = 60_000;

/** The moves a pointer makes on its way to a point. */
const MOVE_STEPS = 4;

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
    /** The session that sends touches. */
    readonly #touch: CDPSession | undefined;
    /** The errors the page threw. */
    readonly #errors: string[] = [];
    /** Where the pointer is. */
    #point = { x: 0, y: 0 };
    /** Whether the finger is down. */
    #isTouching = false;

    /** Create a driver of a page playing a scenario. */
    private constructor(page: Page, browser: Browser, touch: CDPSession | undefined) {
        // set the page and collect the errors it throws
        this.clock = testBrowser().clock;
        this.#page = page;
        this.#browser = browser;
        this.#touch = touch;
        page.on("pageerror", (error) => this.#errors.push(error.message));
    }

    /** Open a page that renders the examples of the running scenario. */
    static async start(given: Given<JSX.Element, Globals>): Promise<BrowserDriver> {
        // open a page in the run's Chromium
        const { endpoint, origin, root } = inject("chromium");
        const browser = await chromium.connect(endpoint);
        const isTouch = given.globals?.pointer === "touch";
        const context = await browser.newContext({ viewport: VIEWPORT, hasTouch: isTouch });
        const page = await context.newPage();
        const touch = isTouch ? await context.newCDPSession(page) : undefined;

        // render the scenario of the running test
        const state = expect.getState();
        const path = present(state.testPath, "the running test's file");
        const title = present(state.currentTestName, "the running test's name");
        const name = present(title.split(" > ").at(-1), "the running scenario's name");
        const query = new URLSearchParams({ scenario: `/${relative(root, path)}`, name });
        const now = testBrowser().clock.now();
        await page.clock.install({ time: now });
        await page.goto(`${origin}/?${query.toString()}`);
        await page.waitForFunction((exposed) => exposed in globalThis, STAGE);

        // stop the page's time until the scenario pauses
        await page.clock.pauseAt(now + LOAD_TIME);

        // refuse a page that loads again during the scenario
        const driver = new BrowserDriver(page, browser, touch);
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
                await this.#pointerUp(step.button);
                break;
            case "pointerMove":
                await this.#pointerMove(step.target, step.position);
                break;
            case "pointerDown":
                await this.#pointerDown(step.button);
                break;
            case "pointerUp":
                await this.#pointerUp(undefined);
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
        // read the point on the element
        const from = this.#point;
        const to = Point.parse(await this.#call("point", target, position));
        this.#point = to;

        // move the mouse or the finger there
        if (this.#touch === undefined) {
            await this.#page.mouse.move(to.x, to.y, { steps: MOVE_STEPS });
        } else if (this.#isTouching) {
            for (let step = 1; step <= MOVE_STEPS; step += 1) {
                const share = step / MOVE_STEPS;
                const x = from.x + (to.x - from.x) * share;
                const y = from.y + (to.y - from.y) * share;
                await this.#touch.send("Input.dispatchTouchEvent", {
                    type: "touchMove",
                    touchPoints: [{ x, y, id: 1 }],
                });
            }
        }
    }

    /** Press a button of the mouse, or land the finger. */
    async #pointerDown(button: "primary" | "secondary" | undefined): Promise<void> {
        if (this.#touch === undefined) {
            await this.#page.mouse.down({ button: button === "secondary" ? "right" : "left" });
        } else {
            this.#isTouching = true;
            await this.#touch.send("Input.dispatchTouchEvent", {
                type: "touchStart",
                touchPoints: [{ ...this.#point, id: 1 }],
            });
        }
    }

    /** Release the button of the mouse, or lift the finger. */
    async #pointerUp(button: "primary" | "secondary" | undefined): Promise<void> {
        if (this.#touch === undefined) {
            await this.#page.mouse.up({ button: button === "secondary" ? "right" : "left" });
        } else {
            this.#isTouching = false;
            await this.#touch.send("Input.dispatchTouchEvent", {
                type: "touchEnd",
                touchPoints: [],
            });
        }
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
