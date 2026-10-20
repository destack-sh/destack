import { space } from "./fixture/space.ts";
import { Caller, principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ACCESS_PUBLICATION } from "@destack/object";
import { ObjectClient } from "@destack/object/client";
import { ResourceContext } from "@destack/resource/context";
import { canonicalize, Identifier, schema } from "@destack/schema";
import { Health } from "@destack/runtime/health";
import { Observable } from "@destack/runtime/observable";
import { Server } from "@destack/runtime/server";

import { expect, onTestFinished, test } from "@destack/test";
import { deviceSetting, setting, SettingValue } from "../src/object/index.ts";
import { connectBunSqlite } from "@destack/db/bun";
import {
    type Setting,
    SettingPlacement,
    SettingResolution,
    SettingSelection,
} from "../src/setting/index.ts";
import { editor, lineNumbers, notes } from "./fixture/setting/index.ts";
import settingPackage from "../src/package.ts";
import { Storage } from "./fixture/storage.ts";
import { alice, deviceSource, named, source } from "./fixture/value.ts";
import { Clock } from "@destack/environment";

/** The space Alice acts in. */
const spaceId = schema.identifier("space").parse("space-019f5530-8000-7000-8000-000000000030");

test.each(TEST_DIALECTS)(
    "resolve and watch values over live queries of the user's scope and the space she acts in on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());
        await storage.own(space.reference(Scope.universe.id, spaceId));

        // follow the values of both settings in Alice's personal scope and the space
        const [personal, acted] = await Promise.all([
            follow(storage, alice),
            follow(storage, spaceId),
        ]);
        const selection = SettingSelection.parse({ scope: alice, space: spaceId });
        const where = { OR: [editor.condition(selection), lineNumbers.condition(selection)] };
        const queries = [personal, acted].map((client) =>
            client.query.setting.findMany({ where }).subscribe(),
        );
        onTestFinished(async () => {
            await Promise.all(queries.map((query) => query.close()));
        });
        await Promise.all(queries.map((query) => query.ready));
        const chain = await acted.replica.chain(acted.database);
        const resolve = (rows: readonly (readonly SettingValue[])[]) => {
            const values = rows.flat();

            return [editor, lineNumbers].map((declared: Setting) =>
                declared.resolve(selection, values, chain),
            );
        };

        // resolve the defaults
        const standard = {
            setting: editor.reference,
            selection,
            value: "standard",
            sources: [{ kind: "default" as const, package: notes }],
            overridden: [],
            enforcement: "ordinary" as const,
        };
        const numbered = { ...standard, setting: lineNumbers.reference, value: true };
        expect(resolve(await Promise.all(queries.map((query) => query.read())))).toEqual([
            standard,
            numbered,
        ]);

        // watch Alice's own value arrive
        const controller = new AbortController();
        onTestFinished(() => controller.abort());
        const watching = Observable.latest(
            queries.map((query) => query.watch(controller.signal)),
            resolve,
        )[Symbol.asyncIterator]();
        let previous = "";
        const changed = async () => {
            // skip results equal to the last one, such as a confirmed prediction
            for (;;) {
                const next = await watching.next();
                const current = canonicalize(next.value);
                if (current !== previous) {
                    previous = current;

                    return next;
                }
            }
        };
        expect(await changed()).toEqual({ done: false, value: [standard, numbered] });
        const saved = personal.mutate(setting).create({
            ...named(editor),
            ...SettingPlacement.of({ ...selection, scope: alice }),
            mode: "set",
            value: "vim",
            release: editor.package.version,
        });
        await saved.confirmed;
        const value = await saved.predicted;
        const chosen = {
            ...standard,
            value: "vim",
            sources: [source(value)],
            overridden: standard.sources,
        };
        expect(await changed()).toEqual({ done: false, value: [chosen, numbered] });
        expect(SettingResolution.observed(chosen, { ...selection, scope: alice })).toEqual({
            id: value.id,
            revision: value.revision,
        });

        // watch the space's recommendation apply beneath Alice's own values
        const recommended = await storage.call(
            setting.call("create", {
                scope: spaceId,
                ...named(lineNumbers),
                mode: "recommend",
                value: false,
                release: lineNumbers.package.version,
            }),
        );
        const lowered = {
            ...numbered,
            value: false,
            sources: [source(recommended)],
            overridden: standard.sources,
        };
        expect(await changed()).toEqual({ done: false, value: [chosen, lowered] });

        // watch the default return once Alice deletes her value
        await personal.mutate(setting).delete({ id: value.id }).confirmed;
        expect(await changed()).toEqual({ done: false, value: [standard, lowered] });
        controller.abort();
        await watching.return?.(undefined);

        // resolve the space's recommendation for an anonymous visitor only from the space
        const anonymous = SettingSelection.parse({ scope: null });
        const visited = acted.query.setting
            .findMany({ where: lineNumbers.condition(anonymous) })
            .subscribe();
        onTestFinished(() => visited.close());
        await visited.ready;
        expect(lineNumbers.resolve(anonymous, await visited.read(), chain)).toEqual({
            ...lowered,
            selection: anonymous,
        });
    },
    5000,
);

