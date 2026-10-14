import { expect, onTestFinished, test } from "@destack/test";
import type { Subject } from "@destack/sync";
import { Snapshot } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { v7 } from "uuid";
import { schema } from "@destack/schema";
import { accessRelationship, accessRole, principal, Relationship } from "../index.ts";
import { account, group, node, space } from "../test/fixture.ts";
import { AccessFixture } from "../test/access.ts";
import { openFixture } from "../test/database.ts";

/** The time every caller acts at. */
const NOW = 1000;

/** Reference a user in the universe. */
function user(id: string): Subject {
    return principal.user.reference("universe", id);
}

test.for(TEST_DIALECTS)(
    "relate a caller to a scope through a direct, wildcard or subject-set share or a role bound on the scope, but not through an expired share or none, on %s",
    async (dialect) => {
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const { database, authorizer } = fixture;
        const snapshot = Snapshot.live(database);
        const note = node.reference("personal", "a");

        // keep the personal space below its account, as every scope has its row
        const replicas = new AccessFixture(database);
        const personal = space.reference("account-1", "personal");
        await replicas.replicateScope(account.reference("universe", "account-1"));
        await replicas.replicateScope(personal, "own");

        // define a role in the personal space to bind on the space itself
        const roleId = schema.identifier("role").parse("role-01996ab0-0000-7000-8000-000000000301");
        await database.insert(accessRole).values({
            id: roleId,
            createdAt: 1,
            updatedAt: 1,
            scope: "personal",
            name: "editor",
            description: "Edits the space",
        });

        // share a note of the personal space, or bind the role on the space, deciding the caller's relation after each
        const relatedAfter = async (
            subject: Subject,
            caller: string,
            expiresAt: number | null = null,
            isBinding = false,
        ) => {
            // share the note with the subject, or bind the role on the space to it
            const row = Relationship.encode(
                isBinding
                    ? {
                          id: `relationship-${v7()}`,
                          object: personal,
                          role: roleId,
                          subject,
                          createdAt: 1,
                          expiresAt,
                      }
                    : {
                          id: `relationship-${v7()}`,
                          object: note,
                          relation: "viewer",
                          subject,
                          createdAt: 1,
                          expiresAt,
                      },
                "personal",
            );
            await database.insert(accessRelationship).values(row);

            // decide the caller's relation, then take the share back
            const access = await authorizer.resolve(snapshot, "personal", {
                subjects: [user(caller)],
                now: NOW,
                attributes: {},
            });
            const isRelated = await authorizer.isRelatedIn(snapshot, "personal", access);
            await database.remove(accessRelationship, [row]);

            return isRelated;
        };

        // let carol be a member of a group in the universe, outside the personal scope
        const team = group.reference("universe", "team");
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: `relationship-${v7()}`,
                    object: team,
                    relation: "member",
                    subject: user("carol"),
                    createdAt: 1,
                    expiresAt: null,
                },
                team.scope,
            ),
        );

        // relate through each share, and through none for an expired share or a stranger
        expect({
            direct: await relatedAfter(user("carol"), "carol"),
            wildcard: await relatedAfter(principal.user.reference("universe", "*"), "dave"),
            set: await relatedAfter({ ...team, relation: "member" }, "carol"),
            role: await relatedAfter({ ...team, relation: "member" }, "carol", null, true),
            expired: await relatedAfter(user("carol"), "carol", 500),
            stranger: await relatedAfter(user("carol"), "erin"),
        }).toEqual({
            direct: true,
            wildcard: true,
            set: true,
            role: true,
            expired: false,
            stranger: false,
        });
    },
);
