import * as access from "@destack/access";
import { accessGrant, Grant, GrantRequest } from "@destack/access";
import { type MethodBuilder } from "../method/record.ts";
import { defineObject } from "../object/object.ts";
import { Execution } from "../method/execution.ts";
import { ObjectPolicy } from "../object/access.ts";
import { defineShape } from "../replica/shape.ts";
import { schema } from "@destack/schema";

/** The grants of a scope: what its principal lets delegates do for it. */
export const grant = defineObject(
    {
        name: "grant",
        plural: "grants",
        mapping: ObjectPolicy.readMapping(accessGrant),
        represents: access.grant,
        permissions: access.grant.definition.permissions,
    },
    (method: MethodBuilder<typeof accessGrant>) => ({
        create: Execution.handle(
            method.create("create", { fields: [], input: GrantRequest, isPredicted: false }),
            async (call, execution) =>
                Grant.create(
                    await ObjectPolicy.scopeObject(call, execution),
                    call.input,
                    Execution.store(execution, call).authorization,
                ),
        ),
        revoke: Execution.handle(
            method.delete("revoke", { isPredicted: false }),
            async (call, execution) => {
                // revoke the grant as its delegator
                await Grant.revoke(
                    await ObjectPolicy.scopeObject(call, execution),
                    call.target.id,
                    Execution.store(execution, call).authorization,
                );

                return {};
            },
        ),
    }),
);

/** The grants delegated to one principal, which it follows to act for their delegators. */
export const grantShape = defineShape({
    name: "grant",
    parameters: schema.object({
        /** The package of the delegate's type. */
        delegatePackageId: schema.string(),
        /** The delegate's type. */
        delegateType: schema.string(),
        /** The scope the delegate lives in. */
        delegateScope: schema.string(),
        /** The delegate's identifier. */
        delegateId: schema.string(),
    }),
    objects: [grant],
    queries: (delegate) => [grant.query({ scopes: "every", where: { ...delegate } })],
});
