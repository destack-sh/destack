import { defineAdapter, definePackage } from "@destack/object";
import { RelayError } from "./error/error.ts";

/** The handle stacks import to install the relay in a residency's home. */
export default definePackage({
    adapters: {
        // the calls between the relay's root and each machine's tunnel partition, from the relay itself
        tunnels: defineAdapter("tunnels", { path: "/machines", callers: "self" }),
    },
    errors: RelayError,
});
