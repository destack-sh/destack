import { principal } from "@destack/access";
import { defineAdapter, definePackage } from "@destack/runtime";
import { module, symbol } from "./graph/object.ts";
import {
    binding,
    branch,
    branchEntry,
    branchChange,
    build,
    capture,
    declaration,
    deployment,
    installation,
    installationKey,
    installationRevision,
    instance,
    invitation,
    proof,
    relationship,
    restore,
    role,
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
        proof,
        declaration,
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
        role,
        relationship,
        invitation,
        module,
        symbol,
    },
    adapters: {
        handoff: defineAdapter("handoff", { path: HANDOFF_PATH, callers: [principal.machine] }),
        hook: defineAdapter("hook", { path: HOOK_PATH, callers: [principal.installation] }),
    },
});
