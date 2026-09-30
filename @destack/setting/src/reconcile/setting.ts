import { defineReconciler, type Reconciler } from "@destack/object/server";
import { schema } from "@destack/schema";
import { SpaceSetting } from "../declare/space.ts";
import { SettingError } from "../error/index.ts";
import { SettingCatalog } from "../inspect/index.ts";
import { setting } from "../object/index.ts";
import type { SettingServiceOptions } from "../server/server.ts";

/** Reconcile the setting values stacks place in their space, checked against the stack's release. */
export function settingReconcilers(
    release: SettingServiceOptions["release"],
): readonly Reconciler[] {
    const values = defineReconciler(setting, {
        keys: ["settings"],
        collect: (document) =>
            schema.record(schema.string(), SpaceSetting).parse(document.settings ?? {}),
        resolve: async (_name, desired: SpaceSetting, context) => {
            // stamp the declaring release, and require a declared value at a placement the space permits
            try {
                const reader = await release(
                    context.manager.packageId,
                    context.manager.installationId,
                );
                const declared = (await SettingCatalog.read(reader)).get(desired.setting);
                const write = {
                    mode: desired.mode,
                    value: desired.value,
                    release: declared.package.version,
                };
                declared.requireWrite(write, context.scope);

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
    });

    return [values];
}
