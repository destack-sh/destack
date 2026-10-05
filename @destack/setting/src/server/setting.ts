import { SpaceSetting } from "../declare/space.ts";
import { setting, type SettingValue } from "../object/index.ts";
import { SettingCatalog, SettingPlacement } from "../setting/index.ts";
import { BuildCache, type BuildReader } from "@destack/package/manifest";
import type { PackageId } from "@destack/package";
import { schema, type Identifier } from "@destack/schema";

/** Serve declared values, checking each written value and each value a stack places against its declaration. */
export function serveSettings(
    release: (
        scope: string,
        packageId: PackageId,
        installationId?: Identifier<"installation">,
    ) => Promise<BuildReader>,
) {
    // read each release's settings once
    const catalogs = new BuildCache((reader) => SettingCatalog.read(reader));

    return {
        setting: setting
            .handle({
                create: {
                    authorize: (call) =>
                        requireDeclared({ ...call.input, scope: call.scope }, release, catalogs),
                },
                update: {
                    authorize: (call) =>
                        requireDeclared(
                            { ...call.target, ...call.input, scope: call.scope },
                            release,
                            catalogs,
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
    release: Parameters<typeof serveSettings>[0],
    catalogs: BuildCache<SettingCatalog>,
): Promise<void> {
    // check it against the release its placement selects
    const placement = SettingPlacement.of(value);
    const reader = await release(
        value.scope,
        placement.package ?? value.packageId,
        placement.installation,
    );
    const catalog = await catalogs.read(reader);
    catalog
        .get({ packageId: value.packageId, name: value.name })
        .requireWrite(
            { ...placement, mode: value.mode, value: value.value, release: value.release },
            value.scope,
        );
}
