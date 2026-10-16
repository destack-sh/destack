import { schema } from "@destack/schema";

import { MethodBuilder } from "../method/record.ts";
import type { ObjectOf } from "../object/object.ts";
import { Execution } from "../method/execution.ts";
import {
    Claim,
    Claimant,
    ClaimOwner,
    ClaimRefusal,
    type ClaimRow,
    claimTable,
    Expiry,
} from "./claim.ts";

/** The request whose write reserved an object's names. */
const ReservedBy = schema.object({
    /** The request's identifier. */
    request: schema.string().min(1),
});

/** The method declarations of a claim type. */
const claimMethod = new MethodBuilder<typeof claimTable>();

/** The methods of a claim type, which stores call to settle their indexed objects' names. */
export const claimMethods = {
    /** Reserve an object's names until its write commits. */
    reserve: Execution.handle(
        claimMethod.mutation({
            permission: "claim",
            target: "scope",
            input: Claimant.extend(ReservedBy.shape),
            output: ClaimRefusal,
        }),
        async (call, execution) => {
            // reserve the free names in the store's transaction
            const { request, ...claimant } = call.input;
            const { database } = Execution.store(execution, call).transaction;

            return { refused: await Claim.reserve(database, claimant, request, call.now) };
        },
    ),

    /** Settle an object's names once its write committed, releasing those it dropped. */
    confirm: Execution.handle(
        claimMethod.mutation({
            permission: "claim",
            target: "scope",
            input: Claimant.extend(ReservedBy.shape),
            output: schema.object({}),
        }),
        async (call, execution) => {
            // settle the names the committed write has
            const { request, ...claimant } = call.input;
            const { database } = Execution.store(execution, call).transaction;
            await Claim.confirm(database, claimant, request);

            return {};
        },
    ),

    /** Release an object's names a failed write reserved. */
    release: Execution.handle(
        claimMethod.mutation({
            permission: "claim",
            target: "scope",
            input: Claimant.pick({ objectId: true, owner: true }).extend(ReservedBy.shape),
            output: schema.object({}),
        }),
        async (call, execution) => {
            // release the names the failed write reserved
            const { request, ...object } = call.input;
            const { database } = Execution.store(execution, call).transaction;
            await Claim.release(database, object, request);

            return {};
        },
    ),

    /** Replace an object's names after a write that reserved none. */
    replace: Execution.handle(
        claimMethod.mutation({
            permission: "claim",
            target: "scope",
            input: Claimant,
            output: ClaimRefusal,
        }),
        async (call, execution) => {
            // replace the names, refusing those others claim
            const { database } = Execution.store(execution, call).transaction;

            return { refused: await Claim.replace(database, call.input) };
        },
    ),

    /** Find the object an active name resolves to. */
    owner: Execution.handle(
        claimMethod.query({
            permission: "resolve",
            target: "scope",
            input: Claim.pick({ index: true, key: true }),
            output: ClaimOwner.nullable(),
        }),
        async (call) => (await Claim.owner(call.read(claimTable), call.input)) ?? null,
    ),

    /** List the expired reservations of some indexes the caller claims names in. */
    expired: Execution.handle(
        claimMethod.query({
            permission: "resolve",
            target: "scope",
            input: schema.object({
                /** The indexes. */
                indexes: schema.array(schema.string().min(1)),
            }),
            output: Expiry,
        }),
        async (call, execution) => {
            // filter the reservations the caller may claim names in
            const { authorization } = Execution.store(execution, call);
            const filter = async (rows: readonly ClaimRow[]) =>
                (await authorization.filter(call.object, call.object.permission("claim"), rows))
                    .rows;

            return Claim.expired(call.read(claimTable), call.input.indexes, call.now, filter);
        },
    ),
};

/** A claim type: a universe object type over the claim table with the claim methods. */
export type ClaimType = ObjectOf<{
    readonly scope: "universe";
    readonly table: typeof claimTable;
    readonly methods: typeof claimMethods;
}>;
