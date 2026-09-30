import { AuditOutbox } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Journal } from "@destack/service/database";
import { subjectContext } from "@destack/service/test";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import type { QueryPage } from "@destack/sync";
import { ObjectServer } from "../src/server/index.ts";
import { notebook, note, notesDatabase, notesJournal } from "./fixture/notes.ts";
import { openSpace } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";

test.for(TEST_DIALECTS)(
    "follow the changes one instance writes through another instance on its own connection on %s",
    async (dialect) => {
        // serve one database from two instances with separate connections
        const storage = await TestDatabase.create(dialect, notesDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const east = serve(storage.database);
        const west = serve(await storage.connect(notesDatabase));

        // follow the notebooks on the west instance, snapshot first
        const controller = new AbortController();
        onTestFinished(() => controller.abort());
        const pages = west.sync(spaceId, context(controller.signal), {
            queries: { notebooks: { object: "notebook" } },
        });
        const snapshot = (await pages.next()).value as QueryPage;

        // write on the east instance and follow on the west
        const created = (await east.call(
            notebook,
            "create",
            { spaceId, requestId: RequestId.create(), name: "Travel" },
            context(controller.signal),
        )) as { id: string };
        let names: unknown[] = [];
        for await (const page of pages) {
            names = page.changes
                .filter((change) => change.table === "destack__object__notebook")
                .map((change) => [change.operation, change.row.id, change.row.name]);
            if (names.length > 0) {
                break;
            }
        }
        expect([snapshot.complete, names]).toEqual([true, [["insert", created.id, "Travel"]]]);
    },
);

/** Serve the notes over one connection as alice. */
function serve(database: DatabaseConnection) {
    return new ObjectServer({
        objects: { notebook, note },
        database,
        context: () => ({
            subjects: [principal.user.reference("universe", "alice")],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(notesJournal),
        audit: AuditRecorder.service(new AuditOutbox(database), {
            package: notebook.package,
            service: "test",
        }),
    });
}

/** Build alice's request context in the space, ending with the signal. */
function context(signal: AbortSignal): ServiceContext {
    return subjectContext(principal.user.reference("universe", "alice"), spaceId, signal);
}