test("resolve a value the device stores over the user's own, off the server", async () => {
    const storage = await Storage.open("sqlite");
    onTestFinished(() => storage.close());
    const personal = await follow(storage, alice);
    const release = editor.package.version;

    // set Alice's own value on the server, and store the device's value in its database
    const placed = await storage.call(
        setting.call("create", {
            scope: alice,
            ...named(editor),
            mode: "set",
            value: "vim",
            release,
        }),
    );
    const local = personal.mutate(deviceSetting).create({
        ...named(editor),
        value: "standard",
        release,
    });
    const value = await local.predicted;
    await local.confirmed;

    // read both layers and resolve the device's value over Alice's
    const selection = SettingSelection.parse({ scope: alice });
    const values = personal.query.setting.findMany({ where: editor.condition(selection) });
    const placedRows = values.subscribe();
    const keptRows = personal.query.deviceSetting.findMany().subscribe();
    onTestFinished(async () => {
        await Promise.all([placedRows.close(), keptRows.close()]);
    });
    await Promise.all([placedRows.ready, keptRows.ready]);
    const chain = await personal.replica.chain(personal.database);
    const resolved = editor.resolve(
        selection,
        await placedRows.read(),
        chain,
        await keptRows.read(),
    );

    // expect the device's value to win, and no mutation of it to reach the outbox
    expect({
        value: resolved.value,
        sources: resolved.sources,
        overridden: resolved.overridden,
        mutations: (await personal.inspect()).mutations,
    }).toEqual({
        value: "standard",
        sources: [deviceSource(value)],
        overridden: [{ kind: "default", package: notes }, source(placed)],
        mutations: { pending: 0, executed: 0, rejected: 0 },
    });
});

test("follow a person's values with the device's own, yielding again once the device's value changes", async () => {
    // store device values for Alice's editor mode and for another setting
    const storage = await Storage.open("sqlite");
    onTestFinished(() => storage.close());
    const personal = await follow(storage, alice);
    const release = editor.package.version;
    const created = await personal.mutate(deviceSetting).create({
        ...named(editor),
        value: "standard",
        release,
    }).predicted;
    await personal.mutate(deviceSetting).create({
        ...named(lineNumbers),
        value: false,
        release: lineNumbers.package.version,
    }).predicted;

    // follow the values along her chain and on the device, then change the device's value
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    const steps = SettingValue.follow(
        personal.database,
        [editor.reference],
        { scope: alice, device: schema.identifier("device").parse(created.scope) },
        stopping.signal,
    );
    const first = await steps.next();
    await personal.mutate(deviceSetting).update({ id: created.id, value: "vim" }).predicted;
    const second = await steps.next();

    // yield the device's value of the followed setting alone, each time it changes
    expect(
        [first, second].map((step) =>
            step.done === true ? [] : step.value.device.map((row) => row.value),
        ),
    ).toEqual([["standard"], ["vim"]]);
});

