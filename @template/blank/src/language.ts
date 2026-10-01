import type { SettingSelection } from "@destack/setting";
import type { SettingValue } from "@destack/setting/object";
import { language } from "./settings/index.ts";

/** Resolve the content language from setting rows along a scope chain, nearest scope first. */
export function readLanguage(
    selection: SettingSelection,
    rows: readonly SettingValue[],
    chain: readonly string[],
) {
    return language.resolve(selection, rows, chain).value;
}
