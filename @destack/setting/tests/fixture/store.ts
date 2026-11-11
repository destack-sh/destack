import { TestObjectStore } from "@destack/object/test";
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
