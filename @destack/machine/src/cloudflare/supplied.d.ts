/** The servers a cloud machine runs in its process, supplied to its bundle by its deploy. */
declare module "destack:servers" {
    /** The server each shipped package exports, by package name. */
    const servers: Readonly<Record<string, unknown>>;
    export default servers;
}

/** The views starts a cloud machine runs in its process, supplied to its bundle by its deploy. */
declare module "destack:views" {
    /** The views start each shipped package exports, by package name. */
    const views: Readonly<Record<string, unknown>>;
    export default views;
}
