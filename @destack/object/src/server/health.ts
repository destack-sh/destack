import { implement } from "./handler.ts";
import { health, HealthDescription, HealthStatus } from "../health/index.ts";
import type { schema } from "@destack/schema";
import { Observable } from "../client/observable.ts";
import { ObjectError } from "../error/index.ts";

/** The milliseconds a starting server asks its callers to wait before they retry. */
export const STARTING_RETRY_MILLISECONDS = 1000;

/** A health status with the reason it holds. */
interface HealthState {
    /** The status. */
    readonly status: HealthStatus;
    /** Why the status holds, such as what a starting server waits for. */
    readonly reason: string;
}

/** The health of a server: whether it serves, and why not. */
export class Health {
    /** The server's name, such as its package's. */
    readonly name: string;
    /** The current status and its reason. */
    readonly #state: Observable<HealthState>;

    /** Create the health of a server that has not started yet. */
    constructor(name: string) {
        this.name = name;
        this.#state = new Observable<HealthState>({
            status: "starting",
            reason: "the server has not started yet",
        });
    }

    /** Read the current status. */
    get status(): HealthStatus {
        return this.#state.value.status;
    }

    /** Read why the current status holds. */
    get reason(): string {
        return this.#state.value.reason;
    }

    /** Set the status and its reason. */
    set(status: HealthStatus, reason: string): void {
        if (status !== this.status || reason !== this.reason) {
            this.#state.set({ status, reason });
        }
    }

    /** Describe the health. */
    check(): schema.Infer<typeof HealthDescription> {
        return { name: this.name, ...this.#state.value };
    }

    /** Yield the health and its changes. */
    async *watch(signal?: AbortSignal) {
        // yield each change, ending once the server drains or stops
        for await (const state of this.#state.watch(signal)) {
            yield { name: this.name, ...state };
            if (state.status === "draining" || state.status === "stopped") {
                return;
            }
        }
    }

    /** Wait until the server is up, refusing at once once it fails, drains or stops. */
    async waitUp(signal?: AbortSignal): Promise<void> {
        // settle once up, and refuse any status past starting
        for await (const { status, reason } of this.#state.watch(signal)) {
            if (status === "up") {
                return;
            } else if (status !== "starting") {
                throw this.#refusal(status, reason);
            }
        }

        // refuse a wait its signal aborted with why the server is not up yet
        const { status, reason } = this.#state.value;
        if (status !== "up") {
            throw this.#refusal(status, reason);
        }
    }

    /** Describe why the server refuses a request now, absent while it serves. */
    refusal(): Error | undefined {
        const { status, reason } = this.#state.value;

        return status === "up" ? undefined : this.#refusal(status, reason);
    }

    /** Describe the refusal a status gives, naming the server, the status and its reason. */
    #refusal(status: Exclude<HealthStatus, "up">, reason: string): Error {
        const message = `${this.name} is ${status}: ${reason}`;
        if (status === "starting") {
            return ObjectError.of("SERVER_STARTING", message, {
                data: { retryAfter: STARTING_RETRY_MILLISECONDS },
            });
        } else if (status === "down") {
            return ObjectError.of("SERVER_DOWN", message);
        } else if (status === "draining") {
            return ObjectError.of("SERVER_DRAINING", message);
        } else {
            return ObjectError.of("SERVER_STOPPED", message);
        }
    }

    /** Answer liveness and readiness probes, a refused readiness with its reason. */
    probe(request: Request): Response | undefined {
        // match the probe paths
        const path = new URL(request.url).pathname;
        if (path !== "/livez" && path !== "/readyz") {
            return;
        }

        // accept only GET and HEAD
        if (request.method !== "GET" && request.method !== "HEAD") {
            return new Response(null, { status: 405, headers: { allow: "GET, HEAD" } });
        }

        // check liveness or readiness
        const ready = path === "/livez" ? this.status !== "stopped" : this.status === "up";
        const body = ready ? "ok\n" : `${this.status}: ${this.reason}\n`;

        // answer a server still starting with when to ask again, as its refusals do
        const headers = new Headers({
            "content-type": "text/plain; charset=utf-8",
            "cache-control": "no-store",
        });
        if (!ready && this.status === "starting") {
            headers.set("retry-after", String(Math.ceil(STARTING_RETRY_MILLISECONDS / 1000)));
        }

        return new Response(request.method === "HEAD" ? null : body, {
            status: ready ? 200 : 503,
            headers,
        });
    }
}

/** Implement the health procedures. */
export function implementHealth(readiness: Health) {
    // implement the procedures
    const implementation = implement(health);

    return implementation.router({
        check: implementation.check.handler(() => readiness.check()),
        watch: implementation.watch.handler(({ signal }) => readiness.watch(signal)),
    });
}
