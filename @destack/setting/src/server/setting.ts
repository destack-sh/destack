import { StackSetting } from "../declare/stack.ts";
import { setting, type SettingValue } from "../object/index.ts";
import {
    Setting,
    SettingPlacement,
    type SettingReference,
    type SettingWrite,
} from "../setting/index.ts";
import { BuildCache } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import type { ObjectType } from "@destack/object";

/** What a store serves its setting values with. */
export interface SettingOptions {
    /** Read the setting each written value is a value of. */
    readonly declared: (
        written: SettingReference & SettingPlacement & Pick<SettingWrite, "release">,
    ) => Promise<Setting>;
    /** The types whose declarations a stack applies before its settings. */
    readonly after: readonly ObjectType[];
}

/** Implement setting values, checked against their declarations. */
export function implementSetting(options: SettingOptions) {
    // read each stack release's settings once
    const { declared, after } = options;
    const builds = new BuildCache((reader) => Setting.list(reader));

    return setting.implement({
        methods: {
            create: {
                authorize: (context) =>
                    requireDeclared({ ...context.input, scope: context.scope }, declared),
            },
            update: {
                authorize: (context) =>
                    requireDeclared(
                        { ...context.target, ...context.input, scope: context.scope },
                        declared,
                    ),
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
                const placed = Setting.find(await builds.read(reader), desired.setting);

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
    // check it against its setting's declaration at its placement
    const placement = SettingPlacement.of(value);
    const reference = { packageId: value.packageId, name: value.name };
    const setting = await declared({ ...placement, ...reference, release: value.release });
    setting.requireWrite(
            { ...placement, mode: value.mode, value: value.value, release: value.release },
            value.scope,
        );
}
