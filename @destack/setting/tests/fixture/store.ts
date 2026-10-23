import { TestObjectStore } from "@destack/object/test";
import type { ObjectReference, Subject } from "@destack/sync";
import { settingServer } from "../../src/server/index.ts";
import { Setting, type SettingDeclared } from "../../src/setting/index.ts";
import { editor, history, lineNumbers, release } from "./setting/index.ts";

/** Serve setting values in a scope, checked against a release declaring some settings. */
export async function openSettings(
    options: {
        /** The scope the values live in. */
        readonly scope?: ObjectReference;
        /** The settings the release declares. */
        readonly settings?: readonly Setting[];
        /** The subjects owning the scope beside itself. */
        readonly owners?: readonly Subject[];
        /** The values the store opened their setting's build for, in order. */
        readonly opened?: SettingDeclared[];
    } = {},
): Promise<TestObjectStore> {
    // serve the setting package against a release declaring the settings
    const { scope, settings = [editor, lineNumbers, history], owners = [], opened } = options;
    const reader = release(settings);
    const open = (written: SettingDeclared) => {
        opened?.push(written);

        return reader;
    };

    return await TestObjectStore.open({
        packages: [settingServer({ declared: Setting.declared(open), after: [] })],
        ...(scope === undefined ? {} : { scope }),
        owners,
    });
}
