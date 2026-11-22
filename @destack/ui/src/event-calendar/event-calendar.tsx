import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import { nth } from "@destack/schema";
import * as style from "@destack/style";
import { space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, Match, omit, Switch, useEnvironment, useLocale, useNow } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { DragAnnouncer } from "../drag/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { EventCalendarAgenda } from "./agenda.tsx";
import { EventCalendarMonth } from "./month.tsx";
import {
    EVENT_CALENDAR_VIEWS,
    EventCalendarContext,
    type EventCalendarProperties,
    EventCalendarControl,
    useEventCalendar,
    EventCalendarEventContext,
    type EventCalendarEventRender,
} from "./control.ts";
import { EventCalendarTime } from "./time.tsx";

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
        alignItems: "center",
        gap: space[2],
    },
    title: {
        flexGrow: 1,
        margin: 0,
        marginInlineStart: space[3],
        fontWeight: weight.semibold,
    },
});

/** Render a calendar of events in a month, week, day or agenda view, after Google Calendar. */
export function EventCalendar(properties: EventCalendarProperties): JSX.Element {
    // share the view, date and drags with the parts
    const control = new EventCalendarControl(
        properties,
        useLocale(),
        useEnvironment(),
        useNow(NOW_INTERVAL),
    );
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
        "onAction",
        "onCreate",
        "now",
        "weekends",
        "timeZone",
        "dayMaxEvents",
        "slotDuration",
        "openingHour",
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
                {...rest}
                {...style.attributes([styles.calendar, properties.xstyle], properties.style)}
            >
                {properties.children ?? (
                    <>
                        <EventCalendarHeader />
                        <EventCalendarBody />
                    </>
                )}
                <DragAnnouncer manager={control.manager} />
            </div>
        </EventCalendarContext>
    );
}

/** The properties of a part of a calendar's header, the native element's attributes included. */
export interface EventCalendarPartProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class"
> {
    /** The StyleX styles applied after the part's. */
    readonly xstyle?: style.Styles;
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
                    <EventCalendarViews />
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
    // title a month by its name, a day in full, and a week by its first and last shown days
    const control = useEventCalendar();
    const locale = useLocale();
    const title = (): string => {
        // read the shown date and view
        const date = control.date();
        const view = control.view();
        if (view === "month") {
            return locale.plainDate(date, { month: "long", year: "numeric" });
        } else if (view === "day") {
            return locale.plainDate(date, { dateStyle: "full" });
        }
        const days = control.days();

        return locale.plainDateRange(nth(days, 0), nth(days, -1), {
            month: "short",
            day: "numeric",
            year: "numeric",
        });
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

/** Render the choice of view: month, week, day or agenda. */
export function EventCalendarViews(): JSX.Element {
    const control = useEventCalendar();
    const locale = useLocale();

    return (
        <ToggleGroup
            variant="outline"
            data-slot="event-calendar-views"
            behavior="replace"
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
            <ToggleGroupItem value="month">{locale.render(t`Month`)}</ToggleGroupItem>
            <ToggleGroupItem value="week">{locale.render(t`Week`)}</ToggleGroupItem>
            <ToggleGroupItem value="day">{locale.render(t`Day`)}</ToggleGroupItem>
            <ToggleGroupItem value="agenda">{locale.render(t`Agenda`)}</ToggleGroupItem>
        </ToggleGroup>
    );
}

/** Render the view the calendar shows. */
export function EventCalendarBody(properties: {
    /** Render an event, the calendar's default when absent. */
    readonly children?: EventCalendarEventRender;
}): JSX.Element {
    const control = useEventCalendar();

    return (
        <EventCalendarEventContext value={properties.children ?? null}>
            <Switch>
                <Match when={control.view() === "month"}>
                    <EventCalendarMonth />
                </Match>
                <Match when={control.view() === "week" || control.view() === "day"}>
                    <EventCalendarTime />
                </Match>
                <Match when={control.view() === "agenda"}>
                    <EventCalendarAgenda />
                </Match>
            </Switch>
        </EventCalendarEventContext>
    );
}
