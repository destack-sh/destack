import { AuditOutbox } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Bookmark } from "@destack/service/bookmark";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { v7 } from "uuid";
import type { ObjectType } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { notebook, note, notesDatabase, notesJournal } from "./fixture/notes.ts";
import { openSpace } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";
import { testJournalKey } from "@destack/service/test";

test.for(TEST_DIALECTS)(
    "execute each kind of mutation within its statement budget on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, notesDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const server = new ObjectServer({
            objects: { notebook, note },
            database: storage.database,
            context: () => ({
                subjects: [principal.user.reference("universe", "alice")],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(notesJournal, testJournalKey),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: notebook.package,
                service: "test",
            }),
        });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: "alice" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const book = (await server.call(
            notebook,
            "create",
            { spaceId, requestId: RequestId.create(), name: "Travel" },
            context,
        )) as { id: string };

        // count the statements each mutation runs, its transaction included
        const counted = async (
            input: Record<string, unknown>,
            object: ObjectType = note,
            name = "create",
        ) => {
            const before = storage.database.state.statements;
            await server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            );

            return storage.database.state.statements - before;
        };
        // run one statement more on PostgreSQL for the fence lock
        const lock = dialect === "postgresql" ? 1 : 0;
        expect({
            create: await counted({ parentId: book.id, title: "Ideas" }),
            chosen: await counted({ parentId: book.id, title: "Plan", id: `note-${v7()}` }),
            update: await counted({ id: book.id, name: "Trips" }, notebook, "update"),
        }).toEqual({ create: 15 + lock, chosen: 16 + lock, update: 14 + lock });
    },
);
