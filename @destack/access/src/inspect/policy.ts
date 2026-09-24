import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { AccessName } from "../access/expression.ts";
import { AccessExpressionDescription } from "./object.ts";
import type { AccessPolicy } from "../access/policy.ts";

/** A mandatory condition applied to a package's named permissions. */
export const AccessPolicyDescription: schema.Schema<AccessPolicy> = defineSchema(
    schema.object({
        name: AccessName,
        packageId: PackageId,
        type: AccessName,
        permissions: schema.array(AccessName).min(1),
        scope: schema.string().min(1).optional(),
        effect: schema.enum(["restrict", "forbid"]),
        condition: AccessExpressionDescription,
    }),
);
