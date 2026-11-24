import { t } from "@destack/locale";
import { defineMount, definePackage } from "@destack/object";
import {
    directoryShape,
    identityShape,
    residencyShape,
    residentsShape,
} from "@destack/account/object";
import { RelayError } from "./error/error.ts";

/** The handle stacks import to install the relay in a residency's home. */
export default definePackage({
    title: t`Relay`,
    description: t`Relay HTTP requests for Destack names to the machines serving them.`,
    icon: "arrows-left-right",
    color: "brown",
    mounts: [
        defineMount({ name: "tunnels", path: "/machines", callers: "self" }),
        defineMount({ name: "names", path: "/names" }),
    ],
    // follow the universe's directory, identities and residencies, and the residency's accounts, which route and verify tunnels
    follows: [directoryShape, identityShape, residencyShape, residentsShape],
    errors: RelayError,
});
