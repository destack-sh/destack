import { SpaceSetting } from "../declare/space.ts";
import { setting, type SettingValue } from "../object/index.ts";
import { SettingCatalog, SettingPlacement } from "../setting/index.ts";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { schema, type Identifier } from "@destack/schema";

/** Open the build of a package's release in a scope, the installation's when given. */
export type OpenRelease = (
    scope: string,
    packageId: PackageId,
    installation?: Identifier<"installation">,
) => Promise<BuildReader>;

/** Serve declared values, checking each written value and each value a stack places against its declaration. */
export function serveSettings(release: OpenRelease) {
    return {
        setting: setting
            .handle({
                create: {
                    authorize: (call) =>
                        requireDeclared({ ...call.input, scope: call.scope }, release),
                },
                update: {
                    authorize: (call) =>
                        requireDeclared(
                            { ...call.target, ...call.input, scope: call.scope },
                            release,
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
                    const declared = (await SettingCatalog.read(reader)).get(desired.setting);

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
    release: OpenRelease,
): Promise<void> {
    // check it against the release its placement selects
    const placement = SettingPlacement.of(value);
    const reader = await release(
        value.scope,
        placement.package ?? value.packageId,
        placement.installation,
    );
    const catalog = await SettingCatalog.read(reader);
    catalog
        .get({ packageId: value.packageId, name: value.name })
        .requireWrite(
            { ...placement, mode: value.mode, value: value.value, release: value.release },
            value.scope,
        );
}
