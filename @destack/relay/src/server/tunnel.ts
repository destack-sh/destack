import { Caller, principal } from "@destack/access";
import { type Domains, MachineAddress } from "@destack/account/address";
import { ResidencyCode } from "@destack/account/object";
import { Clock, Identifier, schema } from "@destack/schema";
import { Bearer, type TokenVerifier } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { refusal } from "@destack/service/server";
import {
    MAX_STREAMS,
    type Renewal,
    Session,
    type Stream,
    type Transport,
} from "../session/index.ts";

/** The path below the relay service's mount for the router's tunnel calls. */
export const TUNNELS_PATH = "/tunnels";

/** The header that carries an admitted machine's admission. */
export const ADMISSION_HEADER = "Destack-Admission";

/** The path machines open and renew their tunnels at. */
export const TUNNEL_PATH = "/tunnel";

/** How long a machine may take to answer a request's head by default: 100 s. */
const ANSWER_TIMEOUT_MILLISECONDS = 100_000;

/** A machine's tunnel as the relay calls it. */
export const MachineDestination = schema.object({
    /** The machine. */
    machineId: schema.identifier("machine"),
    /** The residency the machine's account keeps data in. */
    residencyId: ResidencyCode,
});
/** A machine's tunnel as the relay calls it. */
export type MachineDestination = schema.Infer<typeof MachineDestination>;

/** A machine admitted to open a tunnel. */
export const Admission = MachineDestination.extend({
    /** When its token lapses, in UTC epoch milliseconds. */
    lapsesAt: schema.number(),
    /** The name the relay routes to the machine, such as `laptop--florian.destack.computer`. */
    name: schema.string().min(1),
    /** The token the relay published the machine's endpoint under, which withdrawing it names. */
    publication: schema.string().min(1),
});
/** A machine admitted to open a tunnel. */
export type Admission = schema.Infer<typeof Admission>;

/** One connection of a machine's tunnel: its held socket's session, open until its token lapses. */
interface Connection {
    /** The held socket's identifier, the connection row's key. */
    readonly id: string;
    /** The session over the connection. */
    readonly session: Session;
}

/** How a tunnel verifies its renewals, records their lapses and reports. */
export interface TunnelOptions {
    /** The relay's tunnel URL the machine dials, which its renames come from. */
    readonly url: string;
    /** Verify the universe's tokens machines renew their tunnels with, without a database. */
    readonly tokens: Pick<TokenVerifier, "verify">;
    /** Report a failure no request answers. */
    readonly report: (error: unknown) => void;
    /** The domains names resolve under. */
    readonly domains: Domains;
    /** Record a connection's lapse beyond the tunnel's memory. */
    readonly renewed: (id: string, lapsesAt: number) => Promise<void>;
    /** The clock renewals are verified and lapsed at, the store's. */
    readonly clock: Clock;
    /** How long the machine may take to answer a request's head, 100 s by default. */
    readonly answerTimeout?: number;
}

/** A machine's tunnel to the relay over its connections. */
export class Tunnel {
    /** The machine the tunnel leads to. */
    readonly machineId: Identifier<"machine">;
    /** The name the relay routes to the machine, changed by its renames. */
    name: string;
    /** How the tunnel verifies renewals, records lapses and reports. */
    readonly #options: TunnelOptions;
    /** The open connections, oldest first. */
    readonly #connections: Connection[] = [];

