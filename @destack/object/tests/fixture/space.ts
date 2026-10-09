import {
    accessRelationship,
    anyone,
    none,
    principal,
    Relationship,
    relation,
} from "@destack/access";
import { Scope, type Subject } from "@destack/sync";
import { AccessFixture } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import { schema } from "@destack/schema";
import { v7 } from "uuid";
import { defineObject } from "../../src/index.ts";

/** The spaces the example objects live in. */
export const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: "universe",
    isScope: true,
    fields: {},
    relations: { reader: { subjects: [anyone.all()] } },
    permissions: { read: relation("reader"), share: none() },
    shareable: { by: "share" },
});

/** Reference a space as the principal its server's own work runs as. */
export function spacePrincipal(id: string): Subject {
    return principal.space.reference(Scope.universe.id, id);
}

/** Record a space owned by itself, public unless private, returning its owner role. */
export async function openSpace(
    database: DatabaseConnection,
    id: string,
    visibility: "public" | "private" = "public",
): Promise<string> {
    // record the space's scope, owned by the space as its creation binds it
    const object = space.reference(Scope.universe.id, id);
    const fixture = new AccessFixture(database);
    await fixture.copyScope(object);
    const owner = await fixture.copyOwner(object, spacePrincipal(id), 0);

    // relate anyone to a public space as its reader
    if (visibility === "public") {
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
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

    return owner;
}

/** Refuse reconnecting a client with an unmoved scope. */
export function unmoved(holder: string): never {
    throw new TypeError(`no scope moves in this test, yet one moved to ${holder}`);
}
