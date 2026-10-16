import { Change, type Table } from "@destack/db";
import type { Controller } from "@destack/runtime/control";
import { connection } from "../object/index.ts";
import type { Tunnel } from "./tunnel.ts";

/** The controller ending a partition's tunnel connections as their tokens lapse. */
export class LapseController implements Controller {
    /** The controller's name in reports. */
    readonly name = "lapse";
    /** The connection rows, whose lapses it follows. */
    readonly watches: readonly Table[] = [connection.table];
    /** The partition's tunnel, whose connections it ends. */
    readonly #tunnel: Tunnel;

    /** End the connections of a partition's tunnel as they lapse. */
    constructor(tunnel: Tunnel) {
        this.#tunnel = tunnel;
    }

    /** Select the tunnel's machine for a changed connection row. */
    keys(change: Change): readonly string[] {
        return Change.of(change, connection.table) ? [this.#tunnel.machineId] : [];
    }

    /** List the tunnel's machine while it has a connection row. */
    async list(): Promise<readonly string[]> {
        return (await this.#tunnel.isConnected()) ? [this.#tunnel.machineId] : [];
    }

    /** End the lapsed connections, answering the wait until the next lapse. */
    reconcile(): Promise<number | undefined> {
        return this.#tunnel.lapse();
    }
}
