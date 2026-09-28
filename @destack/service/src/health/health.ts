import { schema } from "@destack/schema";
import { Observable } from "../observable/index.ts";

/** The health of a service. */
export class Health {
    /** The declared service name. */
    readonly name: string;
    /** The current status. */
    readonly #status = new Observable<HealthStatus>("starting");

    /** Create the health. */
    constructor(name: string) {
        this.name = name;
    }

    /** Read the current status. */
    get status(): HealthStatus {
        return this.#status.value;
    }

    /** Set the status. */
    set(status: HealthStatus): void {
        if (status !== this.status) {
            this.#status.set(status);
        }
    }

    /** Describe the health. */
    check(): schema.Infer<typeof HealthDescription> {
        return { name: this.name, status: this.status };
    }

    /** Yield the health and its changes. */
    async *watch(signal?: AbortSignal) {
        // yield each change
        for await (const status of this.#status.watch(signal)) {
            yield { name: this.name, status };
            if (status === "draining" || status === "stopped") {
                return;
            }
        }
    }

    /** Answer liveness and readiness probes. */
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
        const ready = path === "/livez" ? this.status !== "stopped" : this.status === "serving";

        return new Response(request.method === "HEAD" ? null : ready ? "ok\n" : "unavailable\n", {
            status: ready ? 200 : 503,
            headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" },
        });
    }
}

/** The readiness of a service. */
export const HealthStatus = schema.enum([
    "starting",
    "serving",
    "not-serving",
    "draining",
    "stopped",
]);
/** The readiness of a service. */
export type HealthStatus = schema.Infer<typeof HealthStatus>;

/** The health of one service. */
export const HealthDescription = schema.object({
    /** The service name. */
    name: schema.string().min(1),
    /** The status. */
    status: HealthStatus,
});
