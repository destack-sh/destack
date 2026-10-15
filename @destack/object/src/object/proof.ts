import { none } from "@destack/access";
import type { Table } from "@destack/db";
import { Digest } from "@destack/schema";
import { field } from "../field/field.ts";
import type { MethodBuilder } from "../method/record.ts";
import type { ObjectScope } from "./object.ts";

/** A scope's spent key proofs, which a store verifying proofs declares, kept until they expire. */
export const proof = {
    /** Define a scope's spent proofs, each kept by its digest until it expires. */
    definition<const Scope extends ObjectScope>(scope: Scope) {
        return {
            name: "proof",
            plural: "proofs",
            scope,
            key: Digest,
            expiring: [{ after: { milliseconds: 0 }, from: "expiresAt" }],
            fields: {
                /** When the proof expires, in UTC epoch milliseconds. */
                expiresAt: field.time(),
            },
            // let the scope's own principal alone spend proofs, as its owner
            permissions: { spend: none(), expire: none() },
        } as const;
    },

    /** Declare the proofs' one method: spending a proof once, refusing one spent before. */
    methods: <Definition extends Table>(
        method: MethodBuilder<Definition, "expiresAt", "spend">,
    ) => ({
        create: method.create("spend", {
            fields: ["expiresAt"],
            isInternal: true,
            isPredicted: false,
        }),
    }),
};
