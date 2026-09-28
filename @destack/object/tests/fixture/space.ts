import {
    accessRelationship,
    anyone,
    GLOBAL_SCOPE,
    none,
    Relationship,
    relation,
} from "@destack/access";
import { copyScope } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { defineObject } from "../../src/index.ts";

/** The spaces the example objects live in. */
export const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: "global",
    isScope: true,
    fields: {},
    relations: { reader: { subjects: [anyone.all()] } },
    permissions: { read: relation("reader"), share: none() },
    shareable: { by: "share" },
});

/** Record a space, public unless private. */
export async function openSpace(
    database: DatabaseConnection,
    id: string,
    visibility: "public" | "private" = "public",
): Promise<void> {
    // record the space's scope
    const object = space.reference(GLOBAL_SCOPE, id);
    await copyScope(database, object);

    // relate anyone to a public space as its reader
    if (visibility === "public") {
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: identifier("relationship").parse(`relationship-${v7()}`),
                    object,
                    relation: "reader",
                    subject: anyone.reference("*", "*"),
                    createdAt: 0,
                    expiresAt: null,
                },
                id,
            ),
        );
    }
}

/** Refuse reconnecting a client whose scope no test moves. */
export function unmoved(holder: string): never {
    throw new TypeError(`no scope moves in this test, yet one moved to ${holder}`);
}
