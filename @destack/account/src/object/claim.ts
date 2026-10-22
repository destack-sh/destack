import { ANY_SCOPE, none, through, union } from "@destack/access";
import { claimMethods, claimTable, defineObject } from "@destack/object";
import { account } from "./account.ts";
import { placement } from "./placement.ts";

/** A unique name an object claims across databases, stored in the universe. */
export const claim = defineObject(
    {
        name: "claim",
        plural: "claims",
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
        scope: "universe",
        relations: {
            // the placement of the space the name belongs to
            space: { subjects: [placement.policy], grantedBy: null },
            // the placement of a space claiming its name
            object: { subjects: [placement.policy], grantedBy: null },
            // the account the name belongs to
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
