import { principal } from "@destack/access";
import { type Domains, MachineAddress } from "@destack/host";
import { schema, type Identifier } from "@destack/schema";
import { type Authentication, Bearer, type TokenVerifier } from "@destack/service/authentication";
import { type Alarm, TimerAlarm } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { refusal } from "@destack/service/server";
import {
    MAX_STREAMS,
    type Renewal,
    Session,
    type Stream,
    type Transport,
} from "../session/index.ts";
import type { Relay } from "./relay.ts";

/** The path machines open and renew their tunnels at. */
export const TUNNEL_PATH = "/tunnel";

/** How long a machine may take to answer a request's head by default: 100 s, as edge proxies commonly wait for an origin. */
const ANSWER_TIMEOUT_MILLISECONDS = 100_000;

/** A machine admitted to open a tunnel: its token's lapse, and the name the relay routes to it. */
export const Admission = schema.object({
    /** The machine. */
    machineId: schema.identifier("machine"),
    /** When its token lapses, in UTC epoch milliseconds. */
    lapsesAt: schema.number(),
    /** The name the relay routes to the machine, such as `laptop.florian.destack.computer`. */
    name: schema.string().min(1),
});
/** A machine admitted to open a tunnel. */
export type Admission = schema.Infer<typeof Admission>;

/** One connection of a machine's tunnel: its session, open until its token lapses. */
interface Connection {
    /** The session over the connection. */
    readonly session: Session;
    /** When the connection's token lapses, in UTC epoch milliseconds. */
    lapsesAt: number;
}

/** How a tunnel verifies its renewals, closes its lapsed connections and reports. */
export interface TunnelOptions {
    /** The relay's tunnel URL the machine dials, which its renames come from. */
    readonly url: string;
    /** Verify the universe's tokens machines renew their tunnels with, without a database. */
    readonly tokens: Pick<TokenVerifier, "verify">;
    /** The wake-up closing connections as their tokens lapse. */
    readonly alarm: Alarm;
    /** Report a failure no request answers, such as a failed alarm. */
    readonly report: (error: unknown) => void;
    /** The domains names resolve under. */
    readonly domains: Domains;
    /** Keep a connection's new lapse beyond the tunnel's memory, such as in a hibernating WebSocket's attachment. */
    readonly keep?: (session: Session, lapsesAt: number) => void;
    /** How long the machine may take to answer a request's head, 100 s by default. */
    readonly answerTimeout?: number;
}

/** Where a relay's machines keep their tunnels: in the relay's memory, or in a Durable Object per machine. */
export interface TunnelHost {
    /** Forward a request through a machine's tunnel, refusing it while the machine keeps none. */
    fetch(machineId: Identifier<"machine">, request: Request): Promise<Response>;
    /** Tell a machine through its tunnel the name the relay routes to it, nothing while it keeps none. */
    rename(machineId: Identifier<"machine">, name: string): Promise<void>;
    /** Close a machine's tunnel, as its revocation does, answering whether a connection was open. */
    close(machineId: Identifier<"machine">): Promise<boolean>;
}

/** A machine's tunnel to the relay over its connections: the newest takes new requests, and each closes once its token lapses. */
export class Tunnel {
    /** The machine the tunnel reaches. */
    readonly machineId: Identifier<"machine">;
    /** The name the relay routes to the machine, changed by its renames. */
    name: string;
    /** How the tunnel verifies renewals, closes lapsed connections and reports. */
    readonly #options: TunnelOptions;
    /** The open connections, oldest first. */
    readonly #connections: Connection[] = [];

    /** Keep a machine's connections under its name, renewing their tokens and closing them on an alarm as they lapse. */
    constructor(machineId: Identifier<"machine">, name: string, options: TunnelOptions) {
        this.machineId = machineId;
        this.name = name;
        this.#options = options;
    }

    /** Verify a machine's token without a database, answering the machine and its verified caller. */
    static async admit(
        tokens: Pick<TokenVerifier, "verify">,
        token: string,
        now = Date.now(),
    ): Promise<{ readonly machineId: Identifier<"machine">; readonly caller: Authentication }> {
        // require a machine's token
        const caller = await tokens.verify(token, undefined, now);
        const subject = caller.claims.subject;
        if (!principal.machine.is(subject)) {
            throw new ServiceError("FORBIDDEN", { message: "only machines open tunnels" });
        }

        return { machineId: schema.identifier("machine").parse(subject.id), caller };
    }

    /** Whether no connection of the machine is open. */
    get isEmpty(): boolean {
        return this.#connections.length === 0;
    }

