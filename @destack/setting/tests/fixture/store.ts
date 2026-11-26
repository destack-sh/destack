import { principal } from "@destack/access";
import { connectBunSqlite } from "@destack/db/bun";
import { ObjectClient } from "@destack/object/client";
import { TestObjectStore } from "@destack/object/test";
import { Identifier } from "@destack/schema";
import { onTestFinished } from "@destack/test";
import { Scope } from "@destack/sync";
import { deviceSetting, setting } from "../../src/object/index.ts";
import settingPackage from "../../src/package.ts";
import { alice, aliceUser } from "./value.ts";
import type { ObjectReference, Subject } from "@destack/sync";
import { settingServer } from "../../src/server/index.ts";
import type { PackageId } from "@destack/package";
import type { Setting } from "../../src/setting/index.ts";
import { editor, history, lineNumbers, release } from "./setting/index.ts";

/** Serve setting values in a scope, checked against a release declaring some settings. */
export async function openSettings(
    options: {
        /** The scope the values live in. */
        readonly scope?: ObjectReference;
        /** The settings the release declares, or a function returning them at each open. */
        readonly settings?: readonly Setting[] | (() => readonly Setting[]);
        /** The subjects owning the scope beside itself. */
        readonly owners?: readonly Subject[];
        /** The packages and versions the store opened releases of, in order. */
        readonly opened?: { readonly packageId: PackageId; readonly version?: string }[];
    } = {},
): Promise<TestObjectStore> {
    // serve the setting package against a release declaring the settings
    const { scope, settings = [editor, lineNumbers, history], owners = [], opened } = options;

    return await TestObjectStore.open({
        packages: [settingServer({ after: [] })],
        // open the release for each package and version, recording the open
        release: async (_view, _scope, packageId, version) => {
            opened?.push({ packageId, ...(version === undefined ? {} : { version }) });

            return await release(typeof settings === "function" ? settings() : settings);
        },
        ...(scope === undefined ? {} : { scope }),
        owners,
    });
}

/** Follow a scope's values as Alice into a local replica, beside the device's values. */
export async function followScope(
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
        remote: served.as(alice).connect(settingPackage, { scope }),
    });
    const controller = new AbortController();
    const running = client.run(controller.signal);
    served.defer(async () => {
        controller.abort();
        await running;
        await client.close();
    });

    // wait until the scope's access replica arrives, as following its chain requires
    await local.log.waitUntil(
        async () => (await client.access.position(local)) !== undefined,
        controller.signal,
    );

    return client;
}
