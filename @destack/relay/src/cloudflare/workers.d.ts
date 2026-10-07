/** The module workerd provides its runtime classes in. */
declare module "cloudflare:workers" {
    /** The base of a Durable Object whose public methods its stubs call over RPC, taking its state and bindings. */
    export const DurableObject: abstract new (state: unknown, environment: unknown) => object;
}
