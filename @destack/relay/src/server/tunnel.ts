import { telemetry } from "@destack/telemetry";
import relayPackage from "../package.ts";
import { principal } from "@destack/access";
import { asc, eq, type Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { RequestId } from "@destack/runtime/request";
import type { Hibernation, Socket, SocketHandler } from "@destack/runtime/workload";
import { type Call, Scope } from "@destack/sync";
import { connection, tunnel } from "../object/index.ts";
import { PING_MESSAGE, PONG_MESSAGE, TUNNEL_PROTOCOL } from "../session/index.ts";
import { type Domains, MachineAddress } from "@destack/account/address";
import { Identifier, schema } from "@destack/schema";
import { Bearer, TokenVerifier } from "@destack/runtime/authentication";
import { Identity } from "@destack/account/object";
import { createLocalJWKSet } from "jose";
import { CallError } from "@destack/runtime/error";
import { refusal } from "@destack/runtime/server";
import { MAX_STREAMS, type Renewal, Session, type Stream } from "../session/index.ts";

/** The instruments capturing the renewals that fail outside any request. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The path below the relay package's mount for the router's opens and forwards per machine. */
export const TUNNELS_PATH = relayPackage.adapters.tunnels.path;

/** The header with a machine's accepted opening handshake. */
export const HANDSHAKE_HEADER = "Destack-Handshake";

/** The path machines open and renew their tunnels at. */
export const TUNNEL_PATH = "/tunnel";

/** The lifetime of tunnel tokens: ten minutes, Kubernetes' shortest bound service account token. */
export const RELAY_TOKEN_LIFETIME_MILLISECONDS = 10 * 60_000;

/** How long a machine may take to answer a request's head: 100 s, Cloudflare's read timeout. */
const ANSWER_TIMEOUT_MILLISECONDS = 100_000;

/** A machine's accepted opening handshake (RFC 6455 section 4): its token's lapse and name. */
export const Handshake = schema.object({
    /** The machine. */
    machineId: schema.identifier("machine"),
    /** When its token lapses, in UTC epoch milliseconds. */
    lapsesAt: schema.number(),
    /** The name the relay routes to the machine, such as `laptop--florian.destack.computer`. */
    name: schema.string().min(1),
});
/** A machine's accepted opening handshake. */
export type Handshake = schema.Infer<typeof Handshake>;

/** The attachment of a hibernated tunnel socket: the machine it connects. */
const SocketAttachment = schema.object({
    /** The machine the socket connects. */
    machineId: schema.identifier("machine"),
});

/** One connection of a machine's tunnel: a hibernated socket's session until its token lapses. */
interface Connection {
    /** The hibernated socket's identifier, the connection row's key. */
    readonly id: string;
    /** The session over the connection. */
    readonly session: Session;
}

/** What a machine's tunnel runs with in its partition. */
export interface TunnelOptions {
    /** The partition's store, with the tunnel's connection rows. */
    readonly store: Pick<ObjectStore, "database" | "clock" | "mutate" | "self">;
    /** The host of the partition's hibernated sockets. */
    readonly hibernation: Pick<Hibernation, "list" | "socket" | "accept">;
    /** The relay's tunnel URL the machine dials, which its renames come from. */
    readonly url: string;
    /** Verify the universe's tokens machines renew their tunnels with. */
    readonly tokens: Pick<TokenVerifier, "verify">;
    /** The domains names resolve under. */
    readonly domains: Domains;
    /** How long the machine may take to answer a request's head, 100 s by default. */
    readonly answerTimeout?: number;
}

/** A machine's tunnel to the relay in its partition: its connections and their rows. */
export class Tunnel implements SocketHandler {
    /** The machine the tunnel leads to. */
    readonly machineId: Identifier<"machine">;
    /** What the tunnel runs with. */
    readonly #options: TunnelOptions;
    /** The open connections, oldest first. */
    readonly #connections: Connection[] = [];
    /** The name the relay routes to the machine, read from the tunnel's row once needed. */
    #name: string | undefined;
    /** The last write of the tunnel's rows, which the next write follows. */
    #writing: Promise<unknown> = Promise.resolve();

    /** Run a machine's tunnel in its partition. */
    constructor(machineId: Identifier<"machine">, options: TunnelOptions) {
        this.machineId = machineId;
        this.#options = options;
    }

    /** Derive the identifier of a machine's tunnel, the partition of its connections. */
    static id(machineId: Identifier<"machine">): Identifier<"tunnel"> {
        return schema.identifier("tunnel").parse(`tunnel-${Identifier.uuid(machineId)}`);
    }

    /** Read the machine a tunnel leads to from the tunnel's identifier. */
    static machine(tunnelId: string): Identifier<"machine"> {
        const id = schema.identifier("tunnel").parse(tunnelId);

        return schema.identifier("machine").parse(`machine-${Identifier.uuid(id)}`);
    }

    /** Build the verifier of machines' tunnel tokens over a replica of the universe's keys. */
    static verifier(issuer: string, replica: Pick<Snapshot, "read">): TokenVerifier {
        return new TokenVerifier({
            authority: { kind: "universe" },
            issuer,
            audience: relayPackage.id,
            lifetime: RELAY_TOKEN_LIFETIME_MILLISECONDS,
            // read the current keys at each verify, following their rotations
            keys: async (header, token) => {
                const keys = await Identity.keys(replica, Scope.universe.id);

                return createLocalJWKSet(keys)(header, token);
            },
        });
    }

    /** Verify a machine's token, answering the machine and its token's lapse. */
    static async verify(
        tokens: Pick<TokenVerifier, "verify">,
        token: string,
        now: number,
    ): Promise<{ readonly machineId: Identifier<"machine">; readonly lapsesAt: number }> {
        // require a machine's token
        const { caller, credential } = await tokens.verify(token, now);
        const { subject } = caller;
        if (!principal.machine.is(subject)) {
            throw new CallError("FORBIDDEN", { message: "only machines open tunnels" });
        }

        return {
            machineId: schema.identifier("machine").parse(subject.id),
            lapsesAt: credential.expiresAt,
        };
    }

    /** Whether no connection of the machine is open here. */
    get isEmpty(): boolean {
        return this.#connections.length === 0;
    }

    /** Accept the machine's WebSocket after its handshake and record its connection. */
    async accept(handshake: Handshake): Promise<Response> {
        // record the connection, its identifier its hibernated socket's
        const [created] = await this.#serially(() =>
            this.#mutate([
                connection.call("create", {
                    ...connection.in(Tunnel.id(this.machineId)),
                    lapsesAt: handshake.lapsesAt,
                }),
            ]),
        );
        const { id } = schema.looseObject({ id: schema.identifier("connection") }).parse(created);

        // run a session over the socket the host accepts next
        this.#name = handshake.name;
        const attachment = { machineId: this.machineId };
        this.#attach(id, this.#options.hibernation.socket({ id, attachment }));

        return this.#options.hibernation.accept({
            id,
            attachment,
            protocol: TUNNEL_PROTOCOL,
            respond: { request: PING_MESSAGE, response: PONG_MESSAGE },
        });
    }

    /** Take a hibernated socket's message into its connection's session. */
    async message(socket: Socket, message: Uint8Array<ArrayBuffer>): Promise<void> {
        if (await this.#resume(socket)) {
            this.#connections.find((each) => each.id === socket.id)?.session.receive(message);
        }
    }

    /** Forget a hibernated socket's connection and its row once it closed. */
    async close(socket: Socket): Promise<void> {
        this.#remove(socket.id)?.session.terminate();
        await this.#serially(async () => {
            const rows = await this.#rows();
            const closed = rows.filter((row) => row.id === socket.id).map((row) => row.id);
            await this.#mutate(Tunnel.#deletions(this.machineId, closed));
        });
    }

    /** End every connection and remove their rows, answering whether any was open. */
    async end(): Promise<boolean> {
        return this.#serially(async () => {
            // end the connections, and remove every row
            for (const ended of this.#connections.splice(0)) {
                ended.session.close();
            }
            const rows = await this.#rows();
            await this.#mutate(
                Tunnel.#deletions(
                    this.machineId,
                    rows.map((row) => row.id),
                ),
            );

            return rows.length > 0;
        });
    }

    /** End the connections whose tokens lapsed, answering the wait until the next lapse. */
    async lapse(): Promise<number | undefined> {
        return this.#serially(async () => {
            // end each lapsed connection, telling the machine, and remove its row
            const now = this.#options.store.clock.now();
            const rows = await this.#rows();
            const lapsed = rows.filter((row) => row.lapsesAt <= now);
            for (const row of lapsed) {
                this.#remove(row.id)?.session.close();
            }
            await this.#mutate(
                Tunnel.#deletions(
                    this.machineId,
                    lapsed.map((row) => row.id),
                ),
            );

            // wake at the next lapse, the rows ordered by lapse
            const next = rows.find((row) => row.lapsesAt > now);

            return next === undefined ? undefined : next.lapsesAt - now;
        });
    }

    /** Read whether the tunnel has a connection row. */
    async isConnected(): Promise<boolean> {
        return (await this.#rows()).length > 0;
    }

    /** Forward a request through the newest connection, resuming the connections after a sleep. */
    async fetch(request: Request): Promise<Response> {
        // resume the hibernated sockets, and refuse a machine-domain name other than the machine's own
        await this.#resumeAll();
        const hostname = new URL(request.url).hostname;
        const address = MachineAddress.parse(hostname, this.#options.domains.machine);
        if (address !== undefined && hostname !== (await this.#readName())) {
            throw new CallError("MISDIRECTED_REQUEST", {
                message: `${hostname} is unassigned now`,
            });
        }

        // require a connected machine with room for another request
        const newest = this.#connections.at(-1);
        if (newest === undefined) {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `machine ${this.machineId} is disconnected`,
            });
        } else if (newest.session.size >= MAX_STREAMS) {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `machine ${this.machineId} has ${MAX_STREAMS} requests open`,
            });
        }

        return this.#send(newest.session, request);
    }

    /** Take the name the relay routes to the machine now, asking a connected machine to renew. */
    async rename(name: string): Promise<void> {
        // take the name for later renewals and requests, and leave a machine without a connection
        this.#name = name;
        await this.#resumeAll();
        if (this.isEmpty) {
            return;
        }

        // ask the connected machine to renew, to learn the name
        const response = await this.fetch(new Request(this.#options.url, { method: "PUT" }));
        if (!response.ok) {
            throw new CallError("BAD_GATEWAY", {
                message: `machine ${this.machineId} refused to renew: ${response.status}`,
            });
        }
    }

    /** Run a session over a socket, taking the machine's renewals on the streams it opens. */
    #attach(id: string, socket: Socket): void {
        const added: Connection = {
            id,
            session: new Session(socket, "server", {
                accept: (stream) =>
                    void this.#renew(added, stream).catch((error: unknown) =>
                        instruments.captureException(error),
                    ),
            }),
        };
        this.#connections.push(added);
    }

    /** Run a session again over a hibernated socket with a connection row, closing one without. */
    async #resume(socket: Socket): Promise<boolean> {
        // take a connection already running
        if (this.#connections.some((each) => each.id === socket.id)) {
            return true;
        }

        // close a socket without its row, else run a session over it again
        const rows = await this.#rows();
        if (!rows.some((row) => row.id === socket.id)) {
            socket.close();

            return false;
        }
        this.#attach(socket.id, socket);

        return true;
    }

    /** Resume every hibernated socket of the machine after the partition slept. */
    async #resumeAll(): Promise<void> {
        if (this.isEmpty) {
            for (const socket of await this.#options.hibernation.list()) {
                SocketAttachment.parse(socket.attachment);
                await this.#resume(socket);
            }
        }
    }

    /** Read the name the relay routes to the machine, from the tunnel's row after a sleep. */
    async #readName(): Promise<string> {
        if (this.#name === undefined) {
            const [row] = await this.#options.store.database
                .select({ name: tunnel.table.name })
                .from(tunnel.table)
                .where(eq(tunnel.table.id, tunnel.identifier(Tunnel.id(this.machineId))));
            if (row === undefined) {
                throw new CallError("NOT_FOUND", {
                    message: `machine ${this.machineId} has no tunnel`,
                });
            }
            this.#name = row.name;
        }

        return this.#name;
    }

    /** Read the tunnel's connection rows, oldest lapse first. */
    async #rows(): Promise<readonly { readonly id: string; readonly lapsesAt: number }[]> {
        return this.#options.store.database
            .select({ id: connection.table.id, lapsesAt: connection.table.lapsesAt })
            .from(connection.table)
            .where(eq(connection.table.scope, Tunnel.id(this.machineId)))
            .orderBy(asc(connection.table.lapsesAt));
    }

    /** List the calls deleting some of a machine's connection rows. */
    static #deletions(machineId: Identifier<"machine">, ids: readonly string[]): Call[] {
        const scope = Tunnel.id(machineId);

        return ids.map((id) => connection.call("delete", connection.target({ scope, id })));
    }

    /** Run calls on the tunnel's rows as the relay installation, skipping an empty list. */
    async #mutate(calls: readonly Call[]): Promise<readonly unknown[]> {
        // run nothing without calls
        if (calls.length === 0) {
            return [];
        }

        // run them in one mutation as the installation
        const { store } = this.#options;

        return store.mutate(store.self, { id: RequestId.create(store.clock.now()), calls });
    }

    /** Take a connection out of the tunnel, absent when none has the identifier. */
    #remove(id: string): Connection | undefined {
        const index = this.#connections.findIndex((each) => each.id === id);

        return index === -1 ? undefined : this.#connections.splice(index, 1)[0];
    }

    /** Send a request over a session. */
    async #send(session: Session, request: Request): Promise<Response> {
        // abort the request once the answer timeout passes on the tunnel's clock
        const answerTimeout = this.#options.answerTimeout ?? ANSWER_TIMEOUT_MILLISECONDS;
        const expiring = new AbortController();
        const answered = new AbortController();
        void this.#options.store.clock.sleep(answerTimeout, answered.signal).then(
            () => expiring.abort(),
            () => undefined,
        );
        const signal = AbortSignal.any([request.signal, expiring.signal]);

        // forward it and tell a timeout from the caller's own abort
        try {
            return await session.fetch(new Request(request, { signal }));
        } catch (error) {
            if (expiring.signal.aborted) {
                throw new CallError("GATEWAY_TIMEOUT", {
                    message: `machine ${this.machineId} answered no head within ${answerTimeout} ms`,
                });
            }
            throw error;
        } finally {
            answered.abort();
        }
    }

    /** Move a connection row's lapse to its renewed token's. */
    async #renewed(id: string, lapsesAt: number): Promise<void> {
        await this.#serially(() =>
            this.#mutate([
                connection.call("renew", {
                    ...connection.target({ scope: Tunnel.id(this.machineId), id }),
                    lapsesAt,
                }),
            ]),
        );
    }

    /** Write the tunnel's rows after its earlier writes, one write at a time. */
    #serially<Value>(write: () => Promise<Value>): Promise<Value> {
        // chain the write after the last, which a failure leaves usable
        const next = this.#writing.then(write);
        this.#writing = next.catch(() => undefined);

        return next;
    }

    /** Renew a connection's token through a stream the machine opened. */
    async #renew(renewing: Connection, stream: Stream): Promise<void> {
        // verify a renewal through the tunnel path, and refuse any other request
        const request = stream.request();
        const isRenewal = request.method === "PUT" && new URL(request.url).pathname === TUNNEL_PATH;
        const response = isRenewal
            ? await this.#verifyRenewal(request).then(
                  async (lapsesAt) => {
                      // record the lapse and answer the name and the lapse
                      await this.#renewed(renewing.id, lapsesAt);
                      const renewal: Renewal = { name: await this.#readName(), lapsesAt };

                      return Response.json(renewal);
                  },
                  (error: unknown) => (renewing.session.isClosed ? undefined : refusal(error)),
              )
            : refusal(new CallError("NOT_FOUND", { message: "machines only renew their tunnel" }));

        // leave a renewal whose session closed while it was read, as nobody takes its answer
        if (response === undefined) {
            return;
        }

        // answer the renewal
        await stream.respond(response);

        // lapse a connection whose renewal the relay refused, for the lapse controller to end
        if (isRenewal && !response.ok) {
            await this.#renewed(renewing.id, this.#options.store.clock.now());
        }
    }

    /** Verify the machine's own renewed token in a request's bearer header. */
    async #verifyRenewal(request: Request): Promise<number> {
        const verified = await Tunnel.verify(
            this.#options.tokens,
            Bearer.require(request.headers),
            this.#options.store.clock.now(),
        );
        if (verified.machineId !== this.machineId) {
            throw new CallError("FORBIDDEN", {
                message: "machines only renew their own tunnels",
            });
        }

        return verified.lapsesAt;
    }
}
