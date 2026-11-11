import { principal } from "@destack/access";
import { BUILD_STORE_TABLES } from "@destack/build/table";
import { defineMount, definePackage, installation } from "@destack/object";
import { catalog, declaration, module, symbol } from "./object/graph.ts";
import {
    build,
    deployment,
    installationRevision,
    instance,
    restore,
    rollout,
    route,
    snapshot,
    socket,
    transfer,
    reading,
    usage,
} from "./object/index.ts";
import { StackError } from "./error/error.ts";
import { account } from "@destack/account/object";

export { STACK_INSTALLATION } from "./object/rollout.ts";

/** The path of the space-move data plane below the stack package's mount. */
export const HANDOFF_PATH = "/handoff";

/** The path of runners' build uploads below the stack package's mount. */
export const PUSH_PATH = "/push";

/** The path of installations' hook sessions below the stack package's mount. */
export const HOOK_PATH = "/hooks";

/** The handle spaces serve their stack through, and machines and people call it by. */
export default definePackage({
    objects: [
        rollout,
        installationRevision,
        deployment,
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
        reading,
        usage,
    ],
    // the index of the space's builds, beside its objects
    tables: BUILD_STORE_TABLES,
    mounts: [
        defineMount({ name: "handoff", path: HANDOFF_PATH, callers: [principal.machine] }),
        defineMount({ name: "push", path: PUSH_PATH, callers: [principal.machine] }),
        defineMount({ name: "hook", path: HOOK_PATH, callers: [principal.installation] }),
    ],
    access: {
        permissions: {
            // install, change and uninstall the space's declared and installed packages, as their rollouts do
            space: {
                space: [
                    installation.permission("install"),
                    installation.permission("update"),
                    installation.permission("delete"),
                    installation.permission("control"),
                    installation.permission("approve"),
                ],
            },
            // publish a person's builds under their account, asked on their first publish
            optional: { account: [account.permission("publish")] },
        },
    },
    errors: StackError,
});
