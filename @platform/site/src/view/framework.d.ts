/** The client asset manifest the framework plugin generates. */
declare module "virtual:solid-manifest" {
    import type { AssetManifest } from "@destack/web/render";

    /** The client asset manifest of the build, keyed by source path. */
    const manifest: AssetManifest;
    export default manifest;
}

/** The error boundary the framework plugin generates around the document and the application. */
declare module "virtual:solid-ssr-error-boundary.tsx" {
    import type { JSX, ParentProps } from "@destack/view";

    /** Render a failure in place of its children, answering with status 500 on the server. */
    export function DefaultErrorBoundary(properties: ParentProps): JSX.Element;
}
