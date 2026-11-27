import { type Message, t } from "@destack/locale";
import { field } from "@destack/object";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import type { CubicBezier } from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createControllableSignal, For, type JSX, omit, untrack, useLocale } from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { useFieldControl } from "../field/control.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The room the plot shows around the curve's square on every side, for a curve that overshoots. */
const ROOM = 0.25;

/** The lowest and highest coordinate the plot shows, across and up, the curve's square between. */
const RANGE = { min: -ROOM, max: 1 + ROOM } as const;

/** The least width of a curve field, room to drag its points where nothing sizes it. */
const MIN_WIDTH = "10rem";

/** The quarters of the curve's square, where its grid lines run. */
const QUARTERS = [0.25, 0.5, 0.75] as const;

/** CSS's ease, which a curve field starts at. */
const EASE: CubicBezier = [0.25, 0.1, 0.25, 1];

/** The hundredths a control point keeps, as CSS curves are written. */
const PRECISION = 100;

/** How far an arrow moves a control point, and with Shift held, as design tools step. */
const STEPS = { small: 0.01, large: 0.1 } as const;

/** The two control points, with the indices of their x and y in a curve. */
const POINTS = [
    { x: 0, y: 1, name: t.context("label")`First control point` },
    { x: 2, y: 3, name: t.context("label")`Second control point` },
] as const satisfies readonly { readonly x: number; readonly y: number; readonly name: Message }[];

/** The styles of a curve field. */
const styles = style.create({
    field: {
        position: "relative",
        width: "100%",
        minWidth: MIN_WIDTH,
        aspectRatio: "1",
        borderRadius: radius[2],
        backgroundColor: color.muted,
        touchAction: "none",
    },
    plot: {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        overflow: "visible",
    },
    curve: {
        fill: "none",
        stroke: color.foreground,
        strokeWidth: 2.5,
        strokeLinecap: "round",
    },
    arm: {
        stroke: color.mutedForeground,
        strokeWidth: 1.5,
    },
    // the curve's square: its time across and its progress up, lighter than the room around
    box: {
        fill: color.background,
    },
    grid: {
        stroke: color.border,
        strokeWidth: 1,
    },
    end: {
        position: "absolute",
        width: space[2],
        height: space[2],
        translate: "-50% -50%",
        borderRadius: radius.full,
        backgroundColor: color.foreground,
        pointerEvents: "none",
    },
    point: (left: string, top: string) => ({
        left,
        top,
    }),
    handle: {
        position: "absolute",
        boxSizing: "border-box",
        width: space[4],
        height: space[4],
        translate: "-50% -50%",
        padding: 0,
        borderWidth: stroke.ring,
        borderStyle: "solid",
        borderColor: color.background,
        borderRadius: radius.full,
        backgroundColor: color.foreground,
        boxShadow: `0 0 0 1px ${color.border}`,
        cursor: { default: "grab", ":active": "grabbing" },
        outlineOffset: 2,
        scale: { default: null, ":hover": { default: null, [media.hover]: 1.15 } },
    },
    disabled: {
        opacity: 0.5,
        pointerEvents: "none",
    },
});

