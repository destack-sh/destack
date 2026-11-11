import { defineMount, definePackage } from "@destack/object";
import { RelayError } from "./error/error.ts";

/** The lifetime of tunnel tokens: ten minutes, Kubernetes' shortest bound service account token. */
export const TUNNEL_TOKEN_LIFETIME_MILLISECONDS = 10 * 60_000;

/** The handle stacks import to install the relay in a residency's home. */
export default definePackage({
    mounts: [
        defineMount({ name: "tunnels", path: "/machines", callers: "self" }),
        defineMount({ name: "names", path: "/names" }),
    ],
    errors: RelayError,
});
