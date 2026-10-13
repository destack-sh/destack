import { StackSetting } from "../declare/stack.ts";
import { setting, type SettingValue } from "../object/index.ts";
import {
    SettingCatalog,
    SettingPlacement,
    type SettingReference,
    type SettingWrite,
} from "../setting/index.ts";
import settingPackage from "../package.ts";
import { BuildCache } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import type { ObjectType } from "@destack/object";
import { ObjectExtension } from "@destack/object/server";

/** What a store serves its setting values with. */
export interface SettingOptions {
    /** Read the catalog declaring each written value. */
    readonly declared: (
        written: SettingReference & SettingPlacement & Pick<SettingWrite, "release">,
    ) => Promise<SettingCatalog>;
    /** The types whose declarations a stack applies before its settings. */
    readonly after: readonly ObjectType[];
}

/** Extend a store with its setting values, checked against their declarations. */
export function implementSettings(options: SettingOptions): ObjectExtension {
    return ObjectExtension.of(settingPackage, [serveSettings(options)]);
}

/** Serve values checked against their declarations. */
export function serveSettings(options: SettingOptions) {
    // read each stack release's settings once
    const { declared, after } = options;
    const catalogs = new BuildCache((reader) => SettingCatalog.read(reader));

    return setting.implement({
        methods: {
            create: {
                authorize: (call) =>
                    requireDeclared({ ...call.input, scope: call.scope }, declared),
            },
            update: {
                authorize: (call) =>
                    requireDeclared({ ...call.target, ...call.input, scope: call.scope }, declared),
            },
        },
        declaration: {
            after,
            keys: ["settings"],
            collect: (document) =>
                schema.record(schema.string(), StackSetting).parse(document["settings"] ?? {}),
            resolve: async (_name, desired: StackSetting, stack) => {
                // read the release declaring the setting: the stack's own for its package, else the one the space installs
                const isOwn = desired.setting.packageId === stack.manager.packageId;
                const reader = isOwn
                    ? await stack.release(stack.manager.packageId, stack.manager.installationId)
                    : await stack.release(desired.setting.packageId);
                const placed = (await catalogs.read(reader)).get(desired.setting);

                // require a declared value at a placement the space permits, stamped with its release
                const write = {
                    mode: desired.mode,
                    value: desired.value,
                    release: placed.package.version,
                };
                placed.requireWrite(write, stack.scope);

                return { ...desired, release: placed.package.version };
            },
            values: (_name, desired) => ({
                packageId: desired.setting.packageId,
                name: desired.setting.name,
                mode: desired.mode,
                value: desired.value,
                release: desired.release,
            }),
        },
    });
}

/** Require a written setting value to match its declaration. */
async function requireDeclared(
    value: Parameters<typeof SettingPlacement.of>[0] &
        Pick<SettingValue, "packageId" | "name" | "mode" | "value" | "release">,
    declared: SettingOptions["declared"],
): Promise<void> {
    // check it against the catalog declaring it at its placement
    const placement = SettingPlacement.of(value);
    const reference = { packageId: value.packageId, name: value.name };
    const catalog = await declared({ ...placement, ...reference, release: value.release });
    catalog
        .get(reference)
        .requireWrite(
            { ...placement, mode: value.mode, value: value.value, release: value.release },
            value.scope,
        );
}