    /** When the next connection's token lapses, absent while none is open. */
    get lapsesAt(): number | undefined {
        return this.#connections.length === 0
            ? undefined
            : Math.min(...this.#connections.map((connection) => connection.lapsesAt));
    }

    /** Carry a session over a new connection whose token lapses at a time, taking new requests from now on. */
    attach(transport: Transport, lapsesAt: number): Session {
        // take the machine's renewals on streams it opens, and wake at the earliest lapse
        const connection: Connection = {
            session: new Session(transport, "server", {
                accept: (stream) =>
                    void this.#renew(connection, stream).catch(this.#options.report),
            }),
            lapsesAt,
        };
        this.#connections.push(connection);
        this.#arm().catch(this.#options.report);

        return connection.session;
    }

    /** Forget a connection whose WebSocket closed, ending its streams. */
    async detach(session: Session): Promise<void> {
        // forget the session and end its streams
        const index = this.#connections.findIndex((connection) => connection.session === session);
        if (index >= 0) {
            this.#connections.splice(index, 1);
        }
        session.terminate();

        // move the alarm to the connections left
        await this.#arm();
    }

    /** Close the connections whose tokens lapsed by a time, as the alarm wakes. */
    async lapse(now = Date.now()): Promise<void> {
        // close each lapsed connection, telling the machine
        for (const connection of this.#connections.filter((entry) => entry.lapsesAt <= now)) {
            connection.session.close();
            this.#connections.splice(this.#connections.indexOf(connection), 1);
        }

        await this.#arm();
    }

    /** Close every connection, as the machine's revocation does, answering whether one was open. */
    async close(): Promise<boolean> {
        // close every connection and clear the alarm
        const closed = this.#connections.splice(0);
        for (const connection of closed) {
            connection.session.close();
        }
        await this.#arm();

        return closed.length > 0;
    }

    /** Forward a request through the newest connection, refusing a machine name the machine no longer has, one beyond its open requests or one answered too late. */
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

    /** Tell the machine the name the relay routes to it now, refusing when a connected machine does not take it. */
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

    /** Send a request over a session, resetting the stream once the machine answers no head in time. */
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

    /** Renew a connection's token through a stream the machine opened, answering its name or the refusal, and lapse a refused connection. */
    async #renew(connection: Connection, stream: Stream): Promise<void> {
        // verify a renewal through the tunnel path, and refuse any other request
        const request = stream.request();
        const isRenewal = request.method === "PUT" && new URL(request.url).pathname === TUNNEL_PATH;
        const response = isRenewal
            ? await this.#verify(request).then(
                  async (lapsesAt) => {
                      // move the lapse and answer the name and the token's remaining lifetime
                      await this.#lapse(connection, lapsesAt);
                      const expiresIn = Math.max(0, (lapsesAt - Date.now()) / 1000);
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

        // lapse a connection whose renewal the relay refused for the alarm to close
        if (isRenewal && !response.ok) {
            await this.#lapse(connection, Date.now());
        }
    }

    /** Verify the machine's own renewed token in a request's bearer header, answering when it lapses. */
    async #verify(request: Request): Promise<number> {
        const admitted = await Tunnel.admit(this.#options.tokens, Bearer.require(request.headers));
        if (admitted.machineId !== this.machineId) {
            throw new ServiceError("FORBIDDEN", {
                message: "machines only renew their own tunnels",
            });
        }

        return admitted.caller.lapsesAt;
    }

    /** Move a connection's lapse, keeping it beyond memory and setting the alarm to the earliest lapse. */
    async #lapse(connection: Connection, lapsesAt: number): Promise<void> {
        connection.lapsesAt = lapsesAt;
        this.#options.keep?.(connection.session, lapsesAt);
        await this.#arm();
    }

    /** Set the alarm to the earliest lapse, or clear it once no connection is open. */
    async #arm(): Promise<void> {
        const lapsesAt = this.lapsesAt;
        const alarm = this.#options.alarm;
        await (lapsesAt === undefined ? alarm.deleteAlarm() : alarm.setAlarm(lapsesAt));
    }
}

/** The tunnels of a relay's machines in the relay's own memory, as one process serving every tunnel keeps them. */
export class MemoryTunnelHost implements TunnelHost {
    /** The tunnel of each connected machine. */
    readonly #tunnels = new Map<string, Tunnel>();

    /** Run a session over an admitted machine's opened WebSocket as the newest of its tunnel. */
    attach(relay: Relay, admission: Admission, transport: Transport): Session {
        // find or keep the machine's tunnel under the name its admission read
        const { machineId, lapsesAt, name } = admission;
        const tunnel =
            this.#tunnels.get(machineId) ??
            new Tunnel(machineId, name, {
                url: relay.url,
                tokens: relay.tokens,
                alarm: new TimerAlarm(() => void tunnel.lapse().catch(relay.report)),
                report: relay.report,
                domains: relay.domains,
            });
        this.#tunnels.set(machineId, tunnel);

        return tunnel.attach(transport, lapsesAt);
    }

    /** Forget a machine's closed WebSocket, and the machine's tunnel once none of its WebSockets remain. */
    async detach(machineId: Identifier<"machine">, session: Session): Promise<void> {
        // find the machine's tunnel
        const tunnel = this.#tunnels.get(machineId);
        if (tunnel === undefined) {
            return;
        }

        // forget the session and the tunnel without connections
        await tunnel.detach(session);
        if (tunnel.isEmpty) {
            this.#tunnels.delete(machineId);
        }
    }

    /** Forward a request through a machine's tunnel, refusing it while the machine keeps none. */
    async fetch(machineId: Identifier<"machine">, request: Request): Promise<Response> {
        const tunnel = this.#tunnels.get(machineId);
        if (tunnel === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `machine ${machineId} is not connected`,
            });
        }

        return tunnel.fetch(request);
    }

    /** Tell a connected machine the name the relay routes to it. */
    async rename(machineId: Identifier<"machine">, name: string): Promise<void> {
        await this.#tunnels.get(machineId)?.rename(name);
    }

    /** Close a machine's tunnel, answering whether a connection was open. */
    async close(machineId: Identifier<"machine">): Promise<boolean> {
        return (await this.#tunnels.get(machineId)?.close()) ?? false;
    }
}
