import type { IconBodies } from "@destack/icon";
import bracesIcon from "@destack/icon/phosphor/brackets-curly";
import calendarIcon from "@destack/icon/phosphor/calendar-blank";
import choiceIcon from "@destack/icon/phosphor/caret-circle-down";
import checkIcon from "@destack/icon/phosphor/check-square";
import cubeIcon from "@destack/icon/phosphor/cube";
import gradientIcon from "@destack/icon/phosphor/gradient";
import hashIcon from "@destack/icon/phosphor/hash";
import paletteIcon from "@destack/icon/phosphor/palette";
import swatchesIcon from "@destack/icon/phosphor/swatches";
import textIcon from "@destack/icon/phosphor/text-aa";
import userIcon from "@destack/icon/phosphor/user";
import { type JsonObject, LocalReference } from "@destack/schema";
import { type JSX, omit } from "@destack/view";
import { Icon, type IconProperties } from "../icon/index.ts";
import type { ValueKind } from "./value-field.tsx";

/** The icon of each kind of value. */
const ICONS: Readonly<Record<ValueKind, IconBodies>> = {
    text: textIcon,
    number: hashIcon,
    boolean: checkIcon,
    time: calendarIcon,
    choice: choiceIcon,
    person: userIcon,
    object: cubeIcon,
    json: bracesIcon,
};

/** The icon of each named schema, such as a color's, shown in place of the JSON kind's. */
const NAMED_ICONS: Readonly<Partial<Record<string, IconBodies>>> = {
    color: paletteIcon,
    palette: swatchesIcon,
    gradient: gradientIcon,
};

/** The properties of a value's icon, with its icon's own. */
export type ValueIconProperties = Omit<IconProperties, "name" | "icon"> & {
    /** The icon in place of its kind's. */
    readonly icon?: IconBodies | undefined;
    /** What the value is: its kind, and the schema JSON of it meets. */
    readonly type:
        | { readonly kind: Exclude<ValueKind, "json"> }
        | { readonly kind: "json"; readonly schema: JsonObject };
};

/** Render the icon of a kind of value, another where it is given. */
export function ValueIcon(properties: ValueIconProperties): JSX.Element {
    // take a named schema's icon, such as a color's, else the kind's
    const icon = (): IconBodies => {
        const type = properties.type;
        const name = type.kind === "json" ? LocalReference.root(type.schema) : null;

        return (name === null ? undefined : NAMED_ICONS[name]) ?? ICONS[type.kind];
    };

    return <Icon {...omit(properties, "type")} icon={properties.icon ?? icon()} />;
}
