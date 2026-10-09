import { space } from "@destack/account/object";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ObjectClient } from "@destack/object/client";
import { ResourceContext } from "@destack/resource/context";
import { canonicalize, Identifier, schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Observable } from "@destack/service/observable";
import { Server } from "@destack/service/server";

import { expect, onTestFinished, test } from "@destack/test";
import { clientSetting, setting, type SettingValue } from "../src/object/index.ts";
import {
    type Setting,
    SettingPlacement,
    SettingResolution,
    SettingSelection,
} from "../src/setting/index.ts";
import { editor, lineNumbers, notes } from "./fixture/setting/index.ts";
import { settingService, Storage } from "./fixture/storage.ts";
import { alice, clientSource, named, source } from "./fixture/value.ts";

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
        const recommended = await storage.call(setting, "create", spaceId, {
            ...named(lineNumbers),
            mode: "recommend",
            value: false,
            release: lineNumbers.package.version,
        });
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

        // resolve the space's recommendation for an anonymous visitor from the space alone
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

test("resolve a value the client keeps over the user's own, keeping it off the server", async () => {
    const storage = await Storage.open("sqlite");
    onTestFinished(() => storage.close());
    const personal = await follow(storage, alice);
    const release = editor.package.version;

    // set Alice's own value on the server, and keep the client's value in its database
    const placed = await storage.call(setting, "create", alice, {
        ...named(editor),
        mode: "set",
        value: "vim",
        release,
    });
    const kept = personal.mutate(clientSetting).create({
        ...named(editor),
        value: "standard",
        release,
    });
    const value = await kept.predicted;
    await kept.confirmed;

    // read both layers and resolve the client's value over Alice's
    const selection = SettingSelection.parse({ scope: alice });
    const values = personal.query.setting.findMany({ where: editor.condition(selection) });
    const placedRows = values.subscribe();
    const keptRows = personal.query.clientSetting.findMany().subscribe();
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

    // expect the client's value to win, and no mutation of it to reach the outbox
    expect({
        value: resolved.value,
        sources: resolved.sources,
        overridden: resolved.overridden,
        mutations: (await personal.inspect()).mutations,
    }).toEqual({
        value: "standard",
        sources: [clientSource(value)],
        overridden: [{ kind: "default", package: notes }, source(placed)],
        mutations: { pending: 0, executed: 0, rejected: 0 },
    });
});

/** Serve a scope's values to Alice and follow them into a local copy beside the client's own values. */
async function follow(
    storage: Storage,
    scope: string,
): Promise<
    ObjectClient<{ readonly setting: typeof setting; readonly clientSetting: typeof clientSetting }>
> {
    // serve the scope as Alice
    const server = Server.start({
        ...storage.objects.implement(settingService),
        audience: notes.id,
        scope,
        resources: new ResourceContext(),
        health: new Health("setting"),
        drainTimeout: 100,
        authorizeMachine: async () => {},
        authenticate: async () =>
            new Authentication({
                subject: storage.subject,
                subjects: [storage.subject],
                credential: { kind: "session", id: "test-session" },
                audience: notes.id,
                scope,
                verifiedAt: Date.now(),
                expiresAt: Date.now() + 60_000,
            }),
    });
    onTestFinished(() => server.close());

    // copy the scope's values into a local database beside the client's own values
    const local = await TestDatabase.create("sqlite", []);
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        database: local.database,
        objects: { setting, clientSetting },
        scope,
        caller: storage.subject,
        client: principal.client.reference(Scope.universe.id, Identifier.create("client")),
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
