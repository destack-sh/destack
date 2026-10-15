import { PermissionReference } from "../declare/policy.ts";
import { Role } from "./role.ts";

/** What a role grants: all but reserved permissions, or its own and included roles' ones. */
export interface RoleGrant {
    /** Whether the role is the owner role or includes it, granting all but reserved permissions. */
    readonly isOwner: boolean;
    /** The permissions the role grants itself or through the roles it includes. */
    readonly permissions: readonly PermissionReference[];
}

/** Role grants, closed over the roles each role includes. */
export const RoleGrant = {
    /** Close each role over its included roles, following only roles of the same chain. */
    close(
        roles: readonly Pick<Role, "id" | "permissions">[],
        includes: readonly { readonly role: string; readonly included: string }[],
    ): Map<string, RoleGrant> {
        // index the defined roles and their inclusions
        const defined = new Map<string, (typeof roles)[number]>(
            roles.map((role) => [role.id, role]),
        );
        const included = new Map<string, string[]>();
        for (const entry of includes) {
            included.set(entry.role, [...(included.get(entry.role) ?? []), entry.included]);
        }

        // gather each role's permissions through its inclusions, each role once
        const closure = new Map<string, RoleGrant>();
        for (const role of roles) {
            const permissions = new Map<string, PermissionReference>();
            const visited = new Set<string>();
            let isOwner = false;
            const pending: string[] = [role.id];
            for (let id = pending.pop(); id !== undefined; id = pending.pop()) {
                // visit each defined role once
                const next = defined.get(id);
                if (next === undefined || visited.has(next.id)) {
                    continue;
                }
                visited.add(next.id);

                // take its permissions and follow its inclusions
                isOwner ||= next.id === Role.OWNER;
                for (const permission of next.permissions) {
                    permissions.set(PermissionReference.key(permission), permission);
                }
                pending.push(...(included.get(next.id) ?? []));
            }
            closure.set(role.id, { isOwner, permissions: [...permissions.values()] });
        }

        return closure;
    },
};
