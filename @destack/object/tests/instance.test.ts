import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { type DatabaseConnection, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";

import { subjectContext, testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { RequestId, Scope } from "@destack/sync";
import type { RequestContext } from "@destack/object/server";
import { notebook, note, notesDatabase } from "./fixture/note.ts";
import { openSpace, spacePrincipal, space } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";
import { TestObjectStore } from "../src/test/index.ts";

test("follow the changes one instance writes through another instance on a separate connection", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve one database from two instances with separate connections
    const storage = await TestDatabase.create(notesDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const east = await serve(world, storage.database);
    const west = await serve(world, await storage.connect(notesDatabase));

    // follow the notebooks on the west instance, snapshot first
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const pages = west.source.relayed(
        west.source.queriesPublication.subscription({
            name: "objects",
            scope: spaceId,
            target: spaceId,
            parameters: {
                queries: { notebooks: { object: "notebook" } },
            },
        }),
        context(controller.signal, world),
    );
    const first = await pages.next();
    if (first.done === true) {
        throw new TypeError("the sync stream ended before its snapshot");
    }
    const snapshot = first.value;

    // write on the east instance and follow on the west
    const created = await east
        .mutate(context(controller.signal, world).access(), {
            id: RequestId.create(world),
            calls: [notebook.call("create", { spaceId, name: "Travel" })],
        })
        .then(([written]) => written);
    let names: unknown[] = [];
    for await (const page of pages) {
        names = page.changes
            .filter((change) => change.table === notebook.table[TABLE].sqlName)
            .map((change) => [change.operation, change.row["id"], change.row["name"]]);
        if (names.length > 0) {
            break;
        }
    }
    expect([snapshot.complete, names]).toEqual([true, [["insert", created.id, "Travel"]]]);
});

/** Serve the notes over one connection as alice. */
async function serve(world: Environment, database: DatabaseConnection) {
    return await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([notebook, note]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: {
            package: notebook.package,
        },
    });
}

/** Build alice's request context in the space, ending with the signal. */
function context(signal: AbortSignal, world: Environment): RequestContext {
    return subjectContext(principal.user.reference("universe", users.alice), spaceId, {
        environment: world,
        signal,
    });
}
