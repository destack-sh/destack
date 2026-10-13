import { schema } from "@destack/schema";
import type { MethodBuilder } from "../method/method.ts";
import { method } from "../method/record.ts";
import type { ObjectOf } from "../object/object.ts";
import { Claim, Claimant, ClaimOwner, ClaimRefusal, type claimTable, Expiry } from "./claim.ts";

/** The request whose write reserved an object's names. */
const ReservedBy = schema.object({
    /** The request's identifier. */
    request: schema.string().min(1),
});

/** The method declarations of a claim type. */
const claimMethod: MethodBuilder<typeof claimTable> = method;

/** The methods of a claim type, which stores call to keep their indexed objects' names. */
export const claimMethods = {
    /** Reserve an object's names until its write commits. */
    reserve: claimMethod.mutation({
        permission: "claim",
        target: "none",
        input: Claimant.extend(ReservedBy.shape),
        output: ClaimRefusal,
    }),

    /** Settle an object's names once its write committed, releasing those it dropped. */
    confirm: claimMethod.mutation({
        permission: "claim",
        target: "none",
        input: Claimant.extend(ReservedBy.shape),
        output: schema.object({}),
    }),

    /** Release an object's names a failed write reserved. */
    release: claimMethod.mutation({
        permission: "claim",
        target: "none",
        input: Claimant.pick({ objectId: true, owner: true }).extend(ReservedBy.shape),
        output: schema.object({}),
    }),

    /** Replace an object's names after a write that reserved none. */
    replace: claimMethod.mutation({
        permission: "claim",
        target: "none",
        input: Claimant,
        output: ClaimRefusal,
    }),

    /** Find the object an active name resolves to. */
    owner: claimMethod.query({
        permission: "resolve",
        target: "none",
        input: Claim.pick({ index: true, key: true }),
        output: ClaimOwner.nullable(),
    }),

    /** List the expired reservations of some indexes the caller claims names in. */
    expired: claimMethod.query({
        permission: "resolve",
        target: "none",
        input: schema.object({
            /** The indexes. */
            indexes: schema.array(schema.string().min(1)),
        }),
        output: Expiry,
    }),
};

/** A claim type: a universe object type over the claim table with the claim methods. */
export type ClaimType = ObjectOf<{
    readonly scope: "universe";
    readonly table: typeof claimTable;
    readonly methods: typeof claimMethods;
}>;
