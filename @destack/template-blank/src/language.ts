import type { ResourceContext } from "@destack/resource/context";
import type { SettingTarget } from "@destack/setting";
import { SettingClient } from "@destack/setting/client";
import { language } from "./settings/index.ts";
import { settings } from "./connection/index.ts";

/** Read the represented user's language through the host-provided settings context. */
export function readLanguage(target: SettingTarget, resources: ResourceContext) {
    const client = new SettingClient(settings.get(resources), settings.packageId, target);

    return language.get(client);
}
