import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";

import { subjectContext, testCallKey } from "@destack/runtime/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { RequestId } from "@destack/runtime/request";
import type { RequestContext } from "@destack/runtime/server";
import { ObjectStore } from "../src/server/index.ts";
import { notebook, note, notesDatabase } from "./fixture/note.ts";
import { openSpace, spacePrincipal } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";
import { Clock } from "@destack/environment";

test.for(TEST_DIALECTS)(
    "follow the changes one instance writes through another instance on its own connection on %s",
    async (dialect) => {
        // serve one database from two instances with separate connections
        const storage = await TestDatabase.create(dialect, notesDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const east = await serve(storage.database);
        const west = await serve(await storage.connect(notesDatabase));

        // follow the notebooks on the west instance, snapshot first
        const controller = new AbortController();
        onTestFinished(() => controller.abort());
        const pages = west.source.relayed(
            west.source.queriesPublication.subscription({
                name: "objects",
                scope: spaceId,
                below: spaceId,
                parameters: {
                    queries: { notebooks: { object: "notebook" } },
                },
            }),
            context(controller.signal),
        );
        const first = await pages.next();
        if (first.done === true) {
            throw new TypeError("the sync stream ended before its snapshot");
        }
        const snapshot = first.value;

        // write on the east instance and follow on the west
        const created = await east
            .mutate(context(controller.signal).access(), {
                id: RequestId.create(Date.now()),
                calls: [notebook.call("create", { spaceId, name: "Travel" })],
            })
            .then(([written]) => written);
        let names: unknown[] = [];
        for await (const page of pages) {
            names = page.changes
                .filter((change) => change.table === "destack__object__notebook")
                .map((change) => [change.operation, change.row["id"], change.row["name"]]);
            if (names.length > 0) {
                break;
            }
        }
        expect([snapshot.complete, names]).toEqual([true, [["insert", created.id, "Travel"]]]);
    },
);

/** Serve the notes over one connection as alice. */
async function serve(database: DatabaseConnection) {
    return new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([notebook, note]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: {
            package: notebook.package,
        },
    });
}

/** Build alice's request context in the space, ending with the signal. */
function context(signal: AbortSignal): RequestContext {
    return subjectContext(principal.user.reference("universe", users.alice), spaceId, { signal });
}
