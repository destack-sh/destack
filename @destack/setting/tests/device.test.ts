import { principal } from "@destack/access";
import { RequestId, Scope } from "@destack/sync";
import { TestObjectStore } from "@destack/object/test";
import { ObjectClient, Observable } from "@destack/object/client";
import { canonicalize, Identifier, schema } from "@destack/schema";

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
import { openSettings } from "./fixture/store.ts";
import { alice, aliceUser, deviceSource, named, source } from "./fixture/value.ts";

test("resolve and watch values over live queries of the user's scope and the space she acts in", async () => {
    // serve Alice's personal scope and a space she reads
    await using personalStore = await openSettings({ scope: aliceUser });
    await using spaceStore = await openSettings({ owners: [aliceUser] });
    const spaceId = spaceStore.scope;

    // follow the values of both settings in Alice's personal scope and the space
    const [personal, acted] = await Promise.all([
        follow(personalStore, alice),
        follow(spaceStore, spaceId),
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

    // watch Alice's value arrive
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

    // watch the space's recommendation apply beneath Alice's values
    const recommended = await spaceStore
        .as(spaceId)
        .client(settingPackage)
        .setting.create({
            scope: spaceId,
            ...named(lineNumbers),
            mode: "recommend",
            value: false,
            release: lineNumbers.package.version,
            requestId: RequestId.create(spaceStore.environment),
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
});

test("resolve a value the device stores over the user's own, off the server", async () => {
    await using served = await openSettings({ scope: aliceUser });
    const personal = await follow(served, alice);
    const release = editor.package.version;

    // set Alice's own value on the server, and store the device's value in its database
    const placed = await served
        .as(alice)
        .client(settingPackage)
        .setting.create({
            scope: alice,
            ...named(editor),
            mode: "set",
            value: "vim",
            release,
            requestId: RequestId.create(served.environment),
        });
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
    await using served = await openSettings({ scope: aliceUser });
    const personal = await follow(served, alice);
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

/** Follow a scope's values as Alice into a local replica, beside the device's values. */
async function follow(
    served: TestObjectStore,
    scope: string,
): Promise<
    ObjectClient<{ readonly setting: typeof setting; readonly deviceSetting: typeof deviceSetting }>
> {
    // replicate the scope's values into a local database beside the device's own values
    const local = await connectBunSqlite(
        ":memory:",
        ObjectClient.tables({ setting, deviceSetting }),
        { environment: served.environment },
    );
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        environment: served.environment,
        database: local,
        objects: { setting, deviceSetting },
        scope,
        caller: aliceUser,
        device: principal.device.reference(
            Scope.universe.id,
            Identifier.create("device", served.environment),
        ),
        endpoint: served.as(alice).endpoint(settingPackage),
        reconnect: (cell) => {
            throw new TypeError(`no scope moves in this test, yet one moved to ${cell}`);
        },
    });
    const controller = new AbortController();
    const failures: unknown[] = [];
    const running = client.run(controller.signal, (error) => failures.push(error));
    served.defer(async () => {
        controller.abort();
        await running;
        await client.close();
        expect(failures).toEqual([]);
    });

    // wait until the scope's access replica arrives, as following its chain requires
    await local.log.until(
        async () => (await client.access.position(local)) !== undefined,
        controller.signal,
    );

    return client;
}
