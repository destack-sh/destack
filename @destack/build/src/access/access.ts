import { none, Policy } from "@destack/access";
import type {} from "@destack/package/import-meta";

/** The owner of the build package, declaring the policies of its operations. */
const OWNER = import.meta.destack.package;

/** Builds and inspections of a space's source. */
export const build = new Policy(OWNER, {
    name: "build",
    permissions: { start: none(), inspect: none() },
});

/** Previews of a space's source, and the frontends opened in them. */
export const preview = new Policy(OWNER, {
    name: "preview",
    permissions: { read: none(), open: none(), start: none(), stop: none() },
});

/** Every policy of the build package's operations. */
export const BUILD_POLICIES = [build, preview];
