import { none, principal, relation, through, union, ANY_SCOPE } from "@destack/access";
import { claimMethods, claimTable, defineObject } from "@destack/object";
import { account } from "./account.ts";
import { argument, plural, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { AccountHandle } from "../address/handle.ts";

/** A unique name an object claims across databases, stored in the universe. */
export const claim = defineObject(
    {
        name: "claim",
        plural: "claims",
        title: t`${plural(argument("count"), { one: "Claim", other: "Claims" })}`,
        mapping: {
            table: claimTable,
            id: "id",
            scope: "scope",
            attributes: {},
            relations: {
                space: { column: "owner" },
                object: { column: "objectId" },
                account: { column: "owner", scope: ANY_SCOPE },
            },
        },
        scope: "universe",
        access: {
            relations: {
                // the space the name belongs to
                space: { subjects: [principal.space], grantedBy: null },
                // a space claiming its own name
                object: { subjects: [principal.space], grantedBy: null },
                // the account the name belongs to
                account: { subjects: [account.policy], grantedBy: null },
            },
            permissions: {
                read: none(),
                // let a space claim the names it owns, and an account's spaces the account's names
                claim: union(relation("space"), relation("object"), through("account", "claim")),
                resolve: none(),
            },
        },
    },
    (method) => ({
        ...claimMethods,

        /** Find the account holding a handle or a lookalike, and the account paying for it. */
        holder: method.query({
            title: t.context("action")`Read holder`,
            permission: "resolve",
            target: "scope",
            input: schema.object({
                /** The handle. */
                handle: AccountHandle,
            }),
            output: schema
                .object({
                    /** The account holding it, the reserving organization's for a reserved one. */
                    account: schema.identifier("account"),
                    /** The account paying for it, which sells its packages. */
                    payer: schema.identifier("account"),
                })
                .nullable(),
        }),
    }),
);
