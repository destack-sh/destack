import { SpaceSetting } from "../declare/space.ts";
import { setting, type SettingValue } from "../object/index.ts";
import {
    SettingCatalog,
    SettingPlacement,
    type SettingReference,
    type SettingWrite,
} from "../setting/index.ts";
import { BuildCache } from "@destack/package/manifest";
import { schema } from "@destack/schema";

/** Serve declared values, checking each written value against the catalog `declared` reads for it. */
export function serveSettings(
    declared: (
        written: SettingReference & SettingPlacement & Pick<SettingWrite, "release">,
    ) => Promise<SettingCatalog>,
) {
    // read each stack release's settings once
    const catalogs = new BuildCache((reader) => SettingCatalog.read(reader));

    return {
        setting: setting
            .handle({
                create: {
                    authorize: (call) =>
                        requireDeclared({ ...call.input, scope: call.scope }, declared),
                },
                update: {
                    authorize: (call) =>
                        requireDeclared(
                            { ...call.target, ...call.input, scope: call.scope },
                            declared,
                        ),
                },
            })
            .declare({
                keys: ["settings"],
                collect: (document) =>
                    schema.record(schema.string(), SpaceSetting).parse(document["settings"] ?? {}),
                resolve: async (_name, desired: SpaceSetting, stack) => {
                    // read the declaring release
                    const reader = await stack.release(
                        stack.manager.packageId,
                        stack.manager.installationId,
                    );
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
            }),
    };
}

/** Require a written setting value to match its declaration. */
async function requireDeclared(
    value: Parameters<typeof SettingPlacement.of>[0] &
        Pick<SettingValue, "packageId" | "name" | "mode" | "value" | "release">,
    declared: Parameters<typeof serveSettings>[0],
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
