import { DevtoolsExporter, startTelemetry } from "@destack/telemetry/browser";
import { hydrate } from "@destack/web/render";
import { DefaultErrorBoundary } from "virtual:solid-ssr-error-boundary.tsx";
import type {} from "@destack/package/import-meta";
import App from "./app.tsx";
import Document from "./document.tsx";

// write the site's records to the developer tools
// TODO #Incomplete: wire up to .. monitor? service
void startTelemetry(new DevtoolsExporter().options(import.meta.destack.package));

// hydrate the server-rendered document
hydrate(
    () => (
        <DefaultErrorBoundary>
            <Document>
                <DefaultErrorBoundary>
                    <App />
                </DefaultErrorBoundary>
            </Document>
        </DefaultErrorBoundary>
    ),
    document,
);
