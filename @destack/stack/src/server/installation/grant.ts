import {
    PermissionReference,
    PermissionRequest,
    type PermissionScope,
    principal,
} from "@destack/access";
import type { MethodContext } from "@destack/object";
import { type WorkloadDescription } from "@destack/package/manifest";
import { type Installation, role } from "../../object/index.ts";

/** The name managing an installation's grant, beside the declarations of its stack. */
const GRANT_NAME = "permissions";

/** The grant an installation has in its space. */
export const InstallationGrant = {
    /** List the permissions some workloads request in a scope, each once. */
    requested(
        workloads: readonly Pick<WorkloadDescription, "permissions">[],
        scope: PermissionScope,
    ): PermissionReference[] {
        const requested = workloads.flatMap(
            (workload) => PermissionRequest.parse(workload.permissions)[scope] ?? [],
        );
        const keyed = new Map(
            requested.map((permission) => [PermissionReference.key(permission), permission]),
        );

        return [...keyed.values()];
    },

    /** Keep the installation's role in its space granting exactly the permissions. */
    async keep(
        context: MethodContext,
        target: Installation,
        permissions: readonly PermissionReference[],
    ): Promise<void> {
        // keep the role with exactly the permissions, bound to the installation
        const manager = {
            installationId: target.id,
            packageId: target.packageId,
            name: GRANT_NAME,
        };
        await context.mutate(context.self, [
            role.call("keep", {
                ...role.in(target.scope),
                name: `installation/${target.id}`,
                description: `What ${target.name} requests in this space`,
                permissions: [...permissions],
                subject: principal.installation.reference(target.scope, target.id),
                manager,
            }),
        ]);
    },
};
