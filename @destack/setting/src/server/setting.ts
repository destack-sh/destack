import { SpaceSetting } from "../declare/space.ts";
import { setting, type SettingValue } from "../object/index.ts";
import { type OpenSettingCatalog, SettingCatalog, SettingPlacement } from "../setting/index.ts";
import { BuildCache } from "@destack/package/manifest";
import { schema } from "@destack/schema";

/** Serve declared values, checking each written value against the catalog declaring it and each value a stack places against its release. */
export function serveSettings(catalog: OpenSettingCatalog) {
    // read each stack release's settings once
    const catalogs = new BuildCache((reader) => SettingCatalog.read(reader));

    return {
        setting: setting
            .handle({
                create: {
                    authorize: (call) =>
                        requireDeclared({ ...call.input, scope: call.scope }, catalog),
                },
                update: {
                    authorize: (call) =>
                        requireDeclared(
                            { ...call.target, ...call.input, scope: call.scope },
                            catalog,
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
                    const declared = (await catalogs.read(reader)).get(desired.setting);

                    // require a declared value at a placement the space permits, stamped with its release
                    const write = {
                        mode: desired.mode,
                        value: desired.value,
                        release: declared.package.version,
                    };
                    declared.requireWrite(write, stack.scope);

                    return { ...desired, release: declared.package.version };
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
    catalog: OpenSettingCatalog,
): Promise<void> {
    // check it against the catalog declaring it at its placement
    const placement = SettingPlacement.of(value);
    const reference = { packageId: value.packageId, name: value.name };
    const declared = await catalog({ ...placement, ...reference, release: value.release });
    declared
        .get(reference)
        .requireWrite(
            { ...placement, mode: value.mode, value: value.value, release: value.release },
            value.scope,
        );
}
