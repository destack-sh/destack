import { swatches } from "@destack/theme/swatch";
import { state } from "@destack/theme/state";
import type { Hue } from "@destack/color";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import type { IconBodies } from "@destack/icon";
import arrowSquareOut from "@destack/icon/phosphor/arrow-square-out";
import calendarBlank from "@destack/icon/phosphor/calendar-blank";
import clock from "@destack/icon/phosphor/clock";
import copy from "@destack/icon/phosphor/copy";
import trash from "@destack/icon/phosphor/trash";
import user from "@destack/icon/phosphor/user";
import { Icon } from "@destack/ui/icon";
import { color, radius, size, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    useActions,
    createEffect,
    For,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import { ActionButton, duplicateItems, openItem, removeItems } from "../action/index.ts";
import { HoverCardContent, type HoverCardContentProperties } from "../hover-card/index.ts";
import { OpenChange } from "../layer/index.ts";
import { Hover, Press } from "../interaction/index.ts";
import { mergeProperties, renderPart, type Render } from "../part/index.ts";
import { useEventCalendar } from "./control.ts";
import { CalendarEvent, type TimedCalendarEvent } from "./event.ts";

/** The width of a list row's time, room for a range such as "10:30 – 11:45 AM". */
const LIST_TIME_WIDTH = `calc(36 * ${space[1]})`;

/** The heights of a placed event too short for a second line: two lines of extra-small text. */
const SHORT_QUERY = "@container (max-height: 2.5rem)";

/** A calendar as narrow as a phone. */
const NARROW_QUERY = "@container event-calendar (max-width: 36rem)";

/** A calendar too narrow for a time beside a title in a month, about six root text sizes a day. */
const COMPACT_QUERY = "@container event-calendar (max-width: 44rem)";

/** A day's column too narrow for times below a title. */
const SLIM_QUERY = "@container (max-width: 6rem)";

/** The actions an event's preview offers, each by its icon. */
const PREVIEW_ACTIONS = [
    { action: openItem, icon: arrowSquareOut },
    { action: duplicateItems, icon: copy },
    { action: removeItems, icon: trash },
];

/** The styles of an event. */
const styles = style.create({
    preview: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        width: "100%",
    },
    head: {
        display: "flex",
        alignItems: "flex-start",
        gap: space[2],
    },
    mark: {
        flexShrink: 0,
        width: space[1],
        minHeight: "1lh",
        alignSelf: "stretch",
        borderRadius: radius.full,
    },
    heading: {
        flex: 1,
        minWidth: 0,
        fontWeight: weight.semibold,
        overflowWrap: "anywhere",
    },
    details: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
    },
    detail: {
        display: "flex",
        alignItems: "flex-start",
        gap: space[2],
        color: color.mutedForeground,
    },
    sign: {
        flexShrink: 0,
        marginBlockStart: "0.15lh",
    },
    actions: {
        display: "flex",
        flexShrink: 0,
        marginBlock: `calc((1lh - ${size[1]}) / 2)`,
        marginInlineEnd: `calc(-1 * ${space[1]})`,
        color: color.mutedForeground,
    },
    busy: {
        backgroundColor: color.muted,
        color: color.mutedForeground,
        borderWidth: stroke.border,
        borderStyle: "dashed",
        borderColor: color.border,
        cursor: "default",
    },
    selected: {
        outlineWidth: stroke.ring,
        outlineStyle: "solid",
        outlineColor: swatch.accentMark,
        outlineOffset: stroke.ring,
    },
    event: {
        borderWidth: { default: null, [media.forcedColors]: stroke.border },
        borderStyle: { default: null, [media.forcedColors]: "solid" },
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
        marginInline: { default: space[1], [NARROW_QUERY]: stroke.border },
        paddingInlineStart: { default: space[2], [NARROW_QUERY]: space[1] },
        paddingInlineEnd: { default: space[2], [NARROW_QUERY]: 0 },
        borderRadius: { default: radius[2], [NARROW_QUERY]: radius[1] },
        whiteSpace: "nowrap",
    },
    block: {
        flexWrap: { default: "wrap", [SHORT_QUERY]: "nowrap" },
        alignContent: "flex-start",
        alignItems: { default: null, [SHORT_QUERY]: "center" },
        rowGap: 0,
        columnGap: space[1],
        paddingBlock: { default: space[1], [SHORT_QUERY]: 0 },
        paddingInlineStart: `calc(${space[2]} - ${stroke.ring})`,
        paddingInlineEnd: space[2],
        borderInlineStartWidth: stroke.ring,
        borderInlineStartStyle: "solid",
    },
    list: {
        display: "grid",
        gridTemplateColumns: {
            default: `${LIST_TIME_WIDTH} ${space[2]} minmax(0, 1fr)`,
            [NARROW_QUERY]: `${space[2]} minmax(0, 1fr)`,
        },
        alignItems: "center",
        columnGap: space[2],
        width: "100%",
        paddingBlock: space[1],
        paddingInline: space[1],
        cursor: "default",
        backgroundColor: "transparent",
    },
    // lay out each title in its own direction, so one in another script is cut at its end
    title: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        fontWeight: weight.medium,
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
        overflow: "hidden",
        color: color.mutedForeground,
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        fontVariantNumeric: "tabular-nums",
    },
    whole: {
        display: { default: null, [COMPACT_QUERY]: "none" },
        flexShrink: 0,
    },
    below: {
        gridColumn: { default: null, [NARROW_QUERY]: "2" },
        gridRow: { default: null, [NARROW_QUERY]: "2" },
    },
    span: {
        display: { default: null, [SLIM_QUERY]: "none" },
    },
    clipped: {
        textOverflow: { default: "ellipsis", [NARROW_QUERY]: "clip" },
        unicodeBidi: "plaintext",
        fontWeight: { default: weight.medium, [NARROW_QUERY]: weight.regular },
    },
    edged: {
        borderInlineStartWidth: { default: null, [NARROW_QUERY]: stroke.ring },
        borderInlineStartStyle: { default: null, [NARROW_QUERY]: "solid" },
    },
    lone: {
        display: { default: null, [NARROW_QUERY]: "none" },
    },
    lead: {
        flexShrink: 0,
        maxWidth: "100%",
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

/** The styles of an event's place in a view while it is dragged elsewhere. */
export const placement = style.create({
    dragging: {
        opacity: 0.5,
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
    const hue = (): Hue => control.colorOf(properties.event);
    const isBusy = (): boolean => control.isBusy(properties.event);
    const timed = (): TimedCalendarEvent | undefined =>
        CalendarEvent.isTimed(properties.event) ? properties.event : undefined;
    const time = (at: number): string =>
        locale.date(at, { hour: "numeric", minute: "2-digit", timeZone: control.timeZone() });
    const isDayLong = (): boolean =>
        timed() !== undefined && CalendarEvent.isAllDay(properties.event);
    const rest = omit(properties, "event", "layout", "isStart", "isEnd", "children");

    // preview the event in the calendar's card while a pointer rests on it or it has the focus
    const { card } = control;
    let element: HTMLElement | undefined;
    onCleanup(() => card.drop(element));
    const openPreview = () => {
        if (element !== undefined && !isBusy()) {
            const trigger = { element, payload: properties.event.key };
            card.openLater(new OpenChange("trigger-hover", undefined, trigger));
        }
    };
    const hover = new Hover({
        onHoverStart: openPreview,
        onHoverEnd: (event) => card.leave(event),
    });
    const preview = mergeProperties(hover.attributes(), {
        ref: (created: HTMLButtonElement) => (element = created),
        onFocus: openPreview,
        onBlur: () => card.closeLater(),
    });

    // open the event on a press or Enter, and on Space in a list, where no drag takes it
    const press = new Press({
        onPress: (gesture) => (isBusy() ? undefined : control.press(properties.event.key, gesture)),
        keys: untrack(() => properties.layout) === "list" ? ["Enter", " "] : ["Enter"],
        shouldCancelOnMove: untrack(() => properties.layout) !== "list",
    });
    const layouts = {
        inline: styles.inline,
        block: styles.block,
        list: [state.hover, styles.list],
    };
    const isTinted = () =>
        properties.layout === "block" ||
        (properties.layout === "inline" && (timed() === undefined || isDayLong()));

    return renderPart(
        "button",
        "event-calendar-event",
        rest,
        () => [
            text.xs,
            state.focus,
            styles.event,
            layouts[properties.layout],
            isTinted() && !isBusy() && fills.tint(swatches[hue()].tint, swatches[hue()].tintText),
            properties.layout === "block" && !isBusy() && fills.edge(swatches[hue()].mark),
            properties.layout === "inline" &&
                !isBusy() && [styles.edged, fills.edge(swatches[hue()].mark)],
            isBusy() && styles.busy,
            control.selection.isSelected(properties.event.key) && styles.selected,
            !properties.isEnd && styles.continues,
            !properties.isStart && styles.continued,
        ],
        mergeProperties(
            {
                type: "button" as const,
                get "data-event"() {
                    return properties.event.key;
                },
                get "data-color"() {
                    return hue();
                },
                get "data-busy"() {
                    return isBusy() ? "" : undefined;
                },
                get "data-selected"() {
                    return control.selection.isSelected(properties.event.key) ? "" : undefined;
                },
                get "aria-pressed"() {
                    return control.selection.isSelected(properties.event.key) ? "true" : "false";
                },
                get "data-layout"() {
                    return properties.layout;
                },
                get children() {
                    return (
                        <>
                            {/* An event in a list: its time or all day, then its color's dot */}
                            <Show when={properties.layout === "list"}>
                                <span {...style.attrs(styles.time, styles.below)}>
                                    <Show
                                        when={timed()}
                                        fallback={locale.render(t.context("option")`All day`)}
                                    >
                                        {(current) => (
                                            <>
                                                {locale.dateRange(current().start, current().end, {
                                                    hour: "numeric",
                                                    minute: "2-digit",
                                                    timeZone: control.timeZone(),
                                                })}
                                            </>
                                        )}
                                    </Show>
                                </span>
                                <span
                                    {...style.attrs(styles.dot, fills.dot(swatches[hue()].mark))}
                                />
                            </Show>

                            {/* A timed event on a row: its color's dot and its start */}
                            <Show when={properties.layout === "inline" ? timed() : undefined}>
                                {(current) => (
                                    <>
                                        <Show when={!isDayLong()}>
                                            <span
                                                {...style.attrs(
                                                    styles.dot,
                                                    styles.lone,
                                                    fills.dot(swatches[hue()].mark),
                                                )}
                                            />
                                        </Show>
                                        <span {...style.attrs(styles.time, styles.whole)}>
                                            {time(current().start)}
                                        </span>
                                    </>
                                )}
                            </Show>

                            {/* Title */}
                            <span
                                {...style.attrs(
                                    styles.title,
                                    properties.layout === "block" && styles.lead,
                                    properties.layout === "inline" && styles.clipped,
                                )}
                            >
                                {isBusy()
                                    ? locale.render(t.context("option")`Busy`)
                                    : properties.event.title}
                            </span>

                            {/* A timed event in a column: its start and end */}
                            <Show when={properties.layout === "block" ? timed() : undefined}>
                                {(current) => (
                                    <span {...style.attrs(styles.time, styles.span)}>
                                        {locale.dateRange(current().start, current().end, {
                                            hour: "numeric",
                                            minute: "2-digit",
                                            timeZone: control.timeZone(),
                                        })}
                                    </span>
                                )}
                            </Show>
                        </>
                    );
                },
            },
            mergeProperties(press.attributes(), preview),
        ),
    );
}

/** Render the card that previews the event a pointer rests on or the focus is on. */
export function EventCalendarPreviewCard(
    properties: Omit<HoverCardContentProperties, "children">,
): JSX.Element {
    // show the previewed event while the calendar's card is open, closing it as a drag starts
    const control = useEventCalendar();
    const { card } = control;
    createEffect(
        () => control.manager.operation() !== undefined,
        (isDragging) => (isDragging ? card.close() : undefined),
    );
    const event = () => {
        const key = card.state.payload();

        return card.isOpen() && key !== undefined && control.hasEvent(key)
            ? control.eventOf(key)
            : undefined;
    };

    return (
        <Show when={event()}>
            {(current) => (
                <HoverCardContent side="right" align="start" {...properties}>
                    <EventCalendarPreview event={current()} />
                </HoverCardContent>
            )}
        </Show>
    );
}

/** Render an event's preview: its title, times, calendar, resource and actions. */
export function EventCalendarPreview(properties: {
    /** The event. */
    readonly event: CalendarEvent;
    /** The StyleX styles applied after the preview's styles. */
    readonly xstyle?: style.Styles;
}): JSX.Element {
    // read the event's times, calendar and resource, and where its actions run
    const control = useEventCalendar();
    const locale = useLocale();
    const actions = useActions();
    const chip = () => control.elementOf(properties.event.key);
    const source = () => control.sourceOf(properties.event);
    const resource = () => control.resourceOf(properties.event);
    const times = () => {
        const event = properties.event;

        return CalendarEvent.isTimed(event)
            ? locale.dateRange(event.start, event.end, {
                  weekday: "short",
                  month: "short",
                  day: "numeric",
                  hour: "numeric",
                  minute: "2-digit",
                  timeZone: control.timeZone(),
              })
            : locale.plainDateRange(event.start, event.end, {
                  weekday: "short",
                  month: "short",
                  day: "numeric",
              });
    };

    return (
        <div data-slot="event-calendar-preview" {...style.attrs(styles.preview, properties.xstyle)}>
            {/* The title by its color, with what can be done */}
            <div {...style.attrs(text.sm, styles.head)}>
                <span
                    aria-hidden="true"
                    {...style.attrs(
                        styles.mark,
                        fills.dot(swatches[control.colorOf(properties.event)].mark),
                    )}
                />
                <strong data-slot="event-calendar-preview-title" {...style.attrs(styles.heading)}>
                    {properties.event.title}
                </strong>
                <div {...style.attrs(styles.actions)}>
                    <For each={PREVIEW_ACTIONS}>
                        {({ action, icon }) => (
                            <Show when={actions.has(action.reference, chip())}>
                                <ActionButton
                                    action={action}
                                    element={chip()}
                                    icon={icon}
                                    isIconOnly
                                    size="sm"
                                    variant="ghost"
                                />
                            </Show>
                        )}
                    </For>
                </div>
            </div>

            {/* When, in which calendar and for whom */}
            <div {...style.attrs(text.sm, styles.details)}>
                <EventCalendarDetail icon={clock}>{times()}</EventCalendarDetail>
                <Show when={source()}>
                    {(current) => (
                        <EventCalendarDetail icon={calendarBlank}>
                            {current().label}
                        </EventCalendarDetail>
                    )}
                </Show>
                <Show when={resource()}>
                    {(current) => (
                        <EventCalendarDetail icon={user}>{current().label}</EventCalendarDetail>
                    )}
                </Show>
            </div>
        </div>
    );
}

/** Render a line of an event's preview behind its icon. */
function EventCalendarDetail(properties: {
    /** The icon. */
    readonly icon: IconBodies;
    /** The line. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <span data-slot="event-calendar-preview-detail" {...style.attrs(styles.detail)}>
            <Icon icon={properties.icon} xstyle={styles.sign} />
            {properties.children}
        </span>
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
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Render an event as shading behind the others: its color's tint under its title. */
export function EventCalendarBackground(
    properties: EventCalendarBackgroundProperties,
): JSX.Element {
    // take the event's color, else its calendar's
    const control = useEventCalendar();
    const hue = (): Hue => control.colorOf(properties.event);
    const rest = omit(properties, "event", "children");

    return renderPart(
        "div",
        "event-calendar-background",
        rest,
        () => [
            text.xs,
            styles.background,
            fills.shade(swatches[hue()].tint, swatches[hue()].tintText),
        ],
        {
            get "data-color"() {
                return hue();
            },
            get children() {
                return <span {...style.attrs(styles.title)}>{properties.event.title}</span>;
            },
        },
    );
}