/** The properties people change in a curve field's controls and inspector. */
const CURVE_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses changes where the curve cannot change now.`)
        .default(false),
};

/** The properties of a curve field besides its declared ones: its curve. */
export type CurveFieldAttributes = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "onChange"> &
    ElementPartProperties & {
        /** The curve, which makes it controlled. */
        readonly value?: CubicBezier | undefined;
        /** The curve at first while uncontrolled, CSS's ease by default. */
        readonly defaultValue?: CubicBezier;
        /** Handle a person moving a control point. */
        readonly onValueChange?: (value: CubicBezier) => void;
    };

/** Render a cubic Bézier's plot, whose two control points a person drags or moves by arrows. */
export const CurveField = defineComponent({
    name: "curveField",
    properties: CURVE_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<CurveFieldAttributes, typeof CURVE_FIELD_PROPERTIES>,
    ): JSX.Element {
        return <CurvePlot {...properties} />;
    },
});

/** The properties of a curve field, its curve included. */
export type CurveFieldProperties = ComponentProperties<typeof CurveField>;

/** Render the plot of a curve field and follow its curve. */
function CurvePlot(
    properties: RenderProperties<CurveFieldAttributes, typeof CURVE_FIELD_PROPERTIES>,
): JSX.Element {
    // follow the controlled curve, else the field's own, CSS's ease at first
    const locale = useLocale();
    const fieldControl = useFieldControl();
    const [value, setValue] = createControllableSignal<CubicBezier>({
        isControlled: () => "value" in properties,
        value: () => properties.value ?? EASE,
        defaultValue: untrack(() => properties.defaultValue) ?? EASE,
        onChange: (next) => properties.onValueChange?.(next),
    });
    const rest = omit(properties, "value", "defaultValue", "onValueChange", "disabled");
    const at = (index: number): number => value()[index] ?? 0;
    let plot: HTMLElement | undefined;

    // move a control point
    const move = (point: (typeof POINTS)[number], x: number, y: number): void => {
        // keep x within the curve's time and y within the plot, in hundredths
        const next: [number, number, number, number] = [...value()];
        next[point.x] = rounded(Math.min(Math.max(x, 0), 1));
        next[point.y] = rounded(Math.min(Math.max(y, RANGE.min), RANGE.max));
        setValue(next);
    };

    // move a point to where a pointer drags it on the plot
    const drag = (point: (typeof POINTS)[number], event: PointerEvent): void => {
        // read the pointer's place as a share of the plot, y from the top of the plot's range
        const box = present(plot, "a curve field's plot").getBoundingClientRect();
        const x = RANGE.min + ((event.clientX - box.left) / box.width) * (RANGE.max - RANGE.min);
        const y = RANGE.max - ((event.clientY - box.top) / box.height) * (RANGE.max - RANGE.min);
        move(point, x, y);
    };

    // move a point by an arrow, further with Shift held
    const keys = (point: (typeof POINTS)[number], event: KeyboardEvent): void => {
        // read the move of each arrow, a larger one with Shift held
        const step = event.shiftKey ? STEPS.large : STEPS.small;
        const moves: Readonly<Record<string, readonly [number, number]>> = {
            ArrowRight: [step, 0],
            ArrowLeft: [-step, 0],
            ArrowUp: [0, step],
            ArrowDown: [0, -step],
        };

        // move the point by an arrow, leaving other keys to the page
        const by = moves[event.key];
        if (by !== undefined) {
            event.preventDefault();
            move(point, (value()[point.x] ?? 0) + by[0], (value()[point.y] ?? 0) + by[1]);
        }
    };

    return renderPart(
        "div",
        "curve-field",
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
            get "aria-disabled"() {
                return properties.disabled ? ("true" as const) : undefined;
            },
            ref: (element: HTMLElement) => (plot = element),
            get children() {
                return (
                    <>
                        {/* The unit box, the arms to the control points, and the curve */}
                        <svg
                            viewBox="0 0 100 100"
                            preserveAspectRatio="none"
                            aria-hidden="true"
                            {...style.attrs(styles.plot)}
                        >
                            <rect
                                x={plotX(0)}
                                y={plotY(1)}
                                width={plotX(1) - plotX(0)}
                                height={plotY(0) - plotY(1)}
                                {...style.attrs(styles.box)}
                            />
                            <For each={QUARTERS}>
                                {(quarter) => (
                                    <>
                                        <line
                                            x1={plotX(quarter)}
                                            y1={plotY(0)}
                                            x2={plotX(quarter)}
                                            y2={plotY(1)}
                                            vector-effect="non-scaling-stroke"
                                            {...style.attrs(styles.grid)}
                                        />
                                        <line
                                            x1={plotX(0)}
                                            y1={plotY(quarter)}
                                            x2={plotX(1)}
                                            y2={plotY(quarter)}
                                            vector-effect="non-scaling-stroke"
                                            {...style.attrs(styles.grid)}
                                        />
                                    </>
                                )}
                            </For>
                            <line
                                x1={plotX(0)}
                                y1={plotY(0)}
                                x2={plotX(at(0))}
                                y2={plotY(at(1))}
                                vector-effect="non-scaling-stroke"
                                {...style.attrs(styles.arm)}
                            />
                            <line
                                x1={plotX(1)}
                                y1={plotY(1)}
                                x2={plotX(at(2))}
                                y2={plotY(at(3))}
                                vector-effect="non-scaling-stroke"
                                {...style.attrs(styles.arm)}
                            />
                            <path
                                d={`M ${String(plotX(0))} ${String(plotY(0))} C ${String(plotX(at(0)))} ${String(plotY(at(1)))}, ${String(plotX(at(2)))} ${String(plotY(at(3)))}, ${String(plotX(1))} ${String(plotY(1))}`}
                                vector-effect="non-scaling-stroke"
                                {...style.attrs(styles.curve)}
                            />
                        </svg>

                        {/* The curve's ends, at no time and no progress, and at all of both */}
                        <span
                            {...style.attrs(
                                styles.end,
                                styles.point(`${String(plotX(0))}%`, `${String(plotY(0))}%`),
                            )}
                        />
                        <span
                            {...style.attrs(
                                styles.end,
                                styles.point(`${String(plotX(1))}%`, `${String(plotY(1))}%`),
                            )}
                        />

                        {/* The control points, dragged or moved by arrows */}
                        <For each={POINTS}>
                            {(point) => (
                                <button
                                    type="button"
                                    data-slot="curve-field-point"
                                    aria-label={`${locale.render(point.name)}: ${locale.number(value()[point.x] ?? 0)}, ${locale.number(value()[point.y] ?? 0)}`}
                                    disabled={properties.disabled}
                                    onPointerDown={(event) => {
                                        event.currentTarget.setPointerCapture(event.pointerId);
                                    }}
                                    onPointerMove={(event) => {
                                        if (
                                            event.currentTarget.hasPointerCapture(event.pointerId)
                                        ) {
                                            drag(point, event);
                                        }
                                    }}
                                    onKeyDown={(event) => keys(point, event)}
                                    {...style.attrs(
                                        styles.handle,
                                        styles.point(
                                            `${String(plotX(value()[point.x] ?? 0))}%`,
                                            `${String(plotY(value()[point.y] ?? 0))}%`,
                                        ),
                                    )}
                                />
                            )}
                        </For>
                    </>
                );
            },
        },
    );
}

/** Read where an x falls on the plot, in percent from its start. */
function plotX(x: number): number {
    return ((x - RANGE.min) / (RANGE.max - RANGE.min)) * 100;
}

/** Read where a y falls on the plot, in percent from its top, one past the range at its edge. */
function plotY(y: number): number {
    const shown = Math.min(Math.max(y, RANGE.min), RANGE.max);

    return ((RANGE.max - shown) / (RANGE.max - RANGE.min)) * 100;
}

/** Round a coordinate to the hundredths CSS curves are written in. */
function rounded(coordinate: number): number {
    return Math.round(coordinate * PRECISION) / PRECISION;
}
