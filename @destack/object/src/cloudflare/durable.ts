import { type Clock, WebSocketUpgrade } from "@destack/environment";
import type { Alarm } from "@destack/db/database";
import { type HibernationHost, SOCKET_HEADER } from "../workload/socket.ts";
import {
    DurableObjectHibernation,
    type HibernatingSocket,
    type HibernationState,
} from "./hibernation.ts";

/** The tag of the WebSockets a workload object keeps for its workload's servers. */
const WORKLOAD_SOCKET_TAG = "workload";

/** How long an alarm runs a Durable Object's controllers at most: 10 minutes. */
export const ALARM_MILLISECONDS = 10 * 60_000;

/** The members of a Durable Object's state a workload object uses. */
export interface DurableObjectState extends HibernationState {
    /** The object's storage, whose alarm wakes the object. */
    readonly storage: Alarm;
    /** Block the object's other events until a closure settles. */
    readonly blockConcurrencyWhile: <Value>(closure: () => Promise<Value>) => Promise<Value>;
}

/** A workload an object serves: its requests, and the controllers the object's alarm wakes. */
export interface DurableObjectWorkloadInstance {
    /** Serve a request. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** Run the controllers until no key is due now or reconciling, or until a deadline. */
    readonly alarm: (deadline: number) => Promise<void>;
    /** The host keeping its servers' connections' sockets, absent for a workload without any. */
    readonly sockets?: HibernationHost;
}

/** A Durable Object with one workload instance. */
export class DurableObjectWorkload {
    /** The started workload. */
    readonly #started: Promise<DurableObjectWorkloadInstance>;
    /** The clock alarms run on. */
    readonly #clock: Clock;
    /** The WebSockets the object keeps for the workload's servers across hibernation. */
    readonly #sockets: DurableObjectHibernation;

    /** Start a workload before the object takes any event, its alarms timed on a clock. */
    constructor(
        state: DurableObjectState,
        start: (alarm: Alarm) => Promise<DurableObjectWorkloadInstance>,
        clock: Clock,
    ) {
        this.#started = state.blockConcurrencyWhile(() => start(state.storage));
        this.#clock = clock;
        this.#sockets = new DurableObjectHibernation(state, WORKLOAD_SOCKET_TAG);
    }

    /** Serve a request through the started workload, keeping the sockets its servers accept. */
    async fetch(request: Request): Promise<Response> {
        // answer the request
        const workload = await this.#started;
        const answer = await workload.fetch(request);
        const isAccepted =
            WebSocketUpgrade.of(request) !== undefined && answer.headers.has(SOCKET_HEADER);

        return isAccepted ? this.#sockets.accept(await this.#host(), answer) : answer;
    }

    /** Hand a kept WebSocket's message to the workload's servers. */
    async webSocketMessage(
        socket: HibernatingSocket,
        message: ArrayBuffer | string,
    ): Promise<void> {
        this.#sockets.message(await this.#host(), socket, message);
    }

    /** Hand a kept WebSocket's close to the workload's servers. */
    async webSocketClose(socket: HibernatingSocket, code: number, reason: string): Promise<void> {
        this.#sockets.close(await this.#host(), socket, code, reason);
    }

    /** Read the host the workload keeps its sockets at, refusing a workload without one. */
    async #host(): Promise<HibernationHost> {
        const { sockets } = await this.#started;
        if (sockets === undefined) {
            throw new TypeError("a workload without sockets accepts no WebSocket");
        }

        return sockets;
    }

    /** Run the controllers due at the alarm. */
    async alarm(): Promise<void> {
        // run for a bounded time
        const workload = await this.#started;
        await workload.alarm(this.#clock.now() + ALARM_MILLISECONDS);
    }
}
