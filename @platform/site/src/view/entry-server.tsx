import { renderToStream } from "@destack/web/render";
import manifest from "virtual:solid-manifest";
import { DefaultErrorBoundary } from "virtual:solid-ssr-error-boundary.tsx";
import App from "./app.tsx";
import Document from "./document.tsx";

/** Render the requested page inside the site's document. */
export function render() {
    return renderToStream(
        () => (
            <DefaultErrorBoundary>
                <Document>
                    <DefaultErrorBoundary>
                        <App />
                    </DefaultErrorBoundary>
                </Document>
            </DefaultErrorBoundary>
        ),
        { manifest },
    );
}
