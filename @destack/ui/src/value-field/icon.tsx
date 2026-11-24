import type { IconBodies } from "@destack/icon";
import bracesIcon from "@destack/icon/phosphor/brackets-curly";
import calendarIcon from "@destack/icon/phosphor/calendar-blank";
import choiceIcon from "@destack/icon/phosphor/caret-circle-down";
import checkIcon from "@destack/icon/phosphor/check-square";
import cubeIcon from "@destack/icon/phosphor/cube";
import hashIcon from "@destack/icon/phosphor/hash";
import textIcon from "@destack/icon/phosphor/text-aa";
import userIcon from "@destack/icon/phosphor/user";
import type { JSX } from "@destack/view";
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

/** The properties of a value's icon, with its icon's own. */
export type ValueIconProperties = Omit<IconProperties, "name" | "icon"> & {
    /** The icon in place of its kind's. */
    readonly icon?: IconBodies | undefined;
    /** The kind of value. */
    readonly kind: ValueKind;
};

/** Render the icon of a kind of value, another where it is given. */
export function ValueIcon(properties: ValueIconProperties): JSX.Element {
    return <Icon {...properties} icon={properties.icon ?? ICONS[properties.kind]} />;
}
