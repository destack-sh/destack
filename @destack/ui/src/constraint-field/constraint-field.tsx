import { type Message, t } from "@destack/locale";
import { field } from "@destack/object";
import type { ConstraintKind } from "@destack/style";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { createControllableSignal, type JSX, omit, untrack, useLocale } from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { useFieldControl } from "../field/control.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** How a box follows its container across and down. */
export interface Constraints {
    /** How it follows its container across. */
    readonly x: ConstraintKind;
    /** How it follows its container down. */
    readonly y: ConstraintKind;
}

/** An edge of a box, which a constraint pins it to: a line's start or end, its top or bottom. */
type Edge = "start" | "end" | "top" | "bottom";

/** The names of pinning a box to each side of its container, as a person sees the sides. */
const PIN = {
    top: t.context("action")`Pin to the top`,
    left: t.context("action")`Pin to the left`,
    right: t.context("action")`Pin to the right`,
    bottom: t.context("action")`Pin to the bottom`,
};

/** The axis and end of each edge, and its name in each direction of text. */
const EDGES: Readonly<
    Record<
        Edge,
        {
            readonly axis: "x" | "y";
            readonly end: "start" | "end";
            readonly title: { readonly ltr: Message; readonly rtl: Message };
        }
    >
> = {
    top: { axis: "y", end: "start", title: { ltr: PIN.top, rtl: PIN.top } },
    start: { axis: "x", end: "start", title: { ltr: PIN.left, rtl: PIN.right } },
    end: { axis: "x", end: "end", title: { ltr: PIN.right, rtl: PIN.left } },
    bottom: { axis: "y", end: "end", title: { ltr: PIN.bottom, rtl: PIN.bottom } },
};

/** The constraints a field starts at, pinned to the top left, as design tools start a layer. */
const START: Constraints = { x: "start", y: "start" };

