import type { Identifier } from "@destack/schema";
import type { Alarm } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { refusal } from "@destack/service/server";
import { MAX_STREAMS, Session, type Stream, type Transport } from "../session/index.ts";

/** The path machines open and renew their tunnels at. */
export const TUNNEL_PATH = "/tunnel";

/** How long a machine may take to answer a request's head by default: 100 s, as Cloudflare waits for an origin. */
const ANSWER_TIMEOUT_MILLISECONDS = 100_000;

/** A renewed tunnel token: when it lapses, and the name the relay routes to its machine. */
export interface Renewal {
    /** When the renewed token lapses, in UTC epoch milliseconds. */
    readonly lapsesAt: number;
    /** The name the relay routes to the machine, such as `laptop.florian.destack.computer`. */
    readonly name: string;
}

/** One connection of a machine's tunnel: its session, open until its token lapses. */
interface Connection {
    /** The session over the connection. */
    readonly session: Session;
    /** When the connection's token lapses, in UTC epoch milliseconds. */
    lapsesAt: number;
}

// TODO #Incomplete: serve each machine's Tunnel as a Durable Object in a workerd entry, closing connections on the object's alarm
/** A machine's tunnel to the relay over its connections: the newest takes new requests, and each closes once its token lapses. */
export class Tunnel {
    /** The machine the tunnel reaches. */
    readonly machineId: Identifier<"machine">;
    /** Verify a renewal of a connection's token, answering when the new token lapses and the machine's name. */
    readonly #verify: (request: Request) => Promise<Renewal>;
    /** The wake-up closing connections as their tokens lapse. */
    readonly #alarm: Alarm;
    /** Report a failure no request answers, such as a failed alarm. */
    readonly #report: (error: unknown) => void;
    /** How long the machine may take to answer a request's head, in milliseconds. */
    readonly #answerTimeout: number;
    /** The open connections, oldest first. */
    readonly #connections: Connection[] = [];

    /** Keep a machine's connections, renewing their tokens as a verifier admits them and closing them on an alarm as they lapse. */
    constructor(
        machineId: Identifier<"machine">,
        options: {
            /** Verify a renewal of a connection's token, answering when the new token lapses and the machine's name. */
            readonly verify: (request: Request) => Promise<Renewal>;
            /** The wake-up closing connections as their tokens lapse. */
            readonly alarm: Alarm;
            /** Report a failure no request answers. */
            readonly report: (error: unknown) => void;
            /** How long the machine may take to answer a request's head, 100 s by default. */
            readonly answerTimeout?: number;
        },
    ) {
        // keep the machine, its verifier, alarm, report and answer timeout
        this.machineId = machineId;
        this.#verify = options.verify;
        this.#alarm = options.alarm;
        this.#report = options.report;
        this.#answerTimeout = options.answerTimeout ?? ANSWER_TIMEOUT_MILLISECONDS;
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
                accept: (stream) => void this.#renew(connection, stream).catch(this.#report),
            }),
            lapsesAt,
        };
        this.#connections.push(connection);
        this.#arm().catch(this.#report);

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

    /** Forward a request through the newest connection, refusing one beyond the machine's open requests or answered too late. */
    async fetch(request: Request): Promise<Response> {
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

        // forward it, resetting the stream once the machine answers no head in time
        const expiring = new AbortController();
        const timer = setTimeout(() => expiring.abort(), this.#answerTimeout);
        const signal = AbortSignal.any([request.signal, expiring.signal]);
        try {
            return await newest.session.fetch(new Request(request, { signal }));
        } catch (error) {
            if (expiring.signal.aborted) {
                throw new ServiceError("GATEWAY_TIMEOUT", {
                    message: `machine ${this.machineId} answered no head within ${this.#answerTimeout} ms`,
                });
            }
            throw error;
        } finally {
            clearTimeout(timer);
        }
    }

    /** Renew a connection's token through a stream the machine opened, answering the machine's name or the refusal, and lapse a refused connection. */
    async #renew(connection: Connection, stream: Stream): Promise<void> {
        // verify a renewal through the tunnel path, and refuse any other request
        const request = stream.request();
        const isRenewal = request.method === "PUT" && new URL(request.url).pathname === TUNNEL_PATH;
        const response = isRenewal
            ? await this.#verify(request).then(
                  async (renewal) => {
                      connection.lapsesAt = renewal.lapsesAt;
                      await this.#arm();

                      return Response.json({ name: renewal.name });
                  },
                  (error: unknown) => refusal(error),
              )
            : refusal(
                  new ServiceError("NOT_FOUND", { message: "machines only renew their tunnel" }),
              );

        // answer, leaving a stream whose session ended before the answer
        await stream.respond(response).catch(() => {});

        // lapse a connection whose machine no longer stands, for the alarm to close
        if (isRenewal && !response.ok) {
            connection.lapsesAt = Date.now();
            await this.#arm();
        }
    }

    /** Set the alarm to the earliest lapse, or clear it once no connection is open. */
    async #arm(): Promise<void> {
        const lapsesAt = this.lapsesAt;
        await (lapsesAt === undefined ? this.#alarm.deleteAlarm() : this.#alarm.setAlarm(lapsesAt));
    }
}
