import { principal } from "@destack/access";
import { defineAdapter, definePackage } from "@destack/object";
import { catalog, declaration, module, symbol } from "./object/graph.ts";
import {
    binding,
    branch,
    branchEntry,
    branchChange,
    build,
    capture,
    deployment,
    installation,
    installationKey,
    installationRevision,
    instance,
    restore,
    route,
    snapshot,
    socket,
    transfer,
} from "./object/index.ts";

/** The path of the space-move data plane below the stack package's mount. */
export const HANDOFF_PATH = "/handoff";

/** The path of installations' hook sessions below the stack package's mount. */
export const HOOK_PATH = "/hooks";

/** The handle spaces serve their stack through, and machines and people call it by. */
export default definePackage({
    objects: {
        installation,
        installationRevision,
        installationKey,
        binding,
        deployment,
        capture,
        instance,
        transfer,
        snapshot,
        restore,
        build,
        route,
        socket,
        branch,
        branchEntry,
        branchChange,
        module,
        declaration,
        symbol,
        catalog,
    },
    adapters: {
        handoff: defineAdapter("handoff", { path: HANDOFF_PATH, callers: [principal.machine] }),
        hook: defineAdapter("hook", { path: HOOK_PATH, callers: [principal.installation] }),
    },
});
