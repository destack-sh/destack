import { t } from "@destack/locale";
import { defineMount, definePackage } from "@destack/object";
import { directoryShape, identityShape, regionShape } from "@destack/account/object";
import { RelayError } from "./error/error.ts";

/** The relay each region's managed machine hosts: names and self-hosted machines' tunnels. */
export default definePackage({
    title: t.context("title")`Relay`,
    description: t`Relay HTTP requests for Destack names to the machines serving them.`,
    icon: "arrows-left-right",
    color: "brown",
    mounts: [
        defineMount({ name: "tunnels", path: "/machines", callers: "self" }),
        defineMount({ name: "names", path: "/names" }),
    ],
    // follow the universe's directory, identities and regions, which route and verify tunnels
    follows: [directoryShape, identityShape, regionShape],
    errors: RelayError,
});
