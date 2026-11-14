import { TestComputer } from "@destack/machine/computer/test";
import type { Clock } from "@destack/environment";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import type { Given, SetStep } from "@destack/package/declare";
import { type JsonValue, schema } from "@destack/schema";
import type { Driver } from "@destack/test";
import { Stage } from "@destack/update/release";
import { Link } from "../link/index.ts";
import { DESKTOP_OBJECTS } from "../object/index.ts";
import type { DesktopObservation, DesktopStep } from "../scenario/interaction.ts";
import { desktopInteraction } from "../scenario/interaction.ts";
import { Desktop } from "../workload/desktop.ts";
import { openShell } from "./shell.ts";

/** A command the desktop sends its shell, as far as the driver reads it. */
const Command = schema.looseObject({ action: schema.string() });

/** The command the desktop sends to open a window. */
const WINDOW_OPEN = "window.open";

/** The outcome of a start whose desktop ended before its first window. */
const ENDED = "ended";

/** A running desktop: its shell's recording and the loop that runs it. */
interface Running {
    /** The shell's recording and controls. */
    readonly shell: ReturnType<typeof openShell>;
    /** The desktop's run until its shell closes. */
    readonly running: Promise<void>;
}

/** A driver that plays desktop scenarios on a person's laptop beside the owner's home. */
export class DesktopDriver implements Driver<DesktopStep, DesktopObservation> {
    /** The interaction the driver plays. */
    static readonly interaction = desktopInteraction;
    /** The settings its worlds run with: SQLite in WAL, synchronous NORMAL, as in production. */
    static readonly configuration = { durability: "normal" };

    /** The laptop the desktop runs on. */
    readonly #laptop: TestComputer;
    /** The desktop running now, none before a start or after a quit. */
    #running: Running | undefined;
    /** The windows the desktop opened so far. */
    #windows = 0;

    /** Take a laptop beside the owner's home. */
    constructor(laptop: TestComputer) {
        this.#laptop = laptop;
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#laptop.clock;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#laptop.world.simulation.telemetry;
    }

    /** Open the owner's home in the cloud and a laptop with the desktop's objects. */
    static async start(_given: Given): Promise<DesktopDriver> {
        // run the home in the cloud, and the laptop through the relay
        const { home } = await TestComputer.home();
        const relay = await TestComputer.relay(home.world);
        const laptop = await TestComputer.open({
            world: home.world,
            relay: relay.url,
            localObjects: DESKTOP_OBJECTS,
        });

        return new DesktopDriver(laptop);
    }

    /** Start the desktop until its first window opens, or quit it until it ends. */
    async act(step: SetStep | DesktopStep): Promise<void> {
        const { simulation } = this.#laptop.world;
        // refuse the set step, as a desktop world has no example properties
        if (step.action === "set") {
            throw new TypeError("a desktop world takes no example properties");
        }
        // start the desktop and wait for its first window
        else if (step.action === "start") {
            const shell = openShell();
            const desktop = await Desktop.open(
                this.#laptop.device,
                shell.shell,
                Link.of(new Stage("local")),
                this.#laptop.environment,
            );
            const opened = shell.next(isWindowOpen);
            const running = (async () => {
                await desktop.run(undefined);
                await desktop[Symbol.asyncDispose]();
            })();
            this.#running = { shell, running };
            const first = await simulation.wait(
                Promise.race([opened.then(() => WINDOW_OPEN), running.then(() => ENDED)]),
            );
            if (first === ENDED) {
                throw new TypeError("the desktop ended before it opened a window");
            }
        }
        // quit the running desktop by closing its shell's pipe
        else {
            const running = this.#running;
            if (running === undefined) {
                throw new TypeError("the desktop quits without having started");
            }
            this.#windows += running.shell.sent.filter(isWindowOpen).length;
            running.shell.close();
            await simulation.wait(running.running);
            this.#running = undefined;
        }
    }

    /** Count the windows the desktop opened, the running one's included. */
    observe(_observation: DesktopObservation): JsonValue {
        return this.#windows + (this.#running?.shell.sent.filter(isWindowOpen).length ?? 0);
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#laptop.world.simulation.fingerprint;
    }

    /** Close a desktop still running, as the world ends with the test. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.#running?.shell.close();
        await this.#running?.running;
    }
}

/** Report whether a command the desktop sent opens a window. */
function isWindowOpen(command: unknown): boolean {
    const parsed = Command.safeParse(command);

    return parsed.success && parsed.data.action === WINDOW_OPEN;
}
