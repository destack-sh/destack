import { none } from "@destack/access";
import type { Table } from "@destack/db";
import { Digest } from "@destack/schema";
import { field } from "../field/field.ts";
import type { MethodBuilder } from "../method/record.ts";
import { defineObject } from "../object/object.ts";

/** The spent key proofs of a scope, each stored by its digest until it expires. */
export const proof = defineObject(
    {
        name: "proof",
        plural: "proofs",
        key: Digest,
        expiring: [{ after: { milliseconds: 0 }, from: "expiresAt" }],
        fields: {
            /** When the proof expires, in UTC epoch milliseconds. */
            expiresAt: field.time(),
        },
        // let the scope's principal alone spend proofs, as its owner
        permissions: { spend: none(), expire: none() },
    },
    <Definition extends Table>(method: MethodBuilder<Definition, "expiresAt", "spend">) => ({
        create: method.create("spend", {
            fields: ["expiresAt"],
            isInternal: true,
            isPredicted: false,
        }),
    }),
);
