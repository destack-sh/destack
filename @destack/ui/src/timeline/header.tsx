import { state } from "@destack/theme/state";
import { type Message, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, size, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createSignal, For, type JSX, omit, Show, useEnvironment, useLocale } from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import { Move, Sweep } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { ToggleGroup, ToggleGroupItem, type ToggleGroupProperties } from "../toggle-group/index.ts";
import {
    TimelineTrackContext,
    useTimeline,
    type TimelinePartProperties,
    MIN_LABEL_WIDTH,
} from "./control.ts";
import { TIMELINE_ZOOM_LEVELS, type TimelineZoomLevel } from "./window.ts";
import { places } from "./places.ts";

/** The least share of the axis a sweep zooms to, past which a press is a click. */
const MIN_SWEEP = 0.01;

/** The label of each zoom level's preset. */
const LEVEL_LABELS: Readonly<Record<TimelineZoomLevel, Message>> = {
    week: t`Week`,
    month: t`Month`,
    quarter: t`Quarter`,
};

/** The pixels an arrow key resizes the labels' column by. */
const RESIZE_STEP = 16;

/** The styles of a timeline's header. */
const styles = style.create({
    header: {
        position: "relative",
        display: "grid",
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    corner: {
        display: "flex",
        alignItems: "center",
        minWidth: 0,
        paddingInline: space[2],
        borderInlineEndWidth: stroke.border,
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
        color: color.mutedForeground,
    },
    column: {
        display: "flex",
        alignItems: "center",
        minWidth: 0,
        paddingInline: space[2],
        overflow: "hidden",
        borderInlineEndWidth: stroke.border,
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
        color: color.mutedForeground,
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
    },
    axis: {
        position: "relative",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        cursor: "zoom-in",
        userSelect: "none",
    },
    sweep: {
        position: "absolute",
        insetBlock: 0,
        backgroundColor: { default: swatch.accentTint, [media.forcedColors]: "Highlight" },
        pointerEvents: "none",
    },
    axisRow: {
        position: "relative",
        height: size[1],
    },
    period: {
        position: "absolute",
        insetBlock: 0,
        display: "flex",
        alignItems: "center",
        overflow: "hidden",
        paddingInlineStart: space[2],
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
        color: color.foreground,
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
    },
    mark: {
        minWidth: space[5],
        paddingInline: space[1],
        borderRadius: radius.full,
        textAlign: "center",
    },
    tick: {
        position: "absolute",
        insetBlock: 0,
        display: "flex",
        alignItems: "center",
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
        color: color.mutedForeground,
        whiteSpace: "nowrap",
        fontVariantNumeric: "tabular-nums",
    },
    resize: {
        position: "absolute",
        insetBlock: 0,
        zIndex: 2,
        width: space[2],
        marginInlineStart: `calc(${space[2]} / -2)`,
        cursor: "col-resize",
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: swatch.accentMark },
            ":focus-visible": swatch.accentMark,
            ":active": swatch.accentMark,
        },
        backgroundClip: "content-box",
        paddingInline: `calc(${space[2]} / 2 - ${stroke.ring} / 2)`,
    },
    toolbar: {
        display: "flex",
        alignItems: "center",
        flexWrap: "wrap",
        gap: space[2],
        padding: space[2],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
});

