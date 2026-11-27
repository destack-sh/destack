import type { IconBodies } from "@destack/icon";
import penIcon from "@destack/icon/phosphor/pen";
import slidersHorizontalIcon from "@destack/icon/phosphor/sliders-horizontal";
import textSuperscriptIcon from "@destack/icon/phosphor/text-superscript";
import textStrikethroughIcon from "@destack/icon/phosphor/text-strikethrough";
import textAlignLeftIcon from "@destack/icon/phosphor/text-align-left";
import fadersIcon from "@destack/icon/phosphor/faders";
import funnelIcon from "@destack/icon/phosphor/funnel";
import cropIcon from "@destack/icon/phosphor/crop";
import circleHalfIcon from "@destack/icon/phosphor/circle-half";
import crosshairIcon from "@destack/icon/phosphor/crosshair";
import resizeIcon from "@destack/icon/phosphor/resize";
import arrowClockwiseIcon from "@destack/icon/phosphor/arrow-clockwise";
import arrowsOutIcon from "@destack/icon/phosphor/arrows-out";
import bezierCurveIcon from "@destack/icon/phosphor/bezier-curve";
import bracesIcon from "@destack/icon/phosphor/brackets-curly";
import calendarIcon from "@destack/icon/phosphor/calendar-blank";
import choiceIcon from "@destack/icon/phosphor/caret-circle-down";
import checkIcon from "@destack/icon/phosphor/check-square";
import copySimpleIcon from "@destack/icon/phosphor/copy-simple";
import cubeIcon from "@destack/icon/phosphor/cube";
import frameCornersIcon from "@destack/icon/phosphor/frame-corners";
import gradientIcon from "@destack/icon/phosphor/gradient";
import calendarDotsIcon from "@destack/icon/phosphor/calendar-dots";
import coinsIcon from "@destack/icon/phosphor/coins";
import linkIcon from "@destack/icon/phosphor/link";
import repeatIcon from "@destack/icon/phosphor/repeat";
import starIcon from "@destack/icon/phosphor/star";
import hashIcon from "@destack/icon/phosphor/hash";
import hourglassIcon from "@destack/icon/phosphor/hourglass";
import paintBucketIcon from "@destack/icon/phosphor/paint-bucket";
import paletteIcon from "@destack/icon/phosphor/palette";
import rowsIcon from "@destack/icon/phosphor/rows";
import rulerIcon from "@destack/icon/phosphor/ruler";
import selectionIcon from "@destack/icon/phosphor/selection";
import selectionBackgroundIcon from "@destack/icon/phosphor/selection-background";
import stackIcon from "@destack/icon/phosphor/stack";
import swatchesIcon from "@destack/icon/phosphor/swatches";
import textIcon from "@destack/icon/phosphor/text-aa";
import textBIcon from "@destack/icon/phosphor/text-b";
import textItalicIcon from "@destack/icon/phosphor/text-italic";
import textTIcon from "@destack/icon/phosphor/text-t";
import timerIcon from "@destack/icon/phosphor/timer";
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
    "style.color": paletteIcon,
    "style.palette": swatchesIcon,
    "style.gradient": gradientIcon,
    "style.paint": paintBucketIcon,
    "style.fillStack": stackIcon,
    "style.dimension": rulerIcon,
    "style.cubicBezier": bezierCurveIcon,
    "style.fontFamily": penIcon,
    "style.fontWeight": textBIcon,
    "style.fontStyle": textItalicIcon,
    "style.fontFeatures": textSuperscriptIcon,
    "style.fontVariations": slidersHorizontalIcon,
    "style.corners": frameCornersIcon,
    "style.boxSides": arrowsOutIcon,
    "style.size": resizeIcon,
    "style.position": crosshairIcon,
    "style.opacity": circleHalfIcon,
    "style.overflow": cropIcon,
    "style.stroke": selectionIcon,
    "style.shadow": selectionBackgroundIcon,
    "style.shadowStack": copySimpleIcon,
    "style.filter": funnelIcon,
    "style.filterStack": fadersIcon,
    "style.typography": textTIcon,
    "style.textFormat": textAlignLeftIcon,
    "style.textStroke": textStrikethroughIcon,
    "style.transform": arrowClockwiseIcon,
    "style.layout": rowsIcon,
    "style.transition": timerIcon,
    "schema.duration": hourglassIcon,
    "schema.dateRange": calendarDotsIcon,
    "schema.recurrence": repeatIcon,
    "schema.money": coinsIcon,
    "schema.rating": starIcon,
    "schema.url": linkIcon,
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
