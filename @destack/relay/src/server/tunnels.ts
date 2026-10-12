import type { Domains } from "@destack/account/address";
import { and, asc, Change, eq, inArray, type Table } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { Identifier, schema } from "@destack/schema";
import type { TokenVerifier } from "@destack/service/authentication";
import type { Controller } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import type { HeldSocket, Hibernation, SocketHandler } from "@destack/service/workload";
import { connection, tunnel } from "../object/index.ts";
import { PING_MESSAGE, PONG_MESSAGE, TUNNEL_PROTOCOL } from "../session/index.ts";
import { type Admission, Tunnel } from "./tunnel.ts";

/** What a held tunnel socket keeps beside it: the machine it connects. */
const SocketAttachment = schema.object({
    /** The machine the socket connects. */
    machineId: schema.identifier("machine"),
});

/** What the tunnels an instance holds run with. */
export interface TunnelsOptions {
    /** The store of the tunnels' connection rows, in the partition the instance serves. */
    readonly store: ObjectStore;
    /** The WebSockets the instance's host keeps open for it. */
    readonly hibernation: Hibernation;
    /** The relay's tunnel URL machines dial, which renames come from. */
    readonly url: string;
    /** Verify the universe's tokens machines renew their tunnels with. */
    readonly tokens: Pick<TokenVerifier, "verify">;
    /** The domains names resolve under. */
    readonly domains: Domains;
    /** Withdraw at the relay's root the endpoint of a machine whose last connection closed. */
    readonly withdraw: (machineId: Identifier<"machine">) => Promise<void>;
    /** Report a failure no request answers. */
    readonly report: (error: unknown) => void;
}

/** The tunnels of a relay instance. */
export class Tunnels implements SocketHandler {
    /** What the tunnels run with. */
    readonly #options: TunnelsOptions;
    /** The tunnels of the machines with an open connection here, by machine. */
    readonly #tunnels = new Map<string, Tunnel>();

    /** Hold tunnels with a store, a socket host and the relay's tokens. */
    constructor(options: TunnelsOptions) {
        this.#options = options;
    }

