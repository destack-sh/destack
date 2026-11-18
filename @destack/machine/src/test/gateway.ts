import { present } from "@destack/schema";
import type { Listener } from "@destack/environment";
import type { Simulation } from "@destack/environment/simulation";
import type { Machine } from "../machine/machine.ts";

/** The port a machine's HTTPS origin answers at. */
const HTTPS_PORT = 443;

/** The port a machine's HTTP origin answers at. */
const HTTP_PORT = 80;

/** The machines of a world answering at their origins on its network. */
export class TestGateway {
    /** The world whose network the machines answer on. */
    readonly #simulation: Simulation;
    /** The listener of each machine answering, by machine. */
    readonly #listening = new Map<string, Listener>();

    /** Answer for the machines of a world. */
    constructor(simulation: Simulation) {
        this.#simulation = simulation;
    }

    /** Answer a machine's origin on the world's network, once per machine. */
    serve(machine: Machine): void {
        // listen at the machine's origin on its node, handing each request to the machine and keeping its sockets
        if (this.#listening.has(machine.id)) {
            return;
        }
        const origin = new URL(machine.origin);
        const host = present(machine.hibernation, "the machine's sockets");
        const listener = this.#simulation.network.server(machine.id).http({
            hostname: origin.hostname,
            port: Number(origin.port || (origin.protocol === "https:" ? HTTPS_PORT : HTTP_PORT)),
            ...host.listen((request) => machine.serve(request)),
        });
        this.#listening.set(machine.id, listener);
    }

    /** Stop answering for every machine. */
    async [Symbol.asyncDispose](): Promise<void> {
        await Promise.all([...this.#listening.values()].map((listener) => listener.stop(true)));
        this.#listening.clear();
    }
}