/** The properties people change in a constraint field's controls and inspector. */
const CONSTRAINT_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses changes where the constraints cannot change now.`)
        .default(false),
};

/** The styles of a constraint field. */
const styles = style.create({
    field: {
        display: "grid",
        gridTemplateColumns: `${size[1]} minmax(0, 1fr) ${size[1]}`,
        gridTemplateRows: `${space[5]} minmax(${space[6]}, 1fr) ${space[5]}`,
        width: "100%",
        height: "100%",
        borderRadius: radius[2],
        backgroundColor: color.muted,
    },
    frame: {
        gridArea: "2 / 2",
        display: "grid",
        placeItems: "center",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[1],
        backgroundColor: color.background,
    },
    edge: {
        display: "grid",
        placeItems: "center",
        padding: 0,
        borderWidth: 0,
        backgroundColor: "transparent",
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.foreground },
        },
        cursor: "pointer",
        outlineOffset: `calc(-1 * ${stroke.ring})`,
    },
    top: { gridArea: "1 / 2" },
    bottom: { gridArea: "3 / 2" },
    start: { gridArea: "2 / 1" },
    end: { gridArea: "2 / 3" },
    across: {
        width: space[1],
        height: space[3],
        borderRadius: radius.full,
        backgroundColor: "currentColor",
    },
    down: {
        width: space[3],
        height: space[1],
        borderRadius: radius.full,
        backgroundColor: "currentColor",
    },
    pinned: { color: color.foreground },
    center: {
        display: "grid",
        placeItems: "center",
        width: space[5],
        height: space[5],
        padding: 0,
        borderWidth: 0,
        backgroundColor: "transparent",
        color: color.mutedForeground,
        cursor: "pointer",
    },
    cross: {
        position: "relative",
        width: space[3],
        height: space[3],
        backgroundImage:
            "linear-gradient(currentColor, currentColor), linear-gradient(currentColor, currentColor)",
        backgroundSize: `100% ${stroke.border}, ${stroke.border} 100%`,
        backgroundPosition: "center",
        backgroundRepeat: "no-repeat",
    },
    disabled: {
        opacity: 0.5,
        pointerEvents: "none",
    },
});

/** The properties of a constraint field besides its declared ones: its constraints. */
export type ConstraintFieldAttributes = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onChange"
> &
    ElementPartProperties & {
        /** The constraints, which make it controlled. */
        readonly value?: Constraints;
        /** The constraints at first while uncontrolled, the top left by default. */
        readonly defaultValue?: Constraints;
        /** Handle a person pinning or unpinning an edge, or centering. */
        readonly onValueChange?: (value: Constraints) => void;
    };

/** Render a box whose edges and center pin it to its container, as design tools constrain. */
export const ConstraintField = defineComponent({
    name: "constraintField",
    properties: CONSTRAINT_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<ConstraintFieldAttributes, typeof CONSTRAINT_FIELD_PROPERTIES>,
    ): JSX.Element {
        // follow the controlled constraints, else the field's own
        const locale = useLocale();
        const fieldControl = useFieldControl();
        const [value, setValue] = createControllableSignal<Constraints>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? START,
            defaultValue: untrack(() => properties.defaultValue) ?? START,
            onChange: (next) => properties.onValueChange?.(next),
        });
        const rest = omit(properties, "value", "defaultValue", "onValueChange", "disabled");

        // tell whether an edge is pinned
        const isPinned = (edge: Edge): boolean => {
            const kind = value()[EDGES[edge].axis];

            return kind === "both" || kind === EDGES[edge].end;
        };

        // pin or unpin an edge, an axis pinned to neither edge centered
        const toggle = (edge: Edge): void => {
            // read the edge's axis, and which of its two edges are pinned after
            const { axis, end } = EDGES[edge];
            const other: Edge = otherOf(edge);
            const isStart = end === "start" ? !isPinned(edge) : isPinned(other);
            const isEnd = end === "end" ? !isPinned(edge) : isPinned(other);
            setValue({ ...value(), [axis]: kindOf(isStart, isEnd) });
        };

        // draw an edge as a button that pins it, and tell whether both axes are centered
        const isCentered = (): boolean => value().x === "center" && value().y === "center";
        const edge = (side: Edge, mark: style.Styles) => (
            <button
                type="button"
                data-slot="constraint-field-edge"
                aria-pressed={isPinned(side) ? "true" : "false"}
                aria-label={locale.render(EDGES[side].title[locale.direction])}
                disabled={properties.disabled}
                onClick={() => toggle(side)}
                {...style.attrs(styles.edge, styles[side], isPinned(side) && styles.pinned)}
            >
                <span {...style.attrs(mark)} />
            </button>
        );

        return renderPart(
            "div",
            "constraint-field",
            rest,
            () => [styles.field, properties.disabled && styles.disabled],
            {
                role: "group" as const,
                get id() {
                    return fieldControl?.groupAttributes().id;
                },
                get "aria-labelledby"() {
                    return fieldControl?.groupAttributes()["aria-labelledby"];
                },
                get children() {
                    return (
                        <>
                            {/* Edges */}
                            {edge("top", styles.across)}
                            {edge("start", styles.down)}
                            {edge("end", styles.down)}
                            {edge("bottom", styles.across)}

                            {/* The box, centered by its middle */}
                            <span {...style.attrs(styles.frame)}>
                                <button
                                    type="button"
                                    data-slot="constraint-field-center"
                                    aria-pressed={isCentered() ? "true" : "false"}
                                    aria-label={locale.render(t.context("action")`Center`)}
                                    disabled={properties.disabled}
                                    onClick={() => setValue({ x: "center", y: "center" })}
                                    {...style.attrs(styles.center, isCentered() && styles.pinned)}
                                >
                                    <span {...style.attrs(styles.cross)} />
                                </button>
                            </span>
                        </>
                    );
                },
            },
        );
    },
});

/** The properties of a constraint field, its constraints included. */
export type ConstraintFieldProperties = ComponentProperties<typeof ConstraintField>;

/** Read the edge across from another. */
function otherOf(edge: Edge): Edge {
    const others: Readonly<Record<Edge, Edge>> = {
        start: "end",
        end: "start",
        top: "bottom",
        bottom: "top",
    };

    return others[edge];
}

/** Read the constraint of an axis from which of its edges are pinned, centered where neither is. */
function kindOf(isStart: boolean, isEnd: boolean): ConstraintKind {
    if (isStart && isEnd) {
        return "both";
    } else if (isStart) {
        return "start";
    } else if (isEnd) {
        return "end";
    }

    return "center";
}