    /** Accept an admitted machine's WebSocket in its tunnel and record its connection. */
    async accept(admission: Admission): Promise<Response> {
        // record the connection under a new identifier, its held socket's, with its lapse
        const id = Identifier.create("connection");
        const now = this.#options.store.clock.now();
        await this.#options.store.database.insert(connection.table).values({
            id,
            scope: Tunnel.id(admission.machineId),
            lapsesAt: admission.lapsesAt,
            createdAt: now,
            updatedAt: now,
        });

        // carry a session over the socket the host is about to hold
        const attachment = { machineId: admission.machineId };
        const socket = this.#options.hibernation.socket({ id, attachment });
        this.#tunnel(admission.machineId, admission.name).attach(id, socket);

        return this.#options.hibernation.accept({
            id,
            attachment,
            protocol: TUNNEL_PROTOCOL,
            respond: { request: PING_MESSAGE, response: PONG_MESSAGE },
        });
    }

    /** Take a held socket's message into its connection's session. */
    async message(socket: HeldSocket, message: Uint8Array<ArrayBuffer>): Promise<void> {
        const held = await this.#resume(socket);
        held?.receive(socket.id, message);
    }

    /** Forget a held socket's connection once it closed. */
    async close(socket: HeldSocket): Promise<void> {
        // forget the connection and its row
        const { machineId } = SocketAttachment.parse(socket.attachment);
        const held = this.#tunnels.get(machineId);
        held?.detach(socket.id);
        await this.#forget([socket.id]);

        // withdraw the endpoint of a machine left without a connection
        if (held === undefined || held.isEmpty) {
            this.#tunnels.delete(machineId);
            await this.#options.withdraw(machineId);
        }
    }

    /** Forward a request through a machine's tunnel. */
    async fetch(machineId: Identifier<"machine">, request: Request): Promise<Response> {
        const held = await this.#require(machineId);

        return held.fetch(request);
    }

    /** Tell a connected machine the name the relay routes to it now, keeping it for renewals. */
    async rename(machineId: Identifier<"machine">, name: string): Promise<void> {
        // keep the name on the machine's tunnel row
        const rows = await this.#connections(machineId);
        await this.#options.store.database
            .update(tunnel.table)
            .set({ name, updatedAt: this.#options.store.clock.now() })
            .where(eq(tunnel.table.id, Tunnel.id(machineId)));

        // tell a connected machine
        if (rows.length > 0) {
            const held = await this.#require(machineId);
            await held.rename(name);
        }
    }

    /** End every connection of a machine, as its revocation does. */
    async end(machineId: Identifier<"machine">): Promise<boolean> {
        // end the open sessions and forget the tunnel
        const rows = await this.#connections(machineId);
        this.#tunnels.get(machineId)?.close();
        this.#tunnels.delete(machineId);

        // forget the connection rows
        await this.#forget(rows.map((row) => row.id));

        return rows.length > 0;
    }

    /** The controller ending connections as their tokens lapse, on the partition's alarm. */
    get controller(): Controller {
        return new LapseController(this);
    }

    /** End a machine's connections whose tokens lapsed, answering the wait until its next lapse. */
    async lapse(machineId: Identifier<"machine">): Promise<number | undefined> {
        // end each lapsed connection, telling the machine
        const now = this.#options.store.clock.now();
        const rows = await this.#connections(machineId);
        const lapsed = rows.filter((row) => row.lapsesAt <= now);
        const held = this.#tunnels.get(machineId);
        for (const row of lapsed) {
            held?.end(row.id);
        }
        await this.#forget(lapsed.map((row) => row.id));

        // wake at the next lapse
        const next = rows.filter((row) => row.lapsesAt > now).map((row) => row.lapsesAt);

        return next.length === 0 ? undefined : Math.min(...next) - now;
    }

    /** List the machines with a connection recorded here. */
    async machines(): Promise<readonly string[]> {
        const rows = await this.#options.store.database
            .selectDistinct({ tunnelId: connection.table.scope })
            .from(connection.table);

        return rows.map((row) => Tunnel.machine(row.tunnelId));
    }

    /** Find a machine's tunnel, resuming its connections after the instance slept. */
    async #require(machineId: Identifier<"machine">): Promise<Tunnel> {
        // take the tunnel in memory
        const kept = this.#tunnels.get(machineId);
        if (kept !== undefined) {
            return kept;
        }

        // resume the connections the host held while the instance slept
        const held = await this.#options.hibernation.list();
        for (const socket of held) {
            await this.#resume(socket);
        }
        const resumed = this.#tunnels.get(machineId);
        if (resumed === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `machine ${machineId} is not connected`,
            });
        }

        return resumed;
    }

    /** Run a session again over a held socket whose connection row this partition keeps. */
    async #resume(socket: HeldSocket): Promise<Tunnel | undefined> {
        // take a connection already carried
        const { machineId } = SocketAttachment.parse(socket.attachment);
        const kept = this.#tunnels.get(machineId);
        if (kept?.has(socket.id) === true) {
            return kept;
        }

        // close a socket without its connection row
        const tunnelId = Tunnel.id(machineId);
        const [row] = await this.#options.store.database
            .select({ name: tunnel.table.name })
            .from(connection.table)
            .innerJoin(tunnel.table, eq(tunnel.table.id, connection.table.scope))
            .where(
                and(
                    eq(connection.table.scope, tunnelId),
                    eq(connection.table.id, connection.identifier(socket.id)),
                ),
            );
        if (row === undefined) {
            socket.close();

            return undefined;
        }

        // carry a session over it under the name the tunnel row keeps
        const resumed = this.#tunnel(machineId, row.name);
        resumed.attach(socket.id, socket);

        return resumed;
    }

    /** Find or start a machine's tunnel under a name. */
    #tunnel(machineId: Identifier<"machine">, name: string): Tunnel {
        let held = this.#tunnels.get(machineId);
        if (held === undefined) {
            held = new Tunnel(machineId, name, {
                clock: this.#options.store.clock,
                url: this.#options.url,
                tokens: this.#options.tokens,
                domains: this.#options.domains,
                report: this.#options.report,
                renewed: async (id, lapsesAt) => {
                    await this.#options.store.database
                        .update(connection.table)
                        .set({ lapsesAt, updatedAt: this.#options.store.clock.now() })
                        .where(eq(connection.table.id, connection.identifier(id)));
                },
            });
            this.#tunnels.set(machineId, held);
        }

        return held;
    }

    /** Read a machine's connection rows, oldest lapse first. */
    async #connections(
        machineId: Identifier<"machine">,
    ): Promise<readonly { readonly id: string; readonly lapsesAt: number }[]> {
        return await this.#options.store.database
            .select({
                id: connection.table.id,
                lapsesAt: connection.table.lapsesAt,
            })
            .from(connection.table)
            .where(eq(connection.table.scope, Tunnel.id(machineId)))
            .orderBy(asc(connection.table.lapsesAt));
    }

    /** Forget connection rows by identifier. */
    async #forget(ids: readonly string[]): Promise<void> {
        if (ids.length === 0) {
            return;
        }
        await this.#options.store.database.delete(connection.table).where(
            inArray(
                connection.table.id,
                ids.map((id) => connection.identifier(id)),
            ),
        );
    }
}

/** The controller ending tunnel connections as their tokens lapse, keyed by machine. */
class LapseController implements Controller {
    /** The controller's name in reports. */
    readonly name = "lapse";
    /** The connection rows, whose lapses it follows. */
    readonly watches: readonly Table[] = [connection.table];
    /** The tunnels whose connections it ends. */
    readonly #tunnels: Tunnels;

    /** End the connections of a relay instance's tunnels as they lapse. */
    constructor(tunnels: Tunnels) {
        this.#tunnels = tunnels;
    }

    /** Select the machine of a changed connection row. */
    keys(change: Change): readonly string[] {
        return Change.of(change, connection.table)
            ? [Tunnel.machine(Change.image(change).scope)]
            : [];
    }

    /** List the machines with a connection recorded here. */
    list(): Promise<readonly string[]> {
        return this.#tunnels.machines();
    }

    /** End a machine's lapsed connections, answering the wait until its next lapse. */
    reconcile(machineId: string): Promise<number | undefined> {
        return this.#tunnels.lapse(schema.identifier("machine").parse(machineId));
    }
}
