import type { Environment } from "@destack/environment";
import { LineExporter } from "@destack/environment/line";
import { OriginPath } from "@destack/package";
import type { Plugin } from "@destack/package/build";
import { present } from "@destack/schema";
import { OtlpExport, type OtlpSignal } from "@destack/telemetry/otlp";
import type { Connect } from "vite";

/** The OTLP signals the server takes below the telemetry path, each at `/v1/<signal>`. */
const SIGNALS: readonly OtlpSignal[] = ["traces", "logs", "metrics"];

/** The base the server reads request paths against, which have no origin. */
export const ORIGIN = "http://localhost";

/** Answer the paths an origin reserves: print telemetry exports, refuse any other path bare. */
export function originPlugin(environment: Environment<"process">): Plugin {
    return {
        name: "@destack/view/origin",
        configureServer(vite) {
            // print each received span and record as a process's log prints them
            const lines = new LineExporter(
                (line, severity) => LineExporter.console(line, severity),
                LineExporter.options(environment.process.variables),
            );

            // answer below the reserved root before Vite and the view, as an installation origin does
            vite.middlewares.use((request, response, next) => {
                // leave every path outside the reserved root to Vite and the view
                const { pathname } = new URL(present(request.url, "the request's path"), ORIGIN);
                if (pathname !== OriginPath.root && !pathname.startsWith(`${OriginPath.root}/`)) {
                    next();

                    return;
                }

                // take an export posted at a signal's path, and refuse any other request bare
                const signal = SIGNALS.find(
                    (candidate) => pathname === `${OriginPath.telemetry}/v1/${candidate}`,
                );
                if (request.method !== "POST" || signal === undefined) {
                    response.statusCode = 404;
                    response.end();

                    return;
                }
                printExport(request, signal, lines).then(
                    () => {
                        response.statusCode = 200;
                        response.setHeader("content-type", "application/json");
                        response.end("{}");
                    },
                    (error: unknown) => {
                        response.statusCode = 400;
                        response.end(String(error));
                    },
                );
            });
        },
    };
}

/** Print an OTLP/JSON export's spans and log records to the server's log, its metrics left out. */
async function printExport(
    request: Connect.IncomingMessage,
    signal: OtlpSignal,
    lines: LineExporter,
): Promise<void> {
    // read the posted body as an export of the signal
    const chunks: Uint8Array[] = [];
    for await (const chunk of request) {
        // require bytes, as a body streamed without an encoding is
        if (!(chunk instanceof Uint8Array)) {
            throw new TypeError("the export's body streamed text instead of bytes");
        }
        chunks.push(chunk);
    }
    const text = new TextDecoder().decode(Buffer.concat(chunks));
    const exported = new OtlpExport(signal, JSON.parse(text));

    // print each span as it ended, and each record at its severity
    if (signal === "traces") {
        for (const span of exported.spans()) {
            lines.spans.onEnd(span);
        }
    } else if (signal === "logs") {
        lines.logs.export(exported.records(), () => undefined);
    }
}
