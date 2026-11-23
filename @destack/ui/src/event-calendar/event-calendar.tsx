import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import { nth } from "@destack/schema";
import * as style from "@destack/style";
import { space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    createEffect,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useEnvironment,
    useLocale,
    useNow,
} from "@destack/view";
import { Action } from "../action/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { HoverCard } from "../hover-card/index.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import { mergeProperties } from "../part/index.ts";
import { DragAnnouncer, useDragManager } from "../drag/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import {
    EVENT_CALENDAR_VIEWS,
    EventCalendarContext,
    type EventCalendarProperties,
    EventCalendarControl,
    useEventCalendar,
    EventCalendarEventContext,
    type EventCalendarEventRender,
    type EventCalendarPartProperties,
} from "./control.ts";

/** The height of a calendar that sets none: about ten hours of a day at the default hour height. */
const DEFAULT_HEIGHT = "40rem";

/** How often the now line moves: each minute, finer than a column's pixels between snaps. */
const NOW_INTERVAL = { minutes: 1 } as const;

/** The styles of a calendar and its header. */
const styles = style.create({
    calendar: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        height: DEFAULT_HEIGHT,
        minHeight: 0,
    },
    header: {
        display: "flex",
        // move the views below the title where both are too wide
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
    views: {
        flexWrap: "wrap",
        maxWidth: "100%",
    },
    title: {
        flexGrow: 1,
        whiteSpace: "nowrap",
        margin: 0,
        marginInlineStart: space[3],
        fontWeight: weight.medium,
    },
});

/** Render a calendar of events in the views its parts offer. */
export function EventCalendar(properties: EventCalendarProperties): JSX.Element {
    // share the view, date and drags with the parts
    const drags = useDragManager();
    const control = new EventCalendarControl(
        properties,
        drags.manager,
        useLocale(),
        useEnvironment(),
        useNow(NOW_INTERVAL),
    );
    // run the calendar's actions by their keys, outside fields and keys a part took
    const isCommand = useCommandPlatform();
    const keys = {
        onKeyDown: (event: KeyboardEvent) => {
            if (!event.defaultPrevented && !Action.isTyping(event)) {
                Action.handle(control.actions(), event, isCommand());
            }
        },
    };
    const rest = omit(
        properties,
        "events",
        "view",
        "defaultView",
        "onViewChange",
        "date",
        "defaultDate",
        "onDateChange",
        "onEventChange",
        "onRepeatChange",
        "onAction",
        "onCreate",
        "mode",
        "availability",
        "defaultAvailability",
        "onAvailabilityChange",
        "onEventAdd",
        "onEventRemove",
        "onRepeatRemove",
        "onReceive",
        "dropTypes",
        "selection",
        "sources",
        "hiddenSources",
        "defaultHiddenSources",
        "onHiddenSourcesChange",
        "resources",
        "now",
        "weekends",
        "workWeek",
        "dayCount",
        "weekNumbers",
        "timeZone",
        "timeZones",
        "dayMaxEvents",
        "slotDuration",
        "snapDuration",
        "scrollTime",
        "hourHeight",
        "children",
        "xstyle",
        "style",
    );

    return (
        <EventCalendarContext value={control}>
            <div
                data-slot="event-calendar"
                data-view={control.view()}
                data-mode={properties.mode ?? "events"}
                {...mergeProperties(keys, rest)}
                {...style.attributes([styles.calendar, properties.xstyle], properties.style)}
            >
                <HoverCard>{properties.children}</HoverCard>
                <Show when={!drags.isShared}>
                    <DragAnnouncer manager={control.manager} />
                </Show>
            </div>
        </EventCalendarContext>
    );
}

/** Render the header: previous, today, next, the title and the views by default. */
export function EventCalendarHeader(properties: EventCalendarPartProperties): JSX.Element {
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <div
            data-slot="event-calendar-header"
            {...rest}
            {...style.attributes([styles.header, properties.xstyle], properties.style)}
        >
            {properties.children ?? (
                <>
                    <EventCalendarPrevious />
                    <EventCalendarToday />
                    <EventCalendarNext />
                    <EventCalendarTitle />
                    <EventCalendarViewSwitcher />
                </>
            )}
        </div>
    );
}

