import type { ResourceContext } from "@destack/resource/context";
import type { SettingSelection } from "@destack/setting";
import { SettingClient } from "@destack/setting/client";
import { language } from "./settings/index.ts";
import { settings } from "./connection/index.ts";

/** Read the represented user's language through the host-provided settings context. */
export function readLanguage(selection: SettingSelection, resources: ResourceContext) {
    const client = new SettingClient(settings.get(resources), settings.package.id, selection);

    return language.get(client);
}
