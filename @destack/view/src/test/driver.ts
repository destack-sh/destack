import { testBrowser, testWorld } from "./browser.ts";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import type { Driver } from "@destack/test";
import type { Clock } from "@destack/environment";
import type { Given, SetStep } from "@destack/package/declare";
import type { Globals } from "../scenario/interaction.ts";
import type { JsonValue } from "@destack/schema";
import type { JSX } from "@solidjs/web";
import { Stage } from "../example/stage.ts";
import { type Locator, type Observation, type Step, viewInteraction } from "../scenario/index.ts";
import { stubPopovers } from "./dom.ts";

/** A driver that plays UI scenarios in the test DOM in process. */
export class ViewDriver implements Driver<Step, Observation> {
    /** The interaction the driver speaks. */
    static readonly interaction = viewInteraction;
    /** The settings of the worlds: none change a view's numbers. */
    static readonly configuration = {};

    /** The element holding the rendered examples. */
    readonly container: HTMLElement;
    /** The clock the test browser runs on. */
    readonly clock: Clock;
    /** The stage the scenario plays on. */
    readonly #stage: Stage;
    /** The unmounting of the examples and the container, last mounted first. */
    readonly #mounted: DisposableStack;

    /** Create a driver over the examples rendered into a container. */
    constructor(container: HTMLElement, stage: Stage, mounted: DisposableStack) {
        // set the driver's fields
        this.container = container;
        this.clock = testBrowser().clock;
        this.#stage = stage;
        this.#mounted = mounted;
    }

    /** Render a scenario's examples into the document until the driver is disposed. */
    static start(given: Given<JSX.Element, Globals>): ViewDriver {
        // mount into the document until the driver is disposed
        stubPopovers();
        const mounted = new DisposableStack();
        const container = document.createElement("div");
        document.body.append(container);
        mounted.defer(() => container.remove());

        // render the examples over properties a set step changes
        const stage = Stage.render(container, given, testBrowser());
        mounted.defer(() => stage.dispose());

        return new ViewDriver(container, stage, mounted);
    }

    /** Unmount the examples and remove their container. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.#mounted.dispose();
    }

    /** Take one step, letting the rendered examples follow. */
    async act(step: SetStep | Step): Promise<void> {
        await this.#stage.act(step, async (milliseconds) =>
            testWorld().simulation.sleep(milliseconds),
        );
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return testWorld().simulation.fingerprint;
    }

    /** The telemetry the test's world records. */
    get telemetry(): SimulatedTelemetry {
        return testWorld().simulation.telemetry;
    }

    /** Read one observation. */
    observe(observation: Observation): JsonValue {
        return this.#stage.observe(observation);
    }

    /** Find the one element a locator matches. */
    locate(locator: Locator): HTMLElement {
        return this.#stage.locate(locator);
    }
}
