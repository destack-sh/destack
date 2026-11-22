import { state } from "@destack/theme/state";
import type { AccentPreset } from "@destack/color";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit, Show, untrack, useLocale } from "@destack/view";
import { Press } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import {
    type CalendarEvent,
    isAllDay,
    isTimed,
    type TimedCalendarEvent,
    useEventCalendar,
} from "./control.ts";

/** The width of a list row's time, room for a range such as "10:30 – 11:45 AM". */
const LIST_TIME_WIDTH = "9rem";

/** The color an event takes when it sets none. */
const DEFAULT_COLOR: AccentPreset = "blue";

/** The styles of an event. */
const styles = style.create({
    event: {
        display: "flex",
        gap: space[1],
        minWidth: 0,
        overflow: "hidden",
        borderRadius: radius[2],
        textAlign: "start",
        cursor: { default: "grab", ":active": "grabbing" },
        zIndex: 1,
    },
    inline: {
        alignItems: "center",
        marginInline: space[1],
        paddingInline: space[1],
        whiteSpace: "nowrap",
    },
    block: {
        flexWrap: "wrap",
        alignContent: "flex-start",
        columnGap: space[1],
        paddingBlock: space[1],
        paddingInline: space[2],
        borderInlineStartWidth: stroke.ring,
        borderInlineStartStyle: "solid",
    },
    list: {
        display: "grid",
        gridTemplateColumns: `${LIST_TIME_WIDTH} ${space[2]} minmax(0, 1fr)`,
        alignItems: "center",
        gap: space[2],
        width: "100%",
        paddingBlock: space[1],
        paddingInline: space[1],
        cursor: "default",
        backgroundColor: "transparent",
    },
    title: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        fontWeight: weight.medium,
    },
    dragging: {
        opacity: 0.5,
    },
    continues: {
        borderStartEndRadius: 0,
        borderEndEndRadius: 0,
    },
    continued: {
        borderStartStartRadius: 0,
        borderEndStartRadius: 0,
    },
    dot: {
        flexShrink: 0,
        width: space[2],
        height: space[2],
        borderRadius: radius.full,
    },
    time: {
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    background: {
        display: "flex",
        alignItems: "flex-start",
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        pointerEvents: "none",
    },
});

/** The colors an event takes from its swatch. */
const fills = style.create({
    tint: (tint: string, label: string) => ({ backgroundColor: tint, color: label }),
    edge: (solid: string) => ({ borderInlineStartColor: solid }),
    dot: (solid: string) => ({ backgroundColor: solid }),
    shade: (tint: string, label: string) => ({
        backgroundColor: `color-mix(in oklab, ${tint} 60%, transparent)`,
        color: label,
    }),
});

/** The properties of an event, the native button's attributes included. */
export interface EventCalendarEventProperties extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class"
> {
    /** The event. */
    readonly event: CalendarEvent;
    /** How it lays out: on a line in a month, as a block in a column, or as a row in a list. */
    readonly layout: "inline" | "block" | "list";
    /** Whether the event starts here rather than in an earlier row. */
    readonly isStart: boolean;
    /** Whether the event ends here rather than in a later row. */
    readonly isEnd: boolean;
    /** The StyleX styles applied after the event's, such as its place in the view. */
    readonly xstyle?: style.Styles;
}

