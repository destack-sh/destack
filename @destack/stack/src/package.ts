import { principal } from "@destack/access";
import { PACKAGE_STORE_TABLES } from "@destack/build/table";
import { defineAdapter, definePackage, installation } from "@destack/object";
import { catalog, declaration, module, symbol } from "./object/graph.ts";
import {
    binding,
    build,
    capture,
    deployment,
    installationKey,
    installationRevision,
    instance,
    restore,
    rollout,
    route,
    snapshot,
    socket,
    transfer,
} from "./object/index.ts";
import { StackError } from "./error/error.ts";

/** The path of the space-move data plane below the stack package's mount. */
export const HANDOFF_PATH = "/handoff";

/** The path of runners' build uploads below the stack package's mount. */
export const PUSH_PATH = "/push";

/** The path of installations' hook sessions below the stack package's mount. */
export const HOOK_PATH = "/hooks";

/** The handle spaces serve their stack through, and machines and people call it by. */
export default definePackage({
    objects: {
        rollout,
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
        module,
        declaration,
        symbol,
        catalog,
    },
    // the index of the space's builds, beside its objects
    tables: PACKAGE_STORE_TABLES,
    adapters: {
        handoff: defineAdapter("handoff", { path: HANDOFF_PATH, callers: [principal.machine] }),
        push: defineAdapter("push", { path: PUSH_PATH, callers: [principal.machine] }),
        hook: defineAdapter("hook", { path: HOOK_PATH, callers: [principal.installation] }),
    },
    permissions: {
        // install, change and uninstall the space's declared and installed packages, as their rollouts do
        installation: {
            space: [
                installation.permission("install"),
                installation.permission("update"),
                installation.permission("delete"),
            ],
        },
    },
    errors: StackError,
});
