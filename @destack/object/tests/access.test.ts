import { AuditOutbox } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { Authorization, principal, type AccessContext } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import type { QueryPage } from "@destack/sync";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import { Bookmark } from "@destack/service/bookmark";
import type { ServiceContext } from "@destack/service/server";
import { ObjectServer } from "../src/server/index.ts";
import { defineObject, Intrinsic } from "../src/index.ts";
import { request } from "./schema.ts";
import { space } from "./fixture/space.ts";

/** The space whose roles and members the test follows. */
const SPACE_ID = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

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
                tables: [...role.tables, space.table, request],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const database = storage.database;

        // create the space, owned by the owner, in the database holding it
        const owner = principal.user.reference("universe", "owner");
        const object = space.reference("universe", SPACE_ID);

        // serve the space's roles and relationships to the owner
        const server = new ObjectServer({
            objects: { role, relationship },
            policies: [space],
            database,
            context: (): AccessContext => ({ subjects: [owner], now: Date.now(), attributes: {} }),
            journal: new Journal(request),
            audit: AuditRecorder.service(new AuditOutbox(database), {
                package: role.package,
                service: "test",
            }),
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
        const context = {
            scope: SPACE_ID,
            caller: { id: "owner" },
            requireCaller: () => ({ id: "owner" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
            signal: controller.signal,
            request: new Request("https://test.local", { signal: controller.signal }),
        } as unknown as ServiceContext;

        // hold the owner role and its binding from the snapshot
        const pages = server.sync(SPACE_ID, context, {
            queries: {
                roles: { object: "role" },
                members: { object: "relationship" },
            },
        });
        const next = async () => {
            let page = (await pages.next()).value as QueryPage;
            while (page.changes.length === 0) {
                page = (await pages.next()).value as QueryPage;
            }

            // name each role by its name, and each relationship by its subject
            return page.changes.map((change) => [
                change.table,
                change.operation,
                change.row.name ?? change.row.subjectId,
            ]);
        };
        expect(await next()).toEqual([
            ["destack__access__role", "insert", "owner"],
            ["destack__access__relationship", "insert", "owner"],
        ]);

        // hold a new role at once, then its binding to a member
        const editor = (await server.call(
            role,
            "create",
            {
                spaceId: SPACE_ID,
                requestId: RequestId.create(),
                name: "editor",
                description: "Edits the space",
                permissions: [],
            },
            context,
        )) as { id: string };
        expect(await next()).toEqual([["destack__access__role", "insert", "editor"]]);
        const authorization = await server.authorize(database, SPACE_ID, context);
        await authorization.grant({
            object,
            role: editor.id,
            subject: principal.user.reference("universe", "member"),
        });
        expect(await next()).toEqual([["destack__access__relationship", "insert", "member"]]);
    },
);
