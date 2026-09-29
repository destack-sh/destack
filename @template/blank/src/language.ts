import type { SettingSelection } from "@destack/setting";
import type { SettingRow } from "@destack/setting/object";
import { language } from "./settings/index.ts";

/** Resolve the content language from the setting rows of a user's scope and the space they act in. */
export function readLanguage(selection: SettingSelection, rows: readonly SettingRow[]) {
    return language.resolve(selection, rows).value;
}
