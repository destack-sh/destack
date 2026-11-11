import { none } from "@destack/access";
import type { Table } from "@destack/db";
import { Digest } from "@destack/schema";
import { field } from "../field/field.ts";
import type { MethodBuilder } from "../method/record.ts";
import { defineObject } from "../object/object.ts";
import { expiring } from "../trait/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The spent key proofs of a scope, each stored once by its digest until it expires. */
export const proof = defineObject(
    {
        name: "proof",
        plural: "proofs",
        title: t`${plural(argument("count"), { one: "Proof", other: "Proofs" })}`,
        key: "digest",
        fields: {
            /** The digest of the proof's key thumbprint and identifier, which a reuse repeats. */
            digest: field.string(Digest).immutable(),
            /** When the proof expires, in UTC epoch milliseconds. */
            expiresAt: field.time(),
        },
        access: {
            // let the scope's principal alone spend proofs, as its owner
            permissions: { spend: none(), expire: none() },
        },
        traits: [expiring([{ after: { milliseconds: 0 }, from: "expiresAt" }])],
    },
    <Definition extends Table>(
        method: MethodBuilder<Definition, "digest" | "expiresAt", "spend">,
    ) => ({
        create: method.create("spend", {
            fields: ["digest", "expiresAt"],
            visibility: "installation",
            isPredicted: false,
        }),
    }),
);
