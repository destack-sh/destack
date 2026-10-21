import { defineAdapter, definePackage } from "@destack/object";
import { connection, tunnel } from "./object/index.ts";

/** The handle stacks import to install the relay in a residency's home, with its tunnels. */
export default definePackage({
    objects: { tunnel, connection },
    adapters: {
        // the root's calls to each machine's tunnel, from the relay's installation itself
        tunnels: defineAdapter("tunnels", { path: "/machines", callers: "self" }),
    },
});
