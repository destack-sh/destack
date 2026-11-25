import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import { nth } from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    useActions,
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
import { Button, type ButtonProperties } from "../button/index.ts";
import { HoverCard } from "../hover-card/index.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import { mergeProperties } from "../part/index.ts";
import { DragAnnouncer, useDragManager } from "../drag/index.ts";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import {
    EVENT_CALENDAR_VIEWS,
    EventCalendarContext,
    type EventCalendarProperties,
    type EventCalendarView,
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

/** A calendar as narrow as a phone. */
const NARROW_QUERY = "@container event-calendar (max-width: 36rem)";

/** The styles of a calendar and its header. */
const styles = style.create({
    calendar: {
        containerType: "inline-size",
        containerName: "event-calendar",
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        height: DEFAULT_HEIGHT,
        minHeight: 0,
        // cut off a page a finger swipes away
        overflowX: { default: null, [media.pointerCoarse]: "clip" },
        overflowClipMargin: space[1],
    },
    header: {
        display: "flex",
        // move the views below the title where both are too wide
        flexWrap: "wrap",
        alignItems: "center",
        gap: { default: space[2], [NARROW_QUERY]: space[1] },
    },
    views: {
        display: { default: null, [NARROW_QUERY]: "none" },
        flexWrap: "wrap",
        maxWidth: "100%",
    },
    wide: {
        display: { default: null, [NARROW_QUERY]: "none" },
    },
    narrow: {
        display: { default: "none", [NARROW_QUERY]: "inline-flex" },
    },
    tight: {
        paddingInline: { default: space[3], [NARROW_QUERY]: space[2] },
    },
    title: {
        // lead the header of a phone
        order: { default: null, [NARROW_QUERY]: -1 },
        flexGrow: 1,
        whiteSpace: "nowrap",
        margin: 0,
        marginInlineStart: { default: space[3], [NARROW_QUERY]: 0 },
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
    // handle the calendar's actions, running their keys before those of the page
    const actions = useActions();
    const isCommand = useCommandPlatform();
    let root: HTMLElement | undefined;
    const handlers = control.handleActions();
    const keys = {
        ref: (element: HTMLElement) => {
            root = element;
            handlers.forEach((attach) => attach(element));
        },
        onKeyDown: (event: KeyboardEvent) => {
            if (!event.defaultPrevented) {
                actions.press(event, isCommand(), root);
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
                <HoverCard control={control.card}>{properties.children}</HoverCard>
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
                    <EventCalendarViewMenu />
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
            xstyle={[styles.tight, properties.xstyle]}
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

    // title a phone by the months shown, as its days say the rest
    const shortTitle = (): string => {
        if (control.view() === "year") {
            return title();
        }
        const range = control.range();
        const date = control.date();
        const isMonth = control.view() === "month";

        return locale.plainDateRange(isMonth ? date : range.from, isMonth ? date : range.to, {
            month: "short",
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
            {properties.children ?? (
                <>
                    <span {...style.attrs(styles.wide)}>{title()}</span>
                    <span {...style.attrs(styles.narrow)}>{shortTitle()}</span>
                </>
            )}
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

/** Render the menu of the views the calendar's view parts offer, on a phone in their place. */
export function EventCalendarViewMenu(properties: ButtonProperties): JSX.Element {
    // write the view shown
    const control = useEventCalendar();
    const locale = useLocale();
    const title = (view: EventCalendarView): string => locale.render(control.viewTitle(view));

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="outline"
                        {...properties}
                        xstyle={[styles.narrow, styles.tight, properties.xstyle]}
                    />
                )}
                data-slot="event-calendar-view-menu"
            >
                {properties.children ?? title(control.view())}
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
                <DropdownMenuRadioGroup
                    value={control.view()}
                    onValueChange={(view) => {
                        const chosen = EVENT_CALENDAR_VIEWS.find((each) => each === view);
                        if (chosen !== undefined) {
                            control.setView(chosen);
                        }
                    }}
                >
                    <For each={control.views()}>
                        {(view) => (
                            <DropdownMenuRadioItem value={view}>
                                {title(view)}
                            </DropdownMenuRadioItem>
                        )}
                    </For>
                </DropdownMenuRadioGroup>
            </DropdownMenuContent>
        </DropdownMenu>
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
