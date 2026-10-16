import { ANY_SCOPE, none, through, union } from "@destack/access";
import { claimMethods, claimTable, defineObject, defineShape, INTRINSIC } from "@destack/object";
import { schema } from "@destack/schema";
import { endpoint } from "./endpoint.ts";
import { identity, operation } from "./identity.ts";
import { key } from "./key.ts";
import { account } from "./account.ts";
import { machine } from "./machine.ts";
import { placement } from "./placement.ts";

/** A unique name an object claims across databases, stored in the universe. */
export const claim = defineObject(
    {
        name: "claim",
        plural: "claims",
        [INTRINSIC]: {
            table: claimTable,
            mapping: {
                table: claimTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {
                    space: { column: "owner", scope: ANY_SCOPE },
                    object: { column: "objectId", scope: ANY_SCOPE },
                    account: { column: "owner", scope: ANY_SCOPE },
                },
            },
        },
        scope: "universe",
        relations: {
            // the placement of the space owning the name
            space: { subjects: [placement.policy], grantedBy: null },
            // the placement of a space claiming its own name
            object: { subjects: [placement.policy], grantedBy: null },
            // the account owning the name
            account: { subjects: [account.policy], grantedBy: null },
        },
        permissions: {
            read: none(),
            // let the machines serving spaces and the residencies' installations claim names
            claim: union(
                through("space", "serve"),
                through("object", "serve"),
                through("account", "claim"),
            ),
            resolve: none(),
        },
    },
    () => claimMethods,
);

/** One claimed name, by its index and key, which a store follows on demand. */
export const claimShape = defineShape({
    name: "claim",
    parameters: schema.object({ index: schema.string().min(1), key: schema.string().min(1) }),
    objects: [claim],
    queries: (claimed) => [
        claim.query({ scopes: "every", where: { index: claimed.index, key: claimed.key } }),
    ],
});

/** The directory replicated to machines and routers: placements, names, machines and identities. */
export const directoryShape = defineShape({
    name: "directory",
    parameters: schema.object({}),
    objects: [placement, claim, machine, key, endpoint, identity, operation],
    queries: () => [
        placement.query({ scopes: "every" }),
        claim.query({ scopes: "every" }),
        machine.query({ scopes: "every" }),
        // replicate the keys of machines alone, leaving out people's clients
        key.query({
            scopes: "every",
            where: { parentPackageId: machine.typeReference.packageId, parentType: machine.name },
        }),
        endpoint.query({ scopes: "every" }),
        identity.query({ scopes: "every" }),
        operation.query({ scopes: "every" }),
    ],
});
