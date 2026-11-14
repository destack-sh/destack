import type { Environment } from "@destack/environment";
import {
    accessRelationship,
    anyone,
    none,
    principal,
    Relationship,
    relation,
} from "@destack/access";
import { Scope, type Subject } from "@destack/sync";
import { TestAccess } from "@destack/access/test";
import type { DatabaseConnection } from "@destack/db";
import { Identifier } from "@destack/schema";
import { defineObject, shareable } from "../index.ts";
import { argument, plural, t } from "@destack/locale";

/** The spaces the example objects live in. */
export const space = defineObject({
    name: "space",
    plural: "spaces",
    title: t`${plural(argument("count"), { one: "Space", other: "Spaces" })}`,
    scope: "universe",
    fields: {},
    container: "partition",
    access: {
        relations: { reader: { subjects: [principal.user, anyone.all()] } },
        // let its owner manage it, as an account's space does, whose managers approve its plans
        permissions: { read: relation("reader"), manage: none(), share: none() },
    },
    traits: [shareable({ by: "share" })],
});

/** Reference a space as the principal its server's work runs as. */
export function spacePrincipal(id: string): Subject {
    return principal.space.reference(Scope.universe.id, id);
}

/** Record a space with itself as owner, public unless private. */
export async function openSpace(
    database: DatabaseConnection,
    environment: Pick<Environment, "clock" | "crypto" | "random">,
    id: string,
    visibility: "public" | "private" = "public",
): Promise<void> {
    // record the space's scope, with the space as owner as its creation binds it
    const object = space.reference(Scope.universe.id, id);
    const fixture = new TestAccess(database, environment);
    await fixture.replicateScope(object, space.policy);
    await fixture.replicateOwner(object, spacePrincipal(id), 0);

    // relate anyone to a public space as its reader
    if (visibility === "public") {
        await database.transaction(
            async (transaction) =>
                await transaction.insert(accessRelationship).values(
                    Relationship.encode({
                        id: Identifier.create("relationship", environment),
                        object,
                        relation: "reader",
                        subject: anyone.reference("*", "*"),
                        createdAt: 0,
                        expiresAt: null,
                    }),
                ),
        );
    }
}

/** Refuse reconnecting a client with an unmoved scope. */
export function unmoved(holder: string): never {
    throw new TypeError(`no scope moves in this test, yet one moved to '${holder}'`);
}