/** Render the header: a corner above the labels, and the axis by default. */
export function TimelineHeader(
    properties: TimelinePartProperties & {
        /** What stands above the labels' column. */
        readonly corner?: JSX.Element;
    },
): JSX.Element {
    // lay the corner and the axis out beside the labels' column
    const control = useTimeline();
    const locale = useLocale();
    const rest = omit(properties, "corner", "children", "xstyle", "style");

    // measure the labels' column and the header, which the edge reports and is bounded by
    const [corner, setCorner] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const [header, setHeader] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const cornerSize = createElementSize(corner);
    const headerSize = createElementSize(header);

    // drag the labels' edge by the pointer's travel, or a step per arrow key, mirrored right to left
    let start = 0;
    const resize = new Move(
        {
            orientation: "horizontal",
            onMoveStart: () => {
                start = widthOf(corner());
            },
            onMove: (event) => {
                const travel = locale.direction === "rtl" ? -event.offsetX : event.offsetX;
                const step = event.pointerType === "keyboard" ? travel * RESIZE_STEP : travel;
                control.resizeLabels(start + step, widthOf(header()));
            },
        },
        useEnvironment().clock,
    );

    return (
        <div
            data-slot="timeline-header"
            ref={setHeader}
            {...rest}
            {...style.attributes(
                [styles.header, places.columns(control.template()), properties.xstyle],
                properties.style,
            )}
        >
            <div
                data-slot="timeline-corner"
                ref={setCorner}
                {...style.attrs(text.xs, styles.corner)}
            >
                {properties.corner}
            </div>
            {"children" in properties ? properties.children : <TimelineAxis />}

            {/* The edge of the labels' column, dragged or stepped by the arrow keys */}
            <div
                role="separator"
                tabindex={0}
                aria-orientation="vertical"
                aria-label={locale.render(t`Resize the labels`)}
                aria-valuemin={MIN_LABEL_WIDTH}
                aria-valuenow={cornerSize.width === null ? undefined : Math.round(cornerSize.width)}
                aria-valuemax={
                    headerSize.width === null ? undefined : Math.round(headerSize.width / 2)
                }
                data-slot="timeline-resize"
                {...resize.attributes()}
                {...style.attrs(state.focus, styles.resize, places.after(control.labelWidth()))}
            />
        </div>
    );
}

/** Render a bar of controls above the axis, such as Today and the zoom presets. */
export function TimelineToolbar(properties: TimelinePartProperties): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            role="toolbar"
            aria-label={locale.render(t`Timeline`)}
            data-slot="timeline-toolbar"
            {...rest}
            {...style.attributes([styles.toolbar, properties.xstyle], properties.style)}
        />
    );
}

/** Render the zoom presets, a week, a month or a quarter per column, the one shown pressed. */
export function TimelineZoom(
    properties: Omit<
        ToggleGroupProperties,
        "value" | "defaultValue" | "onValueChange" | "behavior" | "multiple" | "children"
    >,
): JSX.Element {
    const control = useTimeline();
    const locale = useLocale();

    return (
        <ToggleGroup
            variant="outline"
            aria-label={locale.render(t`Zoom`)}
            data-slot="timeline-zoom"
            {...properties}
            behavior="replace"
            disallowEmptySelection
            value={control.window.level()}
            onValueChange={(value) => {
                // zoom to the preset the pressed item stands for
                const level = TIMELINE_ZOOM_LEVELS.find((each) => each === value);
                if (level === undefined) {
                    throw new RangeError(`timeline zoom level '${value}' is invalid`);
                }
                control.window.zoomTo(level);
            }}
        >
            <For each={TIMELINE_ZOOM_LEVELS}>
                {(level) => (
                    <ToggleGroupItem value={level}>
                        {locale.render(LEVEL_LABELS[level])}
                    </ToggleGroupItem>
                )}
            </For>
        </ToggleGroup>
    );
}

/** Render the button that brings today to the middle of the window. */
export function TimelineToday(properties: Omit<ButtonProperties, "onClick">): JSX.Element {
    // read today from the clock when the person asks, the ticking now once mounted
    const control = useTimeline();
    const locale = useLocale();
    const { clock } = useEnvironment();

    return (
        <Button
            variant="outline"
            data-slot="timeline-today"
            {...properties}
            onClick={() => control.window.reveal(control.window.now() ?? clock.now())}
        >
            {properties.children ?? locale.render(t`Today`)}
        </Button>
    );
}

