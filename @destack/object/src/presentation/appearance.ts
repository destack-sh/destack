import { Hue } from "@destack/color";
import { Condition, type Moment } from "@destack/db";
import { type JsonObject, schema } from "@destack/schema";
import type { Localization } from "@destack/locale";
import type { ObjectType } from "../object/object.ts";
import { Numbered } from "../trait/numbered.ts";
import { Icon, Presentation } from "../trait/presentable.ts";

/** How an object looks wherever it shows: its title, icon and color. */
export interface ObjectAppearance {
    /** The object's title, else its identifier. */
    readonly title: string;
    /** The object's icon, none when its type shows none. */
    readonly icon: Icon | undefined;
    /** The object's color, stored or by the first condition it meets, none without one. */
    readonly color: Hue | undefined;
}

/** How an object looks, read from its row through its type's presentation. */
export const ObjectAppearance = {
    /** Read an object's appearance from its row at a moment, its title in a locale. */
    of(
        type: ObjectType,
        row: JsonObject,
        moment: Moment,
        localization: Localization,
        key: string | undefined,
    ): ObjectAppearance {
        // read the icon from its field or the type's fixed one
        const { presentation } = type;
        const icon = presentation.icon;
        const stored = icon === undefined || "fixed" in icon ? undefined : row[icon.field];

        // read the color from its field, else from the first condition the row meets
        const colored = presentation.color;
        const color =
            colored === undefined
                ? undefined
                : "field" in colored
                  ? row[colored.field]
                  : colored.conditions.find((each) =>
                        Condition.matches(each.where, type.table, row, moment),
                    )?.swatch;

        return {
            title: ObjectAppearance.title(type, row, localization, key),
            icon:
                icon === undefined
                    ? undefined
                    : "fixed" in icon
                      ? { icon: icon.fixed }
                      : stored === null || stored === undefined
                        ? undefined
                        : Icon.parse(stored),
            color: color === null || color === undefined ? undefined : Hue.parse(color),
        };
    },

    /** Read an object's title, else the identifier people use for it, such as TRK-214. */
    title(
        type: ObjectType,
        row: JsonObject,
        localization: Localization,
        key: string | undefined,
    ): string {
        return (
            Presentation.title(type.presentation, row, localization) ??
            ObjectAppearance.identifier(type, row, key)
        );
    },

    /** Read the identifier people use for an object: its number's, such as TRK-214, else its id. */
    identifier(type: ObjectType, row: JsonObject, key: string | undefined): string {
        // read the id for a type without numbering, or an object waiting for its number
        const numbering = type.numbering;
        const id = schema.string().parse(row["id"]);
        const number = row["number"];
        if (numbering === undefined || typeof number !== "number") {
            return id;
        }
        // read a frozen identifier as issued
        else if (numbering.identifier === "frozen") {
            return schema.string().parse(row["identifier"]);
        }
        // refuse a live identifier without the key of the object it counts within
        else if (key === undefined && Numbered.withinType(type) !== undefined) {
            throw new TypeError(
                `the identifier of ${type.name} '${id}' needs the key it counts within`,
            );
        }

        return Numbered.format(numbering, key, number);
    },
};
