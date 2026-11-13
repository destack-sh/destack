import { type Environment, SpanPath } from "@destack/environment";
import { type Attributes, TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The span total counting the statements run inside a span, as the database adds them. */
const STATEMENTS = "db.client.operation.count";

/** The RPC system of every call, as OpenTelemetry's `rpc.system.name` reports it. */
const RPC_SYSTEM = "orpc";

/** Trace one end's RPC calls: a span and a duration each, a stream's until it closes. */
export function traceCalls(
    kind: "client" | "server",
    environment: Environment,
): (
    path: readonly string[],
    next: () => Promise<unknown>,
    labels?: Attributes,
) => Promise<unknown> {
    // declare the statements per call
    const scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const methods: readonly string[] = [];
    const statements = scope.metric.histogram("destack.call.statements", {
        unit: "{statement}",
        attributes: { "rpc.method": methods },
    });

    return (path, next, labels = {}) => {
        // run the call in a span named by its method, recording the server's statements per call
        const method = path.join("/");
        const attributes = { "rpc.system.name": RPC_SYSTEM, "rpc.method": method, ...labels };
        const duration = { name: `rpc.${kind}.call.duration`, attributes };

        return scope.span(
            method,
            attributes,
            async () => {
                // answer the call, then record the statements it ran on the server
                const result = await next();
                const ran = SpanPath.totals(environment.telemetry.context.active())[STATEMENTS];
                if (kind === "server" && typeof ran === "number") {
                    statements.record(ran, { "rpc.method": method });
                }

                return result;
            },
            { kind, duration },
        );
    };
}
