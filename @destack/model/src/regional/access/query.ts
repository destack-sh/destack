import { RoleQuery } from "@destack/access/database";
import { roleBinding } from "./binding.ts";
import { rolePermission } from "./permission.ts";

/** Scoped role grants mapped to the shared authorization evaluator. */
export const roleQuery = new RoleQuery({
    bindingTable: roleBinding,
    permissionTable: rolePermission,
    bindingRole: roleBinding.roleId,
    permissionRole: rolePermission.roleId,
    revokedAt: roleBinding.revokedAt,
    expiresAt: roleBinding.expiresAt,
    packageId: rolePermission.packageId,
    type: rolePermission.type,
    name: rolePermission.name,
    objectId: rolePermission.objectId,
});