test("replicate a person's values along their chain into a database, yielding each change, and fail with the replica's failure", async () => {
    const storage = await Storage.open("sqlite");
    onTestFinished(() => storage.close());
    const server = serve(storage, alice);

    // replicate Alice's values from the store serving them into a database of the setting tables
    const replica = (fetch: (request: Request) => Promise<Response>) =>
        connectBunSqlite(":memory:", ObjectClient.tables({ setting })).then((database) => {
            onTestFinished(() => database.close());

            return SettingValue.replicate(
                {
                    endpoint: { url: "https://settings.test", fetch },
                    scope: alice,
                    caller: storage.subject,
                    database,
                    settings: [editor.reference],
                    clock: Clock.system,
                },
                AbortSignal.timeout(5000),
            );
        });
    const replicated = await replica((request) => server.fetch(request));

    // yield none, then Alice's value once she sets it
    const first = await replicated.next();
    await storage.call(
        setting.call("create", {
            scope: alice,
            ...named(editor),
            mode: "set",
            value: "vim",
            release: editor.package.version,
        }),
    );
    const second = await replicated.next();
    await replicated.return(undefined);

    // fail a replica whose store refuses it
    const unavailable = {
        defined: true,
        code: "SERVICE_UNAVAILABLE",
        status: 503,
        message: "down",
    };
    const refused = await replica(async () => Response.json(unavailable, { status: 503 }));
    const failure = await refused
        .next()
        .catch((error: unknown) => (error instanceof Error ? error.message : error));

    expect({
        first: first.done === true ? undefined : first.value,
        second:
            second.done === true
                ? undefined
                : second.value.values.map((row) => schema.string().parse(row.value)),
        failure,
    }).toEqual({
        first: { chain: [alice, Scope.universe.id], values: [], device: [] },
        second: ["vim"],
        failure: "down",
    });
});

test("yield a person's values set before the replica starts in its first yield, along their chain however late it arrives", async () => {
    const storage = await Storage.open("sqlite");
    onTestFinished(() => storage.close());
    const server = serve(storage, alice);

    // set Alice's editor before replicating her values
    await storage.call(
        setting.call("create", {
            scope: alice,
            ...named(editor),
            mode: "set",
            value: "vim",
            release: editor.package.version,
        }),
    );

    // replicate them, the access rows carrying the scope chain arriving late, and read the first yield
    const database = await connectBunSqlite(":memory:", ObjectClient.tables({ setting }));
    onTestFinished(() => database.close());
    const settingsServed = Promise.withResolvers<void>();
    const late = async (request: Request) => {
        // hold the access rows back until the setting rows were answered
        const body = await request.clone().text();
        if (body.includes(`"shape":"${ACCESS_PUBLICATION}"`)) {
            await settingsServed.promise;

            return server.fetch(request);
        }

        // answer the setting rows, then release the access rows
        const response = await server.fetch(request);
        settingsServed.resolve();

        return response;
    };
    const replicated = SettingValue.replicate(
        {
            endpoint: { url: "https://settings.test", fetch: late },
            scope: alice,
            caller: storage.subject,
            database,
            settings: [editor.reference],
            clock: Clock.system,
        },
        AbortSignal.timeout(5000),
    );
    const first = await replicated.next();
    await replicated.return(undefined);

    // yield the value already set along the chain, never an empty snapshot before them
    expect(
        first.done === true
            ? undefined
            : {
                  chain: first.value.chain,
                  values: first.value.values.map((row) => schema.string().parse(row.value)),
              },
    ).toEqual({ chain: [alice, Scope.universe.id], values: ["vim"] });
});

/** Serve a scope's values to Alice and follow them into a local replica. */
async function follow(
    storage: Storage,
    scope: string,
): Promise<
    ObjectClient<{ readonly setting: typeof setting; readonly deviceSetting: typeof deviceSetting }>
> {
    // serve the scope as Alice
    const server = serve(storage, scope);

    // replicate the scope's values into a local database beside the device's own values
    const local = await TestDatabase.create("sqlite", []);
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: local.database,
        objects: { setting, deviceSetting },
        scope,
        caller: storage.subject,
        device: principal.device.reference(Scope.universe.id, Identifier.create("device")),
        endpoint: { url: "https://settings.test", fetch: (request) => server.fetch(request) },
        reconnect: (cell) => {
            throw new TypeError(`no scope moves in this test, yet one moved to ${cell}`);
        },
    });
    const controller = new AbortController();
    const running = client.run(controller.signal, (error) => {
        throw error;
    });
    onTestFinished(async () => {
        controller.abort();
        await running;
        await client.close();
    });

    return client;
}

/** Serve a scope's setting values as Alice, until the test ends. */
function serve(storage: Storage, scope: string): Server {
    const server = Server.start({
        clock: Clock.system,
        ...storage.objects.package(settingPackage),
        scope,
        resources: new ResourceContext(),
        health: new Health("setting"),
        drainTimeout: 100,
        authorizeMachine: async () => {},
        authenticate: async () => ({
            caller: Caller.of(storage.subject),
            credential: {
                kind: "session",
                id: "test-session",
                audience: settingPackage.id,
                scope,
                verifiedAt: Date.now(),
                expiresAt: Date.now() + 60_000,
            },
        }),
    });
    onTestFinished(() => server.close());

    return server;
}