    /** Keep a machine's connections under the name the relay routes to it. */
    constructor(machineId: Identifier<"machine">, name: string, options: TunnelOptions) {
        this.machineId = machineId;
        this.name = name;
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

    /** Verify a machine's token without a database. */
    static async admit(
        tokens: Pick<TokenVerifier, "verify">,
        token: string,
        now: number,
    ): Promise<{ readonly machineId: Identifier<"machine">; readonly caller: Caller }> {
        // require a machine's token
        const caller = await tokens.verify(token, now);
        const { subject } = caller;
        if (subject === undefined || !principal.machine.is(subject)) {
            throw new ServiceError("FORBIDDEN", { message: "only machines open tunnels" });
        }

        return { machineId: schema.identifier("machine").parse(subject.id), caller };
    }

    /** Whether no connection of the machine is open. */
    get isEmpty(): boolean {
        return this.#connections.length === 0;
    }

    /** Report whether a connection of the held socket's identifier is open. */
    has(id: string): boolean {
        return this.#connections.some((connection) => connection.id === id);
    }

    /** Carry a session over a held socket, taking new requests from now on. */
    attach(id: string, transport: Transport): Session {
        // take the machine's renewals on streams it opens
        const connection: Connection = {
            id,
            session: new Session(transport, "server", {
                accept: (stream) =>
                    void this.#renew(connection, stream).catch(this.#options.report),
            }),
        };
        this.#connections.push(connection);

        return connection.session;
    }

    /** Feed a held socket's message to its connection's session. */
    receive(id: string, message: Uint8Array): void {
        this.#connections.find((connection) => connection.id === id)?.session.receive(message);
    }

    /** Forget a connection whose socket closed, ending its streams. */
    detach(id: string): void {
        this.#remove(id)?.session.terminate();
    }

    /** End a connection, telling the machine, as its token lapsed. */
    end(id: string): void {
        this.#remove(id)?.session.close();
    }

    /** End every connection, as the machine's revocation does. */
    close(): readonly string[] {
        const closed = this.#connections.splice(0);
        for (const connection of closed) {
            connection.session.close();
        }

        return closed.map((connection) => connection.id);
    }

    /** Forward a request through the newest connection. */
    async fetch(request: Request): Promise<Response> {
        // refuse a machine-domain name other than the machine's own
        const hostname = new URL(request.url).hostname;
        const address = MachineAddress.parse(hostname, this.#options.domains.machine);
        if (address !== undefined && hostname !== this.name) {
            throw new ServiceError("MISDIRECTED_REQUEST", {
                message: `${hostname} names no machine now`,
            });
        }

        // require a connected machine with room for another request
        const newest = this.#connections.at(-1);
        if (newest === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `machine ${this.machineId} is not connected`,
            });
        } else if (newest.session.size >= MAX_STREAMS) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `machine ${this.machineId} has ${MAX_STREAMS} requests open`,
            });
        }

        return this.#send(newest.session, request);
    }

    /** Tell the machine the name the relay routes to it now. */
    async rename(name: string): Promise<void> {
        // take the name for later renewals and requests
        this.name = name;
        if (this.isEmpty) {
            return;
        }

        // tell the connected machine
        const response = await this.fetch(
            new Request(this.#options.url, { method: "PUT", body: JSON.stringify({ name }) }),
        );
        if (!response.ok) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `machine ${this.machineId} refused its name: ${response.status}`,
            });
        }
    }

    /** Take a connection out of the tunnel, absent when none has the identifier. */
    #remove(id: string): Connection | undefined {
        const index = this.#connections.findIndex((connection) => connection.id === id);

        return index === -1 ? undefined : this.#connections.splice(index, 1)[0];
    }

    /** Send a request over a session. */
    async #send(session: Session, request: Request): Promise<Response> {
        // abort the request once the answer timeout passes
        const answerTimeout = this.#options.answerTimeout ?? ANSWER_TIMEOUT_MILLISECONDS;
        const expiring = new AbortController();
        const timer = setTimeout(() => expiring.abort(), answerTimeout);
        const signal = AbortSignal.any([request.signal, expiring.signal]);

        // forward it and tell a timeout from the caller's own abort
        try {
            return await session.fetch(new Request(request, { signal }));
        } catch (error) {
            if (expiring.signal.aborted) {
                throw new ServiceError("GATEWAY_TIMEOUT", {
                    message: `machine ${this.machineId} answered no head within ${answerTimeout} ms`,
                });
            }
            throw error;
        } finally {
            clearTimeout(timer);
        }
    }

    /** Renew a connection's token through a stream the machine opened. */
    async #renew(connection: Connection, stream: Stream): Promise<void> {
        // verify a renewal through the tunnel path, and refuse any other request
        const request = stream.request();
        const isRenewal = request.method === "PUT" && new URL(request.url).pathname === TUNNEL_PATH;
        const response = isRenewal
            ? await this.#verify(request).then(
                  async (lapsesAt) => {
                      // record the lapse and answer the name and the token's remaining lifetime
                      await this.#options.renewed(connection.id, lapsesAt);
                      const expiresIn = Math.max(0, (lapsesAt - this.#options.clock.now()) / 1000);
                      const renewal: Renewal = { name: this.name, expiresIn };

                      return Response.json(renewal);
                  },
                  (error: unknown) => refusal(error),
              )
            : refusal(
                  new ServiceError("NOT_FOUND", { message: "machines only renew their tunnel" }),
              );

        // answer, leaving a stream whose session ended before the answer
        await stream.respond(response).catch(() => {});

        // lapse a connection whose renewal the relay refused, for the lapse controller to end
        if (isRenewal && !response.ok) {
            await this.#options.renewed(connection.id, this.#options.clock.now());
        }
    }

    /** Verify the machine's own renewed token in a request's bearer header. */
    async #verify(request: Request): Promise<number> {
        const admitted = await Tunnel.admit(
            this.#options.tokens,
            Bearer.require(request.headers),
            this.#options.clock.now(),
        );
        if (admitted.machineId !== this.machineId) {
            throw new ServiceError("FORBIDDEN", {
                message: "machines only renew their own tunnels",
            });
        }

        return Caller.requireCredential(admitted.caller).expiresAt;
    }
}
