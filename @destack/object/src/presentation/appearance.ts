import { SwatchName } from "@destack/color";
import type { JsonObject } from "@destack/schema";
import type { ObjectType } from "../object/object.ts";
import { Icon, Presentation } from "../trait/presentable.ts";

/** How an object looks wherever it shows: its title, icon and accent. */
export interface ObjectAppearance {
    /** The object's title, else its identifier. */
    readonly title: string;
    /** The object's icon, none when its type shows none. */
    readonly icon: Icon | undefined;
    /** The object's accent, none when its type colors none. */
    readonly color: SwatchName | undefined;
}

/** How an object looks, read from its row through its type's presentation. */
export const ObjectAppearance = {
    /** Read an object's appearance from its row. */
    of(type: ObjectType, row: JsonObject): ObjectAppearance {
        // read the icon from its field or the type's fixed one, and the accent from its field
        const { presentation } = type;
        const icon = presentation?.icon;
        const stored = icon === undefined || "fixed" in icon ? undefined : row[icon.field];
        const color = presentation?.color === undefined ? undefined : row[presentation.color.field];

        return {
            title: Presentation.title(presentation, row),
            icon:
                icon === undefined
                    ? undefined
                    : "fixed" in icon
                      ? { icon: icon.fixed }
                      : stored === null || stored === undefined
                        ? undefined
                        : Icon.parse(stored),
            color: color === null || color === undefined ? undefined : SwatchName.parse(color),
        };
    },
};
