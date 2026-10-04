import { PermissionReference } from "../policy/policy.ts";

/** What a role grants: every permission but reserved ones, or its own and its included roles' permissions. */
export interface RoleGrant {
    /** Whether the role, or a role it includes, grants every permission. */
    readonly isUniversal: boolean;
    /** The permissions the role grants itself or through the roles it includes. */
    readonly permissions: readonly PermissionReference[];
}

/** One role as a scope chain defines it, with the permissions it grants itself. */
export interface RoleDefinition {
    /** The role identifier. */
    readonly id: string;
    /** Whether the role grants every permission but reserved ones. */
    readonly isUniversal: boolean;
    /** The permissions the role grants itself. */
    readonly permissions: readonly PermissionReference[];
}

/** Close each defined role over the defined roles it includes, following only roles the same chain defines. */
export function close(
    roles: readonly RoleDefinition[],
    includes: readonly { readonly role: string; readonly included: string }[],
): Map<string, RoleGrant> {
    // index the defined roles and their inclusions
    const defined = new Map(roles.map((role) => [role.id, role]));
    const included = new Map<string, string[]>();
    for (const entry of includes) {
        included.set(entry.role, [...(included.get(entry.role) ?? []), entry.included]);
    }

    // gather each role's permissions through its inclusions, each role once
    const closure = new Map<string, RoleGrant>();
    for (const role of roles) {
        const permissions = new Map<string, PermissionReference>();
        const visited = new Set<string>();
        let isUniversal = false;
        const pending = [role.id];
        for (let id = pending.pop(); id !== undefined; id = pending.pop()) {
            // visit each defined role once
            const next = defined.get(id);
            if (next === undefined || visited.has(next.id)) {
                continue;
            }
            visited.add(next.id);

            // take its permissions and follow its inclusions
            isUniversal ||= next.isUniversal;
            for (const permission of next.permissions) {
                permissions.set(PermissionReference.key(permission), permission);
            }
            pending.push(...(included.get(next.id) ?? []));
        }
        closure.set(role.id, { isUniversal, permissions: [...permissions.values()] });
    }

    return closure;
}
