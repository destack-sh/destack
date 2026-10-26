import { Environment } from "@destack/environment";
import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";
import { BrowserEnvironment } from "@destack/environment/browser";
import { EnvironmentContext, TelemetryContext } from "@destack/view";
import { renderToStream } from "@destack/web/render";
import manifest from "virtual:solid-manifest";
import { DefaultErrorBoundary } from "virtual:solid-ssr-error-boundary.tsx";
import App from "./app.tsx";
import Document from "./document.tsx";

/** The site's telemetry on the server rendering its pages. */
const scope = TelemetryScope.of(import.meta.destack.package, Environment.system.telemetry);

/** Render the requested page inside the site's document. */
export function render() {
    return renderToStream(
        () => (
            <EnvironmentContext value={BrowserEnvironment.server(Environment.system)}>
                <TelemetryContext value={scope}>
                    <DefaultErrorBoundary>
                        <Document>
                            <DefaultErrorBoundary>
                                <App />
                            </DefaultErrorBoundary>
                        </Document>
                    </DefaultErrorBoundary>
                </TelemetryContext>
            </EnvironmentContext>
        ),
        { manifest },
    );
}
