import { expect, onTestFinished, test } from "@destack/test";
import { Authorization, principal } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { subjectContext, testCallKey } from "@destack/service/test";
import { RequestId } from "@destack/service/request";
import { ObjectServer } from "../src/server/index.ts";
import { defineObject, Intrinsic } from "../src/index.ts";
import { space } from "./fixture/space.ts";

/** The space whose roles and members the test follows. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** A space's roles. */
const role = defineObject(Intrinsic.role(space));
/** A space's relationships. */
const relationship = defineObject(Intrinsic.relationship(space));

test.for(TEST_DIALECTS)(
    "follow a space's roles and members as the owner defines and binds them on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "main",
                tables: [...role.tables, space.table, journal],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const database = storage.database;

        // reference the owner and the space it owns
        const owner = principal.user.reference("universe", "owner");
        const object = space.reference("universe", SPACE_ID);

        // serve the space's roles and relationships to the owner
        const server = new ObjectServer({
            objects: { role, relationship },
            policies: [space],
            database,
            callKey: testCallKey,
            origin: {
                package: role.package,
                service: "test",
            },
        });
        await database
            .insert(space.table)
            .values({ id: SPACE_ID, scope: "universe", createdAt: 1, updatedAt: 1 });
        await new Authorization(server.authorizer, database, () => ({
            subjects: [owner],
            now: Date.now(),
            attributes: {},
        })).create(object, { owner });
        const controller = new AbortController();
        onTestFinished(() => controller.abort());
        const context = subjectContext(principal.user.reference("universe", "owner"), SPACE_ID, {
            signal: controller.signal,
        });

        // keep the owner role and its binding from the snapshot
        const pages = server.source.relayed(
            server.source.queriesShape.subscription({
                name: "objects",
                scope: SPACE_ID,
                below: SPACE_ID,
                parameters: {
                    queries: {
                        roles: { object: "role" },
                        members: { object: "relationship" },
                    },
                },
            }),
            context,
        );
        const next = async () => {
            // skip the pages without changes
            let read = await pages.next();
            while (read.done !== true && read.value.changes.length === 0) {
                read = await pages.next();
            }
            if (read.done === true) {
                throw new TypeError("the sync stream ended before a change");
            }
            const page = read.value;

            // name each role by its name, and each relationship by its subject
            return page.changes.map((change) => [
                change.table,
                change.operation,
                change.row["name"] ?? change.row["subjectId"],
            ]);
        };
        expect(await next()).toEqual([
            ["destack__access__role", "insert", "owner"],
            ["destack__access__relationship", "insert", "owner"],
        ]);

        // keep a new role at once and its binding to a member
        const reviewer = await server.call(
            role,
            "create",
            {
                spaceId: SPACE_ID,
                requestId: RequestId.create(),
                name: "reviewer",
                description: "Reviews the space",
                permissions: [],
            },
            context,
        );
        expect(await next()).toEqual([["destack__access__role", "insert", "reviewer"]]);
        const authorization = await server.authorize(database, SPACE_ID, context);
        await authorization.grant({
            object,
            role: reviewer.id,
            subject: principal.user.reference("universe", "member"),
        });
        expect(await next()).toEqual([["destack__access__relationship", "insert", "member"]]);
    },
);
