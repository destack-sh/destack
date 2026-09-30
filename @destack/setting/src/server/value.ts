import type { Call } from "@destack/object";
import { SettingError } from "../error/index.ts";
import { SettingCatalog } from "../inspect/index.ts";
import { setting, type SettingRow } from "../object/index.ts";
import { SettingPlacement } from "../setting/index.ts";
import type { SettingServiceOptions } from "./server.ts";

/** Serve declared values, checking each written value against its declaration. */
export function servedObjects(release: SettingServiceOptions["release"]) {
    return {
        setting: setting.handle({
            create: { authorize: (call) => requireDeclared(call, release) },
            update: { authorize: (call) => requireDeclared(call, release) },
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
        const reader = await release(placement.package ?? value.packageId, placement.installation);
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
