import type { Hue } from "@destack/style";
import type { Localization } from "@destack/locale";
import type { Moment } from "@destack/db";
import type { JsonObject, JsonValue } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import type { Hit } from "../client/access.ts";
import type { ObjectType } from "../object/object.ts";
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
    /** One sentence on when to choose it, none without one. */
    readonly description?: string | undefined;
    /** The passage a search matched, none without one. */
    readonly excerpt?: string | undefined;
}

/** One of the values a person picks, groups or filters by, with how it looks. */
export const Choice = {
    /** Read a field's value as a choice in a locale, titled as stored without a title. */
    of(
        { value, title, icon, color, description }: FieldChoice,
        localization: Localization,
    ): Choice {
        return {
            value,
            title: title === undefined ? value : localization.render(title),
            ...(icon === undefined ? {} : { icon: { icon } }),
            ...(color === undefined ? {} : { color }),
            ...(description === undefined ? {} : { description: localization.render(description) }),
        };
    },

    /** Read an object as a choice in a locale, as its type presents its row at a moment. */
    object(
        type: ObjectType,
        reference: ObjectReference,
        row: JsonObject,
        moment: Moment,
        localization: Localization,
    ): Choice {
        return { value: reference, ...type.appearance(row, moment, localization) };
    },

    /** Read a search's hit as a choice in a locale, as it looks at a moment. */
    hit(
        { type, reference, row, excerpt }: Hit,
        moment: Moment,
        localization: Localization,
    ): Choice {
        return {
            ...Choice.object(type, reference, row, moment, localization),
            ...(excerpt === undefined ? {} : { excerpt }),
        };
    },
};