/** Render the axis in two tiers: the periods, such as months, above their marks, such as days. */
export function TimelineAxis(properties: TimelinePartProperties): JSX.Element {
    // place the periods and the marks by their share of the window
    const control = useTimeline();
    const locale = useLocale();
    const rest = omit(properties, "children", "xstyle", "style");
    const share = (at: number) => control.window.share(at);

    // sweep a span across the axis and zoom to it on release
    let element: HTMLElement | undefined;
    const shareAt = (clientX: number): number => {
        // read the pointer's share of the measured axis, mirrored right to left
        const bounds = element?.getBoundingClientRect();
        if (bounds === undefined || bounds.width === 0) {
            throw new RangeError("timeline axis is invalid: it is not measured yet");
        }
        const along = (clientX - bounds.left) / bounds.width;

        return locale.direction === "rtl" ? 1 - along : along;
    };
    const zoom = new Sweep(
        {
            at: (x) => shareAt(x),
            onSweep: (swept) => {
                // zoom to a span wider than a sliver, leaving a click alone
                if (swept.end - swept.start > MIN_SWEEP) {
                    control.window.show({
                        start: control.window.instant(swept.start),
                        end: control.window.instant(swept.end),
                    });
                }
            },
        },
        useEnvironment().clock,
    );

    return (
        <div
            data-slot="timeline-axis"
            {...mergeProperties(
                mergeProperties(
                    { ref: (created: HTMLElement) => (element = created) },
                    zoom.attributes(),
                ),
                rest,
            )}
            {...style.attributes([styles.axis, properties.xstyle], properties.style)}
        >
            {/* The span a sweep zooms to */}
            <Show when={zoom.span()}>
                {(swept) => (
                    <span
                        data-slot="timeline-axis-sweep"
                        {...style.attrs(
                            styles.sweep,
                            places.span(swept().start, swept().end - swept().start),
                        )}
                    />
                )}
            </Show>

            {/* Periods */}
            <Show when={control.window.periods().length > 0}>
                <div
                    data-slot="timeline-axis-periods"
                    aria-hidden="true"
                    {...style.attrs(styles.axisRow)}
                >
                    <For each={control.window.periods()} keyed={(period) => period.start}>
                        {(period) => (
                            <span
                                data-slot="timeline-axis-period"
                                {...style.attrs(
                                    text.xs,
                                    styles.period,
                                    places.span(
                                        share(period().start),
                                        share(period().end) - share(period().start),
                                    ),
                                )}
                            >
                                {period().label}
                            </span>
                        )}
                    </For>
                </div>
            </Show>

            {/* Marks */}
            <div
                data-slot="timeline-axis-marks"
                aria-hidden="true"
                {...style.attrs(styles.axisRow)}
            >
                <For each={control.window.ticks()} keyed={(tick) => tick.at}>
                    {(tick) => (
                        <span {...style.attrs(text.xs, styles.tick, places.at(share(tick().at)))}>
                            <span
                                data-slot="timeline-axis-mark"
                                data-today={tick().isCurrent === true ? "" : undefined}
                                {...style.attrs(
                                    styles.mark,
                                    tick().isCurrent === true && state.checked,
                                )}
                            >
                                {control.window.tickLabel(tick())}
                            </span>
                        </span>
                    )}
                </For>
            </div>

            {/* What stands on the axis, such as a playhead or a loop */}
            <TimelineTrackContext value={true}>{properties.children}</TimelineTrackContext>
        </div>
    );
}

/** Measure a mounted part of the header's width, refusing one not yet mounted. */
function widthOf(element: HTMLElement | undefined): number {
    if (element === undefined) {
        throw new TypeError("timeline labels resized before the header mounted");
    }

    return element.getBoundingClientRect().width;
}

/** The properties of a column beside a timeline's labels. */
export interface TimelineColumnProperties extends TimelinePartProperties {
    /** The column's width, such as 6rem. */
    readonly width: string;
}

/** Render the header of a column beside the labels, whose cells each row renders. */
export function TimelineColumn(properties: TimelineColumnProperties): JSX.Element {
    // take the column's width in the header's and every row's grid, in the headers' order
    const control = useTimeline();
    const [element, setElement] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    control.addColumn({ element, width: () => properties.width });
    const rest = omit(properties, "width", "xstyle", "style", "ref");

    return (
        <div
            ref={[properties.ref, setElement]}
            role="columnheader"
            data-slot="timeline-column"
            {...rest}
            {...style.attributes([text.xs, styles.column, properties.xstyle], properties.style)}
        />
    );
}
