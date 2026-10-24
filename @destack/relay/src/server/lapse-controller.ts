import type { ReconcileController } from "@destack/db";
import type { Tunnel } from "./tunnel.ts";

/** The controller closing a partition's tunnel sockets as they lapse, on the host's alarm. */
export class LapseController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "lapse";
    /** The partition's tunnel, whose sockets it closes. */
    readonly #tunnel: Tunnel;
    /** Queue the partition's machine again, once the loop runs. */
    #enqueue: ((key: string) => void) | undefined;

    /** Close the sockets of a partition's tunnel as they lapse. */
    constructor(tunnel: Tunnel) {
        this.#tunnel = tunnel;
    }

    /** Take the loop's queue, which a socket's opening or moved lapse queues the machine in. */
    source(enqueue: (key: string) => void): void {
        this.#enqueue = enqueue;
    }

    /** Queue the partition's machine, as a socket opened or its lapse moved. */
    changed(): void {
        // leave a change before the loop starts, which its first listing of the machine covers
        this.#enqueue?.(this.#tunnel.machineId);
    }

    /** List the partition's one machine. */
    async list(): Promise<readonly string[]> {
        return [this.#tunnel.machineId];
    }

    /** Close the lapsed sockets, answering the wait until the next lapse, which the alarm runs. */
    reconcile(): Promise<number | undefined> {
        return this.#tunnel.closeLapsed();
    }
}
