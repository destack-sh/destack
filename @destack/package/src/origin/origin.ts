/** The paths every installation origin reserves below `/.destack`, beside its scope paths. */
export const OriginPath = {
    /** The root of the reserved paths. */
    root: "/.destack",
    /** Open an address, or finish a sign-in the issuer returns to with its code. */
    open: "/.destack/open",
    /** Ask for a permission or a fresh sign-in, then return to the page. */
    request: "/.destack/request",
    /** Register the browser as the person's client and bind the page's session to it. */
    session: "/.destack/session",
    /** Stream a change event once the origin serves another build than the page's. */
    reload: "/.destack/reload",
    /** Take the page's OTLP exports, each signal below at `/v1/<signal>`. */
    telemetry: "/.destack/telemetry",
    /** Serve the build's content-addressed files, each below its digest. */
    build: "/.destack/build",
    /** Serve the files the installation's own server output emits. */
    asset: "/.destack/asset",
    /** Serve an image's modules. */
    module: "/.destack/module",
    /** Show one example or scenario of a development build, which the query gives. */
    example: "/.destack/example",
} as const;
