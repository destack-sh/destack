/** The servers a cloud machine runs in its process, supplied to its bundle by its deploy. */
declare module "destack:servers" {
    /** The server each shipped package exports, by package name. */
    const servers: Readonly<Record<string, unknown>>;
    export default servers;
}