/** Render an event: a tinted bar for a whole day, a dot and time on a row, a block in a column. */
export function EventCalendarEvent(properties: EventCalendarEventProperties): JSX.Element {
    // read the event's color, its time if any, and whether it is lifted
    const control = useEventCalendar();
    const locale = useLocale();
    const preset = (): AccentPreset => properties.event.color ?? DEFAULT_COLOR;
    const timed = (): TimedCalendarEvent | undefined =>
        isTimed(properties.event) ? properties.event : undefined;
    const time = (at: number): string =>
        locale.date(at, { hour: "numeric", minute: "2-digit", timeZone: control.timeZone() });
    const isDayLong = (): boolean => timed() !== undefined && isAllDay(properties.event);
    const rest = omit(
        properties,
        "event",
        "layout",
        "isStart",
        "isEnd",
        "xstyle",
        "style",
        "children",
    );

    // open the event on a press or Enter, and on Space in a list, where no drag takes it
    const press = new Press({
        onPress: () => control.act(properties.event.key),
        keys: untrack(() => properties.layout) === "list" ? ["Enter", " "] : ["Enter"],
    });
    const layouts = {
        inline: styles.inline,
        block: styles.block,
        list: [state.hover, styles.list],
    };
    const isTinted = () =>
        properties.layout === "block" ||
        (properties.layout === "inline" && (timed() === undefined || isDayLong()));

    return (
        <button
            type="button"
            data-slot="event-calendar-event"
            data-color={preset()}
            data-layout={properties.layout}
            data-dragging={control.manager.isDragging(properties.event.key) ? "" : undefined}
            {...mergeProperties(press.attributes(), rest)}
            {...style.attributes(
                [
                    text.xs,
                    state.focus,
                    styles.event,
                    layouts[properties.layout],
                    isTinted() &&
                        fills.tint(swatch[`${preset()}Tint`], swatch[`${preset()}TintText`]),
                    properties.layout === "block" && fills.edge(swatch[`${preset()}Mark`]),
                    !properties.isEnd && styles.continues,
                    !properties.isStart && styles.continued,
                    control.manager.isDragging(properties.event.key) && styles.dragging,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {/* An event in a list: its time or all day, then its color's dot */}
            <Show when={properties.layout === "list"}>
                <span {...style.attrs(styles.time)}>
                    <Show when={timed()} fallback={locale.render(t`All day`)}>
                        {(current) =>
                            locale.dateRange(current().start, current().end, {
                                hour: "numeric",
                                minute: "2-digit",
                                timeZone: control.timeZone(),
                            })
                        }
                    </Show>
                </span>
                <span {...style.attrs(styles.dot, fills.dot(swatch[`${preset()}Mark`]))} />
            </Show>

            {/* A timed event on a row: its color's dot and its start */}
            <Show when={properties.layout === "inline" ? timed() : undefined}>
                {(current) => (
                    <>
                        <Show when={!isDayLong()}>
                            <span
                                {...style.attrs(styles.dot, fills.dot(swatch[`${preset()}Mark`]))}
                            />
                        </Show>
                        <span {...style.attrs(styles.time)}>{time(current().start)}</span>
                    </>
                )}
            </Show>

            {/* Title */}
            <span {...style.attrs(styles.title)}>{properties.event.title}</span>

            {/* A timed event in a column: its start and end */}
            <Show when={properties.layout === "block" ? timed() : undefined}>
                {(current) => (
                    <span {...style.attrs(styles.time)}>
                        {locale.dateRange(current().start, current().end, {
                            hour: "numeric",
                            minute: "2-digit",
                            timeZone: control.timeZone(),
                        })}
                    </span>
                )}
            </Show>
        </button>
    );
}

/** The properties of an event shown as shading, the native element's attributes included. */
export interface EventCalendarBackgroundProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class"
> {
    /** The event. */
    readonly event: CalendarEvent;
    /** The StyleX styles applied after the shading's, such as its place in the view. */
    readonly xstyle?: style.Styles;
}

/** Render an event as shading behind the others: its color's tint under its title. */
export function EventCalendarBackground(
    properties: EventCalendarBackgroundProperties,
): JSX.Element {
    const preset = (): AccentPreset => properties.event.color ?? DEFAULT_COLOR;
    const rest = omit(properties, "event", "xstyle", "style", "children");

    return (
        <div
            {...rest}
            data-slot="event-calendar-background"
            data-color={preset()}
            {...style.attributes(
                [
                    text.xs,
                    styles.background,
                    fills.shade(swatch[`${preset()}Tint`], swatch[`${preset()}TintText`]),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            <span {...style.attrs(styles.title)}>{properties.event.title}</span>
        </div>
    );
}
