import type { Hue } from "@destack/color";
import type { Localization } from "@destack/locale";
import type { JsonValue } from "@destack/schema";
import type { FieldChoice } from "../field/choice.ts";
import type { Icon } from "../trait/presentable.ts";

/** One of the values a person picks, groups or filters by, with how it looks. */
export interface Choice {
    /** The value. */
    readonly value: JsonValue;
    /** The title. */
    readonly title: string;
    /** The icon, none without one. */
    readonly icon?: Icon | undefined;
    /** The color, none without one. */
    readonly color?: Hue | undefined;
    /** The passage a search matched, none without one. */
    readonly excerpt?: string | undefined;
}

/** One of the values a person picks, groups or filters by, with how it looks. */
export const Choice = {
    /** Read a field's value as a choice in a locale, titled as stored without a title. */
    of({ value, title, icon, color }: FieldChoice, localization: Localization): Choice {
        return {
            value,
            title: title === undefined ? value : localization.render(title),
            ...(icon === undefined ? {} : { icon: { icon } }),
            ...(color === undefined ? {} : { color }),
        };
    },
};