/** Render the button that moves a view back. */
export function EventCalendarPrevious(properties: Omit<ButtonProperties, "onClick">): JSX.Element {
    const control = useEventCalendar();
    const locale = useLocale();

    return (
        <Button
            variant="outline"
            size="icon"
            data-slot="event-calendar-previous"
            aria-label={locale.render(t`Previous`)}
            {...properties}
            onClick={() => control.step(-1)}
        >
            <Icon icon={locale.direction === "rtl" ? caretRight : caretLeft} />
        </Button>
    );
}

/** Render the button that moves to today. */
export function EventCalendarToday(properties: Omit<ButtonProperties, "onClick">): JSX.Element {
    const control = useEventCalendar();
    const locale = useLocale();

    return (
        <Button
            variant="outline"
            data-slot="event-calendar-today"
            {...properties}
            onClick={() => control.showToday()}
        >
            {properties.children ?? locale.render(t`Today`)}
        </Button>
    );
}

/** Render the button that moves a view forward. */
export function EventCalendarNext(properties: Omit<ButtonProperties, "onClick">): JSX.Element {
    const control = useEventCalendar();
    const locale = useLocale();

    return (
        <Button
            variant="outline"
            size="icon"
            data-slot="event-calendar-next"
            aria-label={locale.render(t`Next`)}
            {...properties}
            onClick={() => control.step(1)}
        >
            <Icon icon={locale.direction === "rtl" ? caretLeft : caretRight} />
        </Button>
    );
}

/** Render the shown range as a title in the reader's locale. */
export function EventCalendarTitle(properties: EventCalendarPartProperties): JSX.Element {
    // title the shown range by the view
    const control = useEventCalendar();
    const locale = useLocale();
    const title = (): string => {
        // title a year by its number, a month by its name and a day in full
        const date = control.date();
        const view = control.view();
        if (view === "year") {
            return locale.plainDate(date, { year: "numeric" });
        } else if (view === "month") {
            return locale.plainDate(date, { month: "long", year: "numeric" });
        } else if (view === "day") {
            return locale.plainDate(date, { dateStyle: "full" });
        }
        // title the other views by their first and last shown days
        else {
            const days = control.days();

            return locale.plainDateRange(nth(days, 0), nth(days, -1), {
                month: "short",
                day: "numeric",
                year: "numeric",
            });
        }
    };

    // take the caller's attributes, and their title in place of the range
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <h2
            data-slot="event-calendar-title"
            aria-live="polite"
            {...rest}
            {...style.attributes([text.lg, styles.title, properties.xstyle], properties.style)}
        >
            {properties.children ?? title()}
        </h2>
    );
}

/** Render the choice among the views the calendar's view parts offer. */
export function EventCalendarViewSwitcher(): JSX.Element {
    const control = useEventCalendar();
    const locale = useLocale();

    return (
        <ToggleGroup
            xstyle={styles.views}
            data-slot="event-calendar-view-switcher"
            behavior="replace"
            disallowEmptySelection
            value={control.view()}
            onValueChange={(view) => {
                // take the view the pressed item stands for, which a replacing group always has
                const chosen = EVENT_CALENDAR_VIEWS.find((each) => each === view);
                if (chosen === undefined) {
                    throw new TypeError(
                        `calendar view switched to an unknown view: '${String(view)}'`,
                    );
                }
                control.setView(chosen);
            }}
        >
            <For each={control.views()}>
                {(view) => (
                    <ToggleGroupItem value={view}>
                        {locale.render(control.viewTitle(view))}
                    </ToggleGroupItem>
                )}
            </For>
        </ToggleGroup>
    );
}

/** Render the views the calendar offers, each showing while chosen. */
export function EventCalendarBody(properties: {
    /** The view parts, such as a month, a time grid and an agenda. */
    readonly children?: JSX.Element;
    /** Render an event, the calendar's default when absent. */
    readonly render?: EventCalendarEventRender;
}): JSX.Element {
    // refuse a chosen view that no mounted view part offers, checked as the view changes
    const control = useEventCalendar();
    createEffect(
        () => control.view(),
        (view) => {
            if (!untrack(control.views).includes(view)) {
                throw new TypeError(`calendar view '${view}' has no view part`);
            }
        },
    );

    return (
        <EventCalendarEventContext value={properties.render ?? null}>
            {properties.children}
        </EventCalendarEventContext>
    );
}
