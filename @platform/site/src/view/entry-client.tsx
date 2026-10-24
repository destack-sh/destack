import { BrowserEnvironment } from "@destack/environment/browser";
import { TelemetryScope } from "@destack/telemetry";
import { DevtoolsExporter, startTelemetry } from "@destack/telemetry/browser";
import { EnvironmentContext, TelemetryContext } from "@destack/view";
import { hydrate } from "@destack/web/render";
import { DefaultErrorBoundary } from "virtual:solid-ssr-error-boundary.tsx";
import type {} from "@destack/package/import-meta";
import App from "./app.tsx";
import Document from "./document.tsx";

// write the site's records to the developer tools
// TODO #Incomplete: wire up to .. monitor? service
/** The page's telemetry, exporting to the developer tools. */
const telemetry = await startTelemetry(
    new DevtoolsExporter().options(import.meta.destack.package),
    BrowserEnvironment.system,
    window,
);
/** The page's environment: the browser's effects, recording through the page's telemetry. */
const environment = { ...BrowserEnvironment.system, telemetry };

/** The site's telemetry scope in the page. */
const scope = TelemetryScope.of(import.meta.destack.package, telemetry);

// hydrate the server-rendered document
hydrate(
    () => (
        <EnvironmentContext value={environment}>
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
    document,
);
