import type { SettingContext } from "@destack/setting";
import { language } from "./settings/index.ts";

/** Read the represented user's language through the host-provided settings context. */
export function readLanguage(settings: SettingContext) {
    return language.get(settings);
}
