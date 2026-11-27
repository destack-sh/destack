import type { IconBodies, IconName } from "@destack/icon";
import { type JsonObject, LocalReference } from "@destack/schema";
import { type JSX, omit, Show } from "@destack/view";
import { Icon, type IconProperties, LazyIcon } from "../icon/index.ts";
import type { ValueKind } from "./value-field.tsx";

/** The icon of each kind of value, by its Phosphor name, loaded on first use. */
const ICONS: Readonly<Record<ValueKind, IconName>> = {
    text: "text-aa",
    number: "hash",
    boolean: "check-square",
    time: "calendar-blank",
    choice: "caret-circle-down",
    person: "user",
    object: "cube",
    json: "brackets-curly",
};

/** The icon of each named schema, such as a color's, in place of the JSON kind's, by name. */
const NAMED_ICONS: Readonly<Partial<Record<string, IconName>>> = {
    "style.color": "palette",
    "style.palette": "swatches",
    "style.gradient": "gradient",
    "style.paint": "paint-bucket",
    "style.fillStack": "stack",
    "style.dimension": "ruler",
    "style.cubicBezier": "bezier-curve",
    "style.fontFamily": "pen",
    "style.fontWeight": "text-b",
    "style.fontStyle": "text-italic",
    "style.fontFeatures": "text-superscript",
    "style.fontVariations": "sliders-horizontal",
    "style.corners": "frame-corners",
    "style.boxSides": "arrows-out",
    "style.size": "resize",
    "style.position": "crosshair",
    "style.opacity": "circle-half",
    "style.overflow": "crop",
    "style.stroke": "selection",
    "style.shadow": "selection-background",
    "style.shadowStack": "copy-simple",
    "style.filter": "funnel",
    "style.filterStack": "faders",
    "style.typography": "text-t",
    "style.textFormat": "text-align-left",
    "style.textStroke": "text-strikethrough",
    "style.transform": "arrow-clockwise",
    "style.layout": "rows",
    "style.transition": "timer",
    "schema.duration": "hourglass",
    "schema.dateRange": "calendar-dots",
    "schema.recurrence": "repeat",
    "schema.money": "coins",
    "schema.rating": "star",
    "schema.url": "link",
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
    const name = (): IconName => {
        const type = properties.type;
        const schema = type.kind === "json" ? LocalReference.root(type.schema) : null;

        return (schema === null ? undefined : NAMED_ICONS[schema]) ?? ICONS[type.kind];
    };
    const rest = omit(properties, "type", "icon");

    // draw an icon given as it is, and the type's by name, its module loaded on first use
    return (
        <Show when={properties.icon} fallback={<LazyIcon {...rest} name={name()} />}>
            {(icon) => <Icon {...rest} icon={icon()} />}
        </Show>
    );
}
