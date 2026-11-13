import { t } from "@destack/locale";
import { defineMount, definePackage } from "@destack/object";
import { RelayError } from "./error/error.ts";

/** The lifetime of tunnel tokens: ten minutes, Kubernetes' shortest bound service account token. */
export const TUNNEL_TOKEN_LIFETIME_MILLISECONDS = 10 * 60_000;

/** The handle stacks import to install the relay in a residency's home. */
export default definePackage({
    title: t`Relay`,
    description: t`Relay HTTP requests for Destack names to the machines serving them.`,
    icon: "arrows-left-right",
    color: "bronze",
    mounts: [
        defineMount({ name: "tunnels", path: "/machines", callers: "self" }),
        defineMount({ name: "names", path: "/names" }),
    ],
    errors: RelayError,
});
