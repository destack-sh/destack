import type { Call } from "@destack/object";
import { schema } from "@destack/schema";
import { SpaceSetting } from "../declare/space.ts";
import { SettingError } from "../error/index.ts";
import { SettingCatalog } from "../inspect/index.ts";
import { setting, type SettingRow } from "../object/index.ts";
import { SettingPlacement } from "../setting/index.ts";
import type { SettingServiceOptions } from "./server.ts";

/** Serve declared values, checking each written value and each value a stack places against its declaration. */
export function servedObjects(release: SettingServiceOptions["release"]) {
    return {
        setting: setting
            .handle({
                create: { authorize: (call) => requireDeclared(call, release) },
                update: { authorize: (call) => requireDeclared(call, release) },
            })
            .declare({
                keys: ["settings"],
                collect: (document) =>
                    schema.record(schema.string(), SpaceSetting).parse(document.settings ?? {}),
                resolve: async (_name, desired: SpaceSetting, stack) => {
                    // stamp the declaring release, and require a declared value at a placement the space permits
                    try {
                        const reader = await stack.release(
                            stack.manager.packageId,
                            stack.manager.installationId,
                        );
                        const declared = (await SettingCatalog.read(reader)).get(desired.setting);
                        const write = {
                            mode: desired.mode,
                            value: desired.value,
                            release: declared.package.version,
                        };
                        declared.requireWrite(write, stack.scope);

                        return { ...desired, release: declared.package.version };
                    } catch (error) {
                        throw error instanceof SettingError ? error.toServiceError() : error;
                    }
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

/** Require a written value to match its declaration in the consumer's release, or else the declaring package's. */
async function requireDeclared(
    call: Call,
    release: SettingServiceOptions["release"],
): Promise<void> {
    // read the value as the call leaves it
    const value = { ...call.target, ...call.input, scope: call.scope } as SettingRow;
    const placement = SettingPlacement.of(value);

    // check it against the release its placement selects
    try {
        const reader = await release(
            call.scope,
            placement.package ?? value.packageId,
            placement.installation,
        );
        const catalog = await SettingCatalog.read(reader);
        catalog
            .get({ packageId: value.packageId, name: value.name })
            .requireWrite(
                { ...placement, mode: value.mode, value: value.value, release: value.release },
                call.scope,
            );
    } catch (error) {
        throw error instanceof SettingError ? error.toServiceError() : error;
    }
}
