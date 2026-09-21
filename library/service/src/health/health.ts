import { schema } from "@destack/schema";
import { Watch } from "../watch/index.ts";

/** Current service health and coalesced change notifications. */
export class Health {
    /** The declared service name. */
    readonly name: string;
    /** The current readiness state. */
    readonly #status = new Watch<HealthStatus>("starting");

    /** Initialize health before the host starts accepting work. */
    constructor(name: string) {
        this.name = name;
    }

    /** Read the current status. */
    get status(): HealthStatus {
        return this.#status.value;
    }

    /** Publish readiness after initialization, dependency changes, or shutdown. */
    set(status: HealthStatus): void {
        if (status !== this.status) {
            this.#status.set(status);
        }
    }

    /** Describe current service health. */
    check(): schema.Infer<typeof HealthDescription> {
        return { name: this.name, status: this.status };
    }

    /** Yield current health and changes until shutdown or subscriber cancellation. */
    async *watch(signal?: AbortSignal) {
        // deliver readiness changes until shutdown starts
        for await (const status of this.#status.watch(signal)) {
            yield { name: this.name, status };
            if (status === "draining" || status === "stopped") {
                return;
            }
        }
    }

    /** Answer conventional liveness and readiness probes, or return undefined for other paths. */
    probe(request: Request): Response | undefined {
        // accept only the standard probe addresses
        const path = new URL(request.url).pathname;
        if (path !== "/livez" && path !== "/readyz") {
            return;
        }

        // restrict probes to reads and omit bodies for HEAD requests
        if (request.method !== "GET" && request.method !== "HEAD") {
            return new Response(null, { status: 405, headers: { allow: "GET, HEAD" } });
        }

        // distinguish process liveness from readiness to accept work
        const ready = path === "/livez" ? this.status !== "stopped" : this.status === "serving";

        return new Response(request.method === "HEAD" ? null : ready ? "ok\n" : "unavailable\n", {
            status: ready ? 200 : 503,
            headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" },
        });
    }
}

/** Service readiness visible to clients and load balancers. */
export const HealthStatus = schema.enum([
    "starting",
    "serving",
    "not-serving",
    "draining",
    "stopped",
]);

/** Service readiness visible to clients and load balancers. */
export type HealthStatus = schema.Infer<typeof HealthStatus>;

/** Health of one named service. */
export const HealthDescription = schema.object({
    /** Declared service name. */
    name: schema.string().min(1),
    /** Current readiness. */
    status: HealthStatus,
});
