import { type Localization, t } from "@destack/locale";
import type { SearchState } from "@destack/view";

/** Title a search by the text it took for the one typed, undefined where it took none. */
export function titleOf(search: SearchState, locale: Localization): string | undefined {
    const text = search.text();

    return text === undefined || !search.isCorrected()
        ? undefined
        : locale.render(t`Results for “${text}”`);
}
