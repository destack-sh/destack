import { Locale, Localization } from "@destack/locale";
import { TimeZone } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, Errored, flush, For, LocaleContext } from "@destack/view";
import { render, stubPopovers, testWorld } from "@destack/view/test";
import { Draggable, DragProvider, useDragManager } from "../drag/index.ts";
import {
    type CalendarDrop,
    CalendarEvent,
    CalendarRepeatChange,
    CalendarRepeatRemoval,
    type CalendarEventDraft,
    EventCalendar,
    EventCalendarAgenda,
    EventCalendarBody,
    EventCalendarColumn,
    EventCalendarColumnContent,
    EventCalendarDayColumnList,
    EventCalendarDate,
    EventCalendarDayContent,
    EventCalendarDayHeader,
    EventCalendarHeader,
    EventCalendarHourGrid,
    EventCalendarTimeGrid,
    EventCalendarMonth,
    EventCalendarMonthDay,
    EventCalendarNext,
    type EventCalendarOptions,
    EventCalendarTime,
    EventCalendarTitle,
    type EventCalendarView,
    EventCalendarWeek,
    EventCalendarPreviewCard,
    EventCalendarSourceList,
    EventCalendarDayGrid,
    EventCalendarYear,
    useEventCalendarDay,
} from "./index.ts";

/** The time zone the test calendar runs in, which leaves summer time on 25 October 2026. */
const ZONE = "Europe/Vienna";

/** The marks that isolate interpolated names in rendered messages. */
const ISOLATES = /[⁦-⁩]/gu;

/** The en dash between a range's ends, which ICU versions space differently. */
const RANGE_DASH = /\s*–\s*/gu;

/** Read rendered text without isolating marks, its ranges' dashes spaced one way. */
function plain(text: string | null | undefined): string {
    return (text ?? "").replaceAll(ISOLATES, "").replaceAll(RANGE_DASH, " – ");
}

/** Read the instant of a wall time in Vienna. */
function vienna(date: string, time: string): number {
    return TimeZone.instant(ZONE, date, time);
}

/** Render a calendar in a locale on a date, its changes collected. */
function drawCalendar(
    tag: string,
    date: string,
    events: readonly CalendarEvent[],
    view: EventCalendarView = "month",
    options: Partial<EventCalendarOptions> = {},
): {
    readonly container: HTMLElement;
    readonly changes: CalendarEvent[];
    readonly created: { readonly start: number; readonly end: number }[];
} {
    const changes: CalendarEvent[] = [];
    const created: { readonly start: number; readonly end: number }[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <EventCalendar
                timeZone={ZONE}
                {...options}
                events={events}
                defaultDate={date}
                defaultView={view}
                onEventChange={(change) => changes.push(change)}
                onCreate={(span) => created.push(span)}
            >
                <EventCalendarHeader />
                <EventCalendarBody>
                    <EventCalendarMonth />
                    <EventCalendarTime />
                    <EventCalendarYear />
                    <EventCalendarAgenda />
                </EventCalendarBody>
            </EventCalendar>
        </LocaleContext>
    ));

    return { container, changes, created };
}

/** Read each week row's events as their titles and first and last days. */
function weeksOf(container: HTMLElement): string[][] {
    return [...container.querySelectorAll("[data-slot=event-calendar-week]")]
        .map((week) =>
            [
                ...week.querySelectorAll<HTMLElement>(
                    "[data-slot=event-calendar-event], [data-slot=event-calendar-more]",
                ),
            ].map((element) =>
                element.dataset["slot"] === "event-calendar-more"
                    ? (element.textContent ?? "")
                    : `${plain(element.textContent)} ${element.dataset["start"] ?? ""}..${element.dataset["end"] ?? ""}`,
            ),
        )
        .filter((week) => week.length > 0);
}

/** Read the titles of the background events. */
function shadesOf(container: HTMLElement): (string | null)[] {
    return [...container.querySelectorAll("[data-slot=event-calendar-background]")].map(
        (shade) => shade.textContent,
    );
}

/** Read the text of the events, without the marks that isolate names. */
function eventsOf(container: HTMLElement): string[] {
    return [...container.querySelectorAll("[data-slot=event-calendar-event]")].map((event) =>
        plain(event.textContent),
    );
}

/** Press and release a key on the focused element. */
function press(container: HTMLElement, key: string, options: KeyboardEventInit = {}): void {
    for (const type of ["keydown", "keyup"]) {
        (container.ownerDocument.activeElement ?? container).dispatchEvent(
            new KeyboardEvent(type, { key, bubbles: true, cancelable: true, ...options }),
        );
    }
    flush();
}

test("lay out November 2026 from Monday in Austria, cutting a conference at the week's end and folding a busy day into more", () => {
    // a conference over a weekend, three more events on its first day, and a timed standup
    const { container } = drawCalendar("de-AT", "2026-11-19", [
        {
            key: "conference",
            title: "Conference",
            start: "2026-11-19",
            end: "2026-11-24",
        },
        {
            key: "launch",
            title: "Launch",
            start: "2026-11-19",
            end: "2026-11-19",
        },
        {
            key: "review",
            title: "Review",
            start: "2026-11-19",
            end: "2026-11-19",
        },
        {
            key: "retro",
            title: "Retro",
            start: "2026-11-19",
            end: "2026-11-19",
        },
        {
            key: "standup",
            title: "Standup",
            start: vienna("2026-11-23", "09:00"),
            end: vienna("2026-11-23", "09:15"),
        },
    ]);
    const firstDays = [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-day]")]
        .filter((_, index) => index % 7 === 0)
        .map((day) => day.dataset["date"]);

    // six weeks from Monday 26 October, the conference cut at Sunday, one event folded into more
    expect({ firstDays, weeks: weeksOf(container) }).toEqual({
        firstDays: [
            "2026-10-26",
            "2026-11-02",
            "2026-11-09",
            "2026-11-16",
            "2026-11-23",
            "2026-11-30",
        ],
        weeks: [
            [
                "Conference 2026-11-19..2026-11-22",
                "Launch 2026-11-19..2026-11-19",
                "Review 2026-11-19..2026-11-19",
                "+1 more",
            ],
            ["Conference 2026-11-23..2026-11-24", "9:00Standup 2026-11-23..2026-11-23"],
        ],
    });
});

test("move the focused day with the arrows and a month with Page Down, the title following", () => {
    // focus the 19th, step right and a month on
    const { container } = drawCalendar("en-US", "2026-11-19", []);
    container.querySelector<HTMLElement>('[data-slot=event-calendar-date][tabindex="0"]')?.focus();
    const visits: string[] = [];
    for (const key of ["ArrowRight", "ArrowDown", "PageDown"]) {
        press(container, key);
        visits.push(container.ownerDocument.activeElement?.getAttribute("aria-label") ?? "");
    }

    expect({ visits, title: container.querySelector("h2")?.textContent }).toEqual({
        visits: [
            "Friday, November 20, 2026",
            "Friday, November 27, 2026",
            "Sunday, December 27, 2026",
        ],
        title: "December 2026",
    });
});

test("move a timed event two days across the end of summer time, at its local start and end", () => {
    // lift the 09:00 meeting on Saturday 24 October with Space, step two days on and drop it
    const { container, changes } = drawCalendar("en-GB", "2026-10-24", [
        {
            key: "planning",
            title: "Planning",
            start: vienna("2026-10-24", "09:00"),
            end: vienna("2026-10-24", "10:30"),
        },
    ]);
    container.querySelector<HTMLElement>("[data-slot=event-calendar-event]")?.focus();
    for (const key of [" ", "ArrowRight", "ArrowRight", " "]) {
        press(container, key);
    }

    // monday's 09:00 is an hour later in UTC than Saturday's
    const [change] = changes;
    expect(
        change === undefined || typeof change.start !== "number" || typeof change.end !== "number"
            ? change
            : [
                  TimeZone.date(ZONE, change.start),
                  TimeZone.time(ZONE, change.start),
                  TimeZone.time(ZONE, change.end),
                  change.start - vienna("2026-10-24", "09:00"),
              ],
    ).toEqual(["2026-10-26", "09:00", "10:30", 2 * 86_400_000 + 3_600_000]);
});

/** Write a timed change as its local day, start and end in Vienna. */
function wall(event: CalendarEvent | undefined): string {
    return event === undefined || typeof event.start !== "number" || typeof event.end !== "number"
        ? "none"
        : `${TimeZone.date(ZONE, event.start)} ${TimeZone.time(ZONE, event.start)}-${TimeZone.time(ZONE, event.end)}`;
}

test("show a week in columns, whole days and day-long times on top, and move and resize timed events by keys", () => {
    // a two-day offsite on top, and a planning meeting below
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "offsite",
                title: "Offsite",
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-19", "17:00"),
            },
            {
                key: "planning",
                title: "Planning",
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-18", "10:00"),
            },
        ],
        "week",
    );
    const columns = [
        ...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-column]"),
    ].map((column) => column.dataset["date"]);
    const allDay = [
        ...container.querySelectorAll(
            "[data-slot=event-calendar-all-day] [data-slot=event-calendar-event]",
        ),
    ].map((event) => event.textContent);

    // move the meeting a day on and a slot later, then stretch its end by two slots
    container
        .querySelector<HTMLElement>(
            "[data-slot=event-calendar-timed] [data-slot=event-calendar-event]",
        )
        ?.focus();
    for (const key of [" ", "ArrowRight", "ArrowDown", " "]) {
        press(container, key);
    }
    container.querySelector<HTMLElement>("[data-slot=event-calendar-resize-end]")?.focus();
    for (const key of [" ", "ArrowDown", "ArrowDown", " "]) {
        press(container, key);
    }

    expect({ columns, allDay, changes: changes.map(wall) }).toEqual({
        columns: [
            "2026-11-16",
            "2026-11-17",
            "2026-11-18",
            "2026-11-19",
            "2026-11-20",
            "2026-11-21",
            "2026-11-22",
        ],
        allDay: ["9:00Offsite"],
        changes: ["2026-11-19 09:15-10:15", "2026-11-18 09:00-10:30"],
    });
});

test("move an overnight event by its part after midnight, the part with its end alone stretching it", () => {
    // a release from 22:00 to 02:00 across Wednesday night
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "release",
                title: "Release",
                start: vienna("2026-11-18", "22:00"),
                end: vienna("2026-11-19", "02:00"),
            },
        ],
        "week",
    );
    const parts = [
        ...container.querySelectorAll<HTMLElement>(
            "[data-slot=event-calendar-timed] [data-slot=event-calendar-event]",
        ),
    ];
    const handles = container.querySelectorAll("[data-slot=event-calendar-resize-end]").length;

    // lift the Thursday part and move it a slot later
    parts[1]?.focus();
    for (const key of [" ", "ArrowDown", " "]) {
        press(container, key);
    }

    expect({ parts: parts.length, handles, changes: changes.map(wall) }).toEqual({
        parts: 2,
        handles: 1,
        changes: ["2026-11-18 22:15-02:15"],
    });
});

/** Count the hour labels between the midnights of a day in London. */
function countHourLabels(date: string): number {
    const { container } = drawCalendar("en-GB", date, [], "day");

    return container.querySelectorAll("[data-slot=event-calendar-time-grid] [aria-hidden] span")
        .length;
}

test("label each wall hour once on the day summer time ends, its repeated hour at one place", () => {
    // the day summer time ends repeats two o'clock, which the wall clock shows once
    expect([countHourLabels("2026-10-25"), countHourLabels("2026-10-26")]).toEqual([23, 23]);
});

test("place a day's events on the wall clock, the day summer time ends lined up with the others", () => {
    // a noon meeting on the Saturday and on the 25-hour Sunday after it
    const { container } = drawCalendar(
        "en-GB",
        "2026-10-25",
        [
            {
                key: "sunday",
                title: "Sunday",
                start: vienna("2026-10-25", "12:00"),
                end: vienna("2026-10-25", "13:00"),
            },
            {
                key: "saturday",
                title: "Saturday",
                start: vienna("2026-10-24", "12:00"),
                end: vienna("2026-10-24", "13:00"),
            },
        ],
        "week",
    );
    const tops = [
        ...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-timed]"),
    ].map((event) => event.style.getPropertyValue("--x-top"));

    expect(tops).toEqual(["50%", "50%"]);
});

test("drag a timed event by its middle an hour down, its landing showing it at the new time", () => {
    // lay the day out at forty-eight pixels an hour, a planning meeting from 9:00 to 10:00
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "planning",
                title: "Planning",
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-18", "10:00"),
            },
        ],
        "day",
    );
    const column = container.querySelector<HTMLElement>("[data-slot=event-calendar-column]");
    const event = container.querySelector<HTMLElement>(
        "[data-slot=event-calendar-timed] [data-slot=event-calendar-event]",
    );
    if (column === null || event === null) {
        throw new TypeError("the day column or its event did not render");
    }
    column.getBoundingClientRect = () => new DOMRect(0, 0, 400, 24 * 48);
    const pointer = (type: string, y: number) => {
        event.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: 50,
                clientY: y,
                pointerId: 4,
                width: 1,
                height: 1,
            }),
        );
        flush();
    };

    // press at 9:30, step down in two moves to 10:30, and read the landing before letting go
    pointer("pointerdown", 9.5 * 48);
    pointer("pointermove", 9.5 * 48 + 12);
    pointer("pointermove", 10.5 * 48);
    const landing = plain(
        container.querySelector("[data-slot=event-calendar-landing]")?.textContent,
    );
    pointer("pointerup", 10.5 * 48);

    expect({ changes: changes.map(wall), landing }).toEqual({
        changes: ["2026-11-18 10:00-11:00"],
        landing: "Planning10:00 – 11:00",
    });
});

test("move a timed event's start alone by its top handle, a quarter hour per arrow key", () => {
    // a planning meeting from 9:00 to 10:00, its start lifted and stepped two snaps earlier
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "planning",
                title: "Planning",
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-18", "10:00"),
            },
        ],
        "day",
    );
    container.querySelector<HTMLElement>("[data-slot=event-calendar-resize-start]")?.focus();
    for (const key of [" ", "ArrowUp", "ArrowUp", " "]) {
        press(container, key);
    }

    expect(changes.map(wall)).toEqual(["2026-11-18 08:30-10:00"]);
});

test("sweep out a new event over empty time, snapped to quarter hours", () => {
    // lay the day out at forty-eight pixels an hour and sweep from about 9:05 to 10:20
    const { container, created } = drawCalendar("en-GB", "2026-11-18", [], "day");
    const column = container.querySelector<HTMLElement>("[data-slot=event-calendar-column]");
    if (column === null) {
        throw new TypeError("the day column did not render");
    }
    column.getBoundingClientRect = () => new DOMRect(0, 0, 400, 24 * 48);
    const pointer = (type: string, y: number) => {
        column.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: 50,
                clientY: y,
                pointerId: 3,
                width: 1,
                height: 1,
            }),
        );
        flush();
    };
    pointer("pointerdown", 9 * 48 + 4);
    pointer("pointermove", 10 * 48 + 16);
    pointer("pointerup", 10 * 48 + 16);

    expect(
        created.map(
            (span) => `${TimeZone.time(ZONE, span.start)}-${TimeZone.time(ZONE, span.end)}`,
        ),
    ).toEqual(["09:00-10:15"]);
});

test("list the week's days that have events, whole days first, then by start", () => {
    const { container } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "review",
                title: "Review",
                start: vienna("2026-11-18", "15:00"),
                end: vienna("2026-11-18", "16:00"),
            },
            {
                key: "launch",
                title: "Launch",
                start: "2026-11-18",
                end: "2026-11-18",
            },
            {
                key: "standup",
                title: "Standup",
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-18", "09:15"),
            },
            {
                key: "retro",
                title: "Retro",
                start: vienna("2026-11-20", "16:00"),
                end: vienna("2026-11-20", "17:00"),
            },
        ],
        "agenda",
    );
    const days = [...container.querySelectorAll("[data-slot=event-calendar-agenda] > li")].map(
        (day) => [
            day.querySelector("h3")?.textContent,
            ...[...day.querySelectorAll("[data-slot=event-calendar-event]")].map((event) =>
                plain(event.textContent).replaceAll(/\s/gu, " "),
            ),
        ],
    );

    expect(days).toEqual([
        ["Wed 18 Nov", "All dayLaunch", "09:00 – 09:15Standup", "15:00 – 16:00Review"],
        ["Fri 20 Nov", "16:00 – 17:00Retro"],
    ]);
});

test("compose the header from its parts, render events with the caller's part, and snap to its slot", () => {
    // a week with a title and a next button alone, events as plain buttons, and half-hour slots
    const changes: CalendarEvent[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <EventCalendar
                events={[
                    {
                        key: "planning",
                        title: "Planning",
                        start: vienna("2026-11-18", "09:00"),
                        end: vienna("2026-11-18", "10:00"),
                    },
                ]}
                defaultDate="2026-11-18"
                defaultView="week"
                timeZone={ZONE}
                snapDuration={30}
                onEventChange={(change) => changes.push(change)}
            >
                <EventCalendarHeader>
                    <EventCalendarTitle />
                    <EventCalendarNext />
                </EventCalendarHeader>
                <EventCalendarBody
                    render={(event) => (
                        <button type="button" data-slot="own-event" ref={event.ref}>
                            own {event.event.title}
                        </button>
                    )}
                >
                    <EventCalendarTime />
                </EventCalendarBody>
            </EventCalendar>
        </LocaleContext>
    ));
    const header = [
        ...(container.querySelector("[data-slot=event-calendar-header]")?.children ?? []),
    ];
    const own = container.querySelector("[data-slot=own-event]")?.textContent;

    // step the meeting's end a slot later
    container.querySelector<HTMLElement>("[data-slot=event-calendar-resize-end]")?.focus();
    for (const key of [" ", "ArrowDown", " "]) {
        press(container, key);
    }
    container.querySelector<HTMLElement>("[data-slot=event-calendar-next]")?.click();
    flush();

    expect({
        header: header.map((part) => part.getAttribute("data-slot")),
        own,
        changes: changes.map(wall),
        title: container.querySelector("[data-slot=event-calendar-title]")?.textContent,
    }).toEqual({
        header: ["event-calendar-title", "event-calendar-next"],
        own: "own Planning",
        changes: ["2026-11-18 09:00-10:30"],
        title: "23 – 29 Nov 2026",
    });
});

/** Read the text of each element of a slot, as plain text. */
function slots(container: HTMLElement, slot: string): string[] {
    return [...container.querySelectorAll(`[data-slot=${slot}]`)].map((each) =>
        plain(each.textContent),
    );
}

/** Render a day's own heading from the day around it. */
function OwnHeading() {
    const { day } = useEventCalendarDay();

    return <span data-slot="own-heading">{day}</span>;
}

test("compose the week and the month from their parts, each replaced or left out by the caller", () => {
    // a week of own headings and columns without the now line, and a month of own dates
    const events = [
        {
            key: "planning",
            title: "Planning",
            start: vienna("2026-11-18", "09:00"),
            end: vienna("2026-11-18", "10:00"),
        },
    ];
    const week = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <EventCalendar
                events={events}
                defaultDate="2026-11-18"
                defaultView="day"
                timeZone={ZONE}
            >
                <EventCalendarTime>
                    <EventCalendarDayHeader>{() => <OwnHeading />}</EventCalendarDayHeader>
                    <EventCalendarTimeGrid>
                        <EventCalendarDayColumnList>
                            {() => (
                                <EventCalendarColumn>
                                    <EventCalendarHourGrid />
                                    <EventCalendarColumnContent />
                                </EventCalendarColumn>
                            )}
                        </EventCalendarDayColumnList>
                    </EventCalendarTimeGrid>
                </EventCalendarTime>
            </EventCalendar>
        </LocaleContext>
    ));
    const month = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <EventCalendar
                events={events}
                defaultDate="2026-11-18"
                defaultView="month"
                timeZone={ZONE}
            >
                <EventCalendarMonth>
                    <EventCalendarDayGrid>
                        {() => (
                            <EventCalendarWeek>
                                {() => (
                                    <EventCalendarMonthDay>
                                        <EventCalendarDate>·</EventCalendarDate>
                                        <EventCalendarDayContent />
                                    </EventCalendarMonthDay>
                                )}
                            </EventCalendarWeek>
                        )}
                    </EventCalendarDayGrid>
                </EventCalendarMonth>
            </EventCalendar>
        </LocaleContext>
    ));

    expect({
        headings: slots(week.container, "own-heading"),
        columns: slots(week.container, "event-calendar-column").length,
        events: slots(week.container, "event-calendar-event"),
        nowLines: slots(week.container, "event-calendar-now").length,
        weekdays: slots(month.container, "event-calendar-weekday-header").length,
        dates: new Set(slots(month.container, "event-calendar-date")),
        monthEvents: slots(month.container, "event-calendar-event").length,
    }).toEqual({
        headings: ["2026-11-18"],
        columns: 1,
        events: ["Planning09:00 – 10:00"],
        nowLines: 0,
        weekdays: 0,
        dates: new Set(["·"]),
        monthEvents: 1,
    });
});

test("move an event from the part of it in a later week, each part lifting on its own", () => {
    // an event from Saturday to Tuesday, cut at the week's end, lifted by its Monday part
    const { container, changes } = drawCalendar("de-AT", "2026-11-18", [
        { key: "trip", title: "Trip", start: "2026-11-14", end: "2026-11-17" },
    ]);
    const parts = [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-event]")];
    parts.find((part) => part.dataset["start"] === "2026-11-16")?.focus();
    for (const key of [" ", "ArrowRight", " "]) {
        press(container, key);
    }

    expect({ parts: parts.map((part) => part.dataset["start"]), changes }).toEqual({
        parts: ["2026-11-14", "2026-11-16"],
        changes: [{ key: "trip", title: "Trip", start: "2026-11-15", end: "2026-11-18" }],
    });
});

test("hide the weekend from the week and the month, the arrows and Home and End stepping over it", () => {
    // a trip from Friday to Monday over a weekend the calendar hides
    const trip: CalendarEvent = {
        key: "trip",
        title: "Trip",
        start: "2026-11-20",
        end: "2026-11-23",
    };
    const month = drawCalendar("en-US", "2026-11-20", [trip], "month", { weekends: false });
    const week = drawCalendar("en-US", "2026-11-20", [trip], "week", { weekends: false });
    const day = drawCalendar("en-US", "2026-11-20", [trip], "day", { weekends: false });
    day.container.querySelector<HTMLElement>("[data-slot=event-calendar-next]")?.click();
    flush();
    month.container
        .querySelector<HTMLElement>('[data-slot=event-calendar-date][tabindex="0"]')
        ?.focus();
    const visits: string[] = [];
    for (const key of ["ArrowRight", "End", "Home", "ArrowLeft"]) {
        press(month.container, key);
        visits.push(month.container.ownerDocument.activeElement?.getAttribute("aria-label") ?? "");
    }

    // the month shows five weekdays and cuts the trip at the hidden weekend, the week five days
    expect({
        weekdays: [...month.container.querySelectorAll("abbr")].map((name) => name.textContent),
        weeks: weeksOf(month.container),
        columns: [
            ...week.container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-column]"),
        ].map((column) => column.dataset["date"]),
        title: week.container.querySelector("h2")?.textContent,
        nextDay: day.container.querySelector("h2")?.textContent,
        visits,
    }).toEqual({
        weekdays: ["Mon", "Tue", "Wed", "Thu", "Fri"],
        weeks: [["Trip 2026-11-20..2026-11-20"], ["Trip 2026-11-23..2026-11-23"]],
        columns: ["2026-11-16", "2026-11-17", "2026-11-18", "2026-11-19", "2026-11-20"],
        title: "Nov 16 – 20, 2026",
        nextDay: "Monday, November 23, 2026",
        visits: [
            "Monday, November 23, 2026",
            "Friday, November 27, 2026",
            "Monday, November 23, 2026",
            "Friday, November 20, 2026",
        ],
    });
});

test("shade background events behind the others, outside their packing and the list", () => {
    // an open slot behind a meeting, and a whole day of travel
    const events: CalendarEvent[] = [
        {
            key: "open",
            title: "Open slot",
            start: vienna("2026-11-18", "14:00"),
            end: vienna("2026-11-18", "16:00"),
            display: "background",
        },
        {
            key: "planning",
            title: "Planning",
            start: vienna("2026-11-18", "15:00"),
            end: vienna("2026-11-18", "16:00"),
        },
        {
            key: "travel",
            title: "Travel",
            start: "2026-11-19",
            end: "2026-11-19",
            display: "background",
        },
    ];
    const week = drawCalendar("en-GB", "2026-11-18", events, "week");
    const month = drawCalendar("en-GB", "2026-11-18", events, "month");
    const agenda = drawCalendar("en-GB", "2026-11-18", events, "agenda");

    // the meeting takes its column's whole width, and the list shows it alone
    expect({
        week: shadesOf(week.container),
        timed: week.container.querySelectorAll("[data-slot=event-calendar-timed]").length,
        month: shadesOf(month.container),
        monthEvents: eventsOf(month.container),
        agenda: eventsOf(agenda.container),
    }).toEqual({
        week: ["Travel", "Open slot"],
        timed: 1,
        month: ["Travel"],
        monthEvents: ["15:00Planning"],
        agenda: ["15:00 – 16:00Planning"],
    });
});

test("mark today and the time now from the calendar's now", () => {
    // a calendar whose now is a Thursday morning, shown on its week
    const { container } = drawCalendar("en-GB", "2026-11-19", [], "week", {
        now: vienna("2026-11-19", "10:30"),
    });
    const now = container.querySelector<HTMLElement>("[data-slot=event-calendar-now]");
    const column = now?.closest<HTMLElement>("[data-slot=event-calendar-column]");

    // the now line sits in Thursday's column, 10:30 being 43.75% through the day
    expect({ date: column?.dataset["date"], style: now?.getAttribute("style") }).toEqual({
        date: "2026-11-19",
        style: "--x-top: 43.75%;",
    });
});

/** Read an agenda's days as their headings and their events' times and titles. */
function agendaOf(container: HTMLElement): (string | undefined)[][] {
    return [...container.querySelectorAll("[data-slot=event-calendar-agenda] > li")].map((day) => [
        day.querySelector("h3")?.textContent ?? undefined,
        ...[...day.querySelectorAll("[data-slot=event-calendar-event]")].map((event) =>
            plain(event.textContent).replaceAll(/\s/gu, " "),
        ),
    ]);
}

test("repeat a weekday standup at its local time across the end of summer time, leaving out and moving single repeats", () => {
    // a standup each weekday from Monday 19 October, Wednesday's left out and Friday's an hour later
    const standup: CalendarEvent = {
        key: "standup",
        title: "Standup",
        start: vienna("2026-10-19", "09:00"),
        end: vienna("2026-10-19", "09:15"),
        recurrenceRules: [
            {
                frequency: "weekly",
                byDay: [1, 2, 3, 4, 5].map((day) => ({ day })),
            },
        ],
        recurrenceOverrides: {
            "2026-10-28": { excluded: true },
            "2026-10-30": {
                start: vienna("2026-10-30", "10:00"),
                end: vienna("2026-10-30", "10:15"),
            },
        },
    };
    const before = agendaOf(drawCalendar("en-GB", "2026-10-21", [standup], "agenda").container);
    const after = agendaOf(drawCalendar("en-GB", "2026-10-28", [standup], "agenda").container);

    expect({ before, after }).toEqual({
        before: [
            ["Mon 19 Oct", "09:00 – 09:15Standup"],
            ["Tue 20 Oct", "09:00 – 09:15Standup"],
            ["Wed 21 Oct", "09:00 – 09:15Standup"],
            ["Thu 22 Oct", "09:00 – 09:15Standup"],
            ["Fri 23 Oct", "09:00 – 09:15Standup"],
        ],
        after: [
            ["Mon 26 Oct", "09:00 – 09:15Standup"],
            ["Tue 27 Oct", "09:00 – 09:15Standup"],
            ["Thu 29 Oct", "09:00 – 09:15Standup"],
            ["Fri 30 Oct", "10:00 – 10:15Standup"],
        ],
    });
});

test("repeat a meeting at its own zone's wall time, which moves in the calendar's zone as summer times end apart", () => {
    // a weekly 9:00 meeting in New York, shown in Vienna, whose summer time ends a week earlier
    const sync: CalendarEvent = {
        key: "sync",
        title: "Sync",
        start: TimeZone.instant("America/New_York", "2026-10-19", "09:00"),
        end: TimeZone.instant("America/New_York", "2026-10-19", "09:30"),
        timeZone: "America/New_York",
        recurrenceRules: [{ frequency: "weekly" }],
    };
    const weeks = ["2026-10-19", "2026-10-26", "2026-11-02"].map((date) =>
        agendaOf(drawCalendar("en-GB", date, [sync], "agenda").container),
    );

    expect(weeks).toEqual([
        [["Mon 19 Oct", "15:00 – 15:30Sync"]],
        [["Mon 26 Oct", "14:00 – 14:30Sync"]],
        [["Mon 2 Nov", "15:00 – 15:30Sync"]],
    ]);
});

test("move one repeat by the keys, handing on the repeating event with that repeat's override", () => {
    // a weekly review on Wednesdays, this week's lifted and moved a day on
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "review",
                title: "Review",
                start: vienna("2026-11-04", "15:00"),
                end: vienna("2026-11-04", "16:00"),
                recurrenceRules: [{ frequency: "weekly" }],
            },
        ],
        "week",
    );
    container
        .querySelector<HTMLElement>(
            "[data-slot=event-calendar-timed] [data-slot=event-calendar-event]",
        )
        ?.focus();
    for (const key of [" ", "ArrowRight", " "]) {
        press(container, key);
    }
    const [change] = changes;

    expect({
        key: change?.key,
        start: change === undefined ? undefined : wall(change),
        overrides: change?.recurrenceOverrides,
    }).toEqual({
        key: "review",
        start: "2026-11-04 15:00-16:00",
        overrides: {
            "2026-11-18": {
                start: vienna("2026-11-19", "15:00"),
                end: vienna("2026-11-19", "16:00"),
            },
        },
    });
});

test("hand a moved repeat to the choice of changing it alone, it and the later ones, or all of them", () => {
    // a weekly review on Wednesdays since 4 November, this week's moved a day on by the keys
    const moves: CalendarRepeatChange[] = [];
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "review",
                title: "Review",
                start: vienna("2026-11-04", "15:00"),
                end: vienna("2026-11-04", "16:00"),
                recurrenceRules: [{ frequency: "weekly" }],
                recurrenceOverrides: { "2026-11-11": { excluded: true } },
            },
        ],
        "week",
        { onRepeatChange: (move) => moves.push(move) },
    );
    container
        .querySelector<HTMLElement>(
            "[data-slot=event-calendar-timed] [data-slot=event-calendar-event]",
        )
        ?.focus();
    for (const key of [" ", "ArrowRight", " "]) {
        press(container, key);
    }
    const [move] = moves;
    if (move === undefined) {
        throw new TypeError("the move was not handed on");
    }
    const [ended, continued] = move.following("review-from-19");

    // the plain change handler hears nothing once the choice is the caller's
    expect({
        changes,
        only: move.only().recurrenceOverrides,
        all: wall(move.all()),
        ended: [wall(ended), ended.recurrenceRules, ended.recurrenceOverrides],
        continued: [continued.key, wall(continued), continued.recurrenceRules],
    }).toEqual({
        changes: [],
        only: {
            "2026-11-11": { excluded: true },
            "2026-11-18": {
                start: vienna("2026-11-19", "15:00"),
                end: vienna("2026-11-19", "16:00"),
            },
        },
        all: "2026-11-05 15:00-16:00",
        ended: [
            "2026-11-04 15:00-16:00",
            [{ frequency: "weekly", until: "2026-11-17" }],
            { "2026-11-11": { excluded: true } },
        ],
        continued: ["review-from-19", "2026-11-19 15:00-16:00", [{ frequency: "weekly" }]],
    });
});

test("offer the views the parts compose, the days view showing four shown days and paging past hidden ones", () => {
    // four days from a Friday, the weekend hidden, then the next four
    const { container } = drawCalendar("en-US", "2026-11-20", [], "days", { weekends: false });
    const columns = () =>
        [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-column]")].map(
            (column) => column.dataset["date"],
        );
    const title = () => container.querySelector("h2")?.textContent;
    const first = { columns: columns(), title: title() };
    container.querySelector<HTMLElement>("[data-slot=event-calendar-next]")?.click();
    flush();

    expect({
        views: [
            ...container.querySelectorAll("[data-slot=event-calendar-view-switcher] button"),
        ].map((button) => (button.textContent ?? "").replaceAll(ISOLATES, "")),
        first,
        next: { columns: columns(), title: title() },
    }).toEqual({
        views: ["Day", "4 days", "Week", "Month", "Year", "Agenda"],
        first: {
            columns: ["2026-11-20", "2026-11-23", "2026-11-24", "2026-11-25"],
            title: "Nov 20 – 25, 2026",
        },
        next: {
            columns: ["2026-11-26", "2026-11-27", "2026-11-30", "2026-12-01"],
            title: "Nov 26 – Dec 1, 2026",
        },
    });
});

test("show a year of months, each day marked by its events, a press opening its day", () => {
    // a trip from Friday to Monday and a meeting on the Friday
    const events: readonly CalendarEvent[] = [
        { key: "trip", title: "Trip", start: "2026-11-20", end: "2026-11-23" },
        {
            key: "meeting",
            title: "Meeting",
            start: vienna("2026-11-20", "10:00"),
            end: vienna("2026-11-20", "11:00"),
        },
    ];
    const { container } = drawCalendar("en-US", "2026-11-20", events, "year");
    const months = container.querySelectorAll("[data-slot=event-calendar-year-month]").length;
    const marked = [
        ...container.querySelectorAll<HTMLElement>(
            "[data-slot=event-calendar-year-day]:not([data-count='0'])",
        ),
    ].map((day) => `${day.dataset["date"] ?? ""} ${day.dataset["count"] ?? ""}`);
    const title = container.querySelector("h2")?.textContent;
    container
        .querySelector<HTMLElement>("[data-slot=event-calendar-year-day][data-date='2026-11-20']")
        ?.click();
    flush();

    expect({
        months,
        marked,
        title,
        opened: container.querySelector("h2")?.textContent,
    }).toEqual({
        months: 12,
        marked: ["2026-11-20 2", "2026-11-21 1", "2026-11-22 1", "2026-11-23 1"],
        title: "2026",
        opened: "Friday, November 20, 2026",
    });
});

test("draw a line per slot, shade the time outside working hours, and number the weeks", () => {
    // half-hour slots in a week that works nine to five on Wednesdays alone
    const options = {
        slotDuration: 30,
        weekNumbers: true,
        workWeek: { 3: [{ start: "09:00", end: "17:00" }] },
    } as const;
    const week = drawCalendar("en-GB", "2026-11-18", [], "week", options);
    const month = drawCalendar("en-GB", "2026-11-18", [], "month", options);
    const column = (date: string) =>
        week.container.querySelector(`[data-slot=event-calendar-column][data-date='${date}']`);
    const count = (date: string, slot: string) =>
        column(date)?.querySelectorAll(`[data-slot=${slot}]`).length;

    expect({
        wednesday: {
            hours: count("2026-11-18", "event-calendar-hour-line"),
            slots: count("2026-11-18", "event-calendar-slot-line"),
            shaded: count("2026-11-18", "event-calendar-non-working"),
        },
        monday: count("2026-11-16", "event-calendar-non-working"),
        weekNumber: (
            week.container.querySelector("[data-slot=event-calendar-week-number]")?.textContent ??
            ""
        ).replaceAll(ISOLATES, ""),
        monthNumbers: [
            ...month.container.querySelectorAll("[data-slot=event-calendar-week-number]"),
        ].map((number) => (number.textContent ?? "").replaceAll(ISOLATES, "")),
    }).toEqual({
        // twenty-three hour lines and twenty-four half hours inside the day, the hours off around nine to five
        wednesday: { hours: 23, slots: 24, shaded: 2 },
        monday: 1,
        weekNumber: "W47",
        monthNumbers: ["W44", "W45", "W46", "W47", "W48", "W49"],
    });
});

test("color events by their calendar, hide a calendar by its box, and show a teammate's calendar as busy time", () => {
    // a work calendar in green, a teammate shown busy, and a meeting on each
    const opened: string[] = [];
    const events: readonly CalendarEvent[] = [
        {
            key: "standup",
            title: "Standup",
            source: "work",
            start: vienna("2026-11-18", "09:00"),
            end: vienna("2026-11-18", "09:30"),
        },
        {
            key: "dentist",
            title: "Dentist",
            source: "ines",
            start: vienna("2026-11-18", "11:00"),
            end: vienna("2026-11-18", "12:00"),
        },
    ];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <EventCalendar
                events={events}
                sources={[
                    { key: "work", label: "Work", color: "green" },
                    { key: "ines", label: "Ines", display: "busy" },
                ]}
                defaultDate="2026-11-18"
                defaultView="day"
                timeZone={ZONE}
                onAction={(key) => opened.push(key)}
            >
                <EventCalendarSourceList />
                <EventCalendarBody>
                    <EventCalendarTime />
                </EventCalendarBody>
            </EventCalendar>
        </LocaleContext>
    ));
    const shown = () =>
        [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-event]")].map(
            (event) =>
                `${event.dataset["color"] ?? ""}${event.dataset["busy"] === undefined ? "" : " busy"} ${
                    event.querySelector("span:not([aria-hidden])")?.textContent ?? ""
                }`,
        );
    const before = shown();
    for (const event of container.querySelectorAll<HTMLElement>(
        "[data-slot=event-calendar-event]",
    )) {
        event.click();
    }
    container
        .querySelector<HTMLElement>(
            "[data-slot=event-calendar-source][data-source=work] [role=checkbox]",
        )
        ?.click();
    flush();

    // the busy block hides its title and opens nothing, and the hidden calendar's meeting is gone
    expect({ before, opened, after: shown() }).toEqual({
        before: ["green Standup", "blue busy Busy"],
        opened: ["standup"],
        after: ["blue busy Busy"],
    });
});

test("give each resource its days side by side, moving an event onto another's column", () => {
    // move Ada's Friday review a column on by the keys, over Ada's and Grace's work weeks
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "review",
                title: "Review",
                resource: "ada",
                start: vienna("2026-11-20", "10:00"),
                end: vienna("2026-11-20", "11:00"),
            },
        ],
        "week",
        {
            weekends: false,
            resources: [
                { key: "ada", label: "Ada" },
                { key: "grace", label: "Grace" },
            ],
        },
    );
    const columns = [
        ...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-column]"),
    ].map((column) => column.dataset["date"]);
    const resources = [...container.querySelectorAll("[data-slot=event-calendar-resource]")].map(
        (resource) => resource.textContent,
    );
    container
        .querySelector<HTMLElement>(
            "[data-slot=event-calendar-timed] [data-slot=event-calendar-event]",
        )
        ?.focus();
    for (const key of [" ", "ArrowRight", " "]) {
        press(container, key);
    }

    expect({
        resources,
        columns,
        changes: changes.map((change) => [change.resource, wall(change)]),
    }).toEqual({
        resources: ["Ada", "Grace"],
        columns: [
            "2026-11-16",
            "2026-11-17",
            "2026-11-18",
            "2026-11-19",
            "2026-11-20",
            "2026-11-16",
            "2026-11-17",
            "2026-11-18",
            "2026-11-19",
            "2026-11-20",
        ],
        // past Ada's Friday the next column is Grace's Monday, at the same time
        changes: [["grace", "2026-11-16 10:00-11:00"]],
    });
});

test("copy an event by dropping it with Alt held, and move every selected event with the one moved", () => {
    // a planning and a review on Wednesday, the review selected with shift beside the planning
    const events: readonly CalendarEvent[] = [
        {
            key: "planning",
            title: "Planning",
            start: vienna("2026-11-18", "09:00"),
            end: vienna("2026-11-18", "10:00"),
        },
        {
            key: "review",
            title: "Review",
            start: vienna("2026-11-18", "14:00"),
            end: vienna("2026-11-18", "15:00"),
        },
    ];
    const added: CalendarEventDraft[] = [];
    const { container, changes } = drawCalendar("en-GB", "2026-11-18", events, "week", {
        onEventAdd: (event) => added.push(event),
    });
    const chip = (title: string) =>
        [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-event]")].find(
            (each) => (each.textContent ?? "").includes(title),
        );

    // copy the planning a day on, dropping it with Alt held
    chip("Planning")?.focus();
    press(container, " ");
    press(container, "ArrowRight");
    press(container, " ", { altKey: true });
    const copies = added.map((event) => [event.title, wall({ ...event, key: "copy" })]);

    // select both by clicks with shift held, then move the planning a day on, which moves the review along
    for (const title of ["Planning", "Review"]) {
        chip(title)?.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true }));
        flush();
    }
    const selected = [
        ...container.querySelectorAll<HTMLElement>(
            "[data-slot=event-calendar-event][data-selected]",
        ),
    ].map((each) => each.querySelector("span:not([aria-hidden])")?.textContent);
    chip("Planning")?.focus();
    press(container, " ");
    press(container, "ArrowRight");
    press(container, " ");

    expect({
        copies,
        selected,
        changes: changes.map((change) => [change.key, wall(change)]),
    }).toEqual({
        copies: [["Planning", "2026-11-19 09:00-10:00"]],
        selected: ["Planning", "Review"],
        changes: [
            ["planning", "2026-11-19 09:00-10:00"],
            ["review", "2026-11-19 14:00-15:00"],
        ],
    });
});

test("delete and duplicate an event by its keys, a deleted repeat left out of its series", () => {
    // a one-off meeting and a daily standup on Wednesday
    const added: CalendarEventDraft[] = [];
    const removed: CalendarEvent[] = [];
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "meeting",
                title: "Meeting",
                start: vienna("2026-11-18", "11:00"),
                end: vienna("2026-11-18", "12:00"),
            },
            {
                key: "standup",
                title: "Standup",
                start: vienna("2026-11-16", "09:00"),
                end: vienna("2026-11-16", "09:15"),
                recurrenceRules: [{ frequency: "daily" }],
            },
        ],
        "day",
        { onEventAdd: (event) => added.push(event), onEventRemove: (event) => removed.push(event) },
    );
    const chip = (title: string) =>
        [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-event]")].find(
            (each) => (each.textContent ?? "").includes(title),
        );
    chip("Meeting")?.focus();
    press(container, "d", { ctrlKey: true });
    press(container, "Delete");
    chip("Standup")?.focus();
    press(container, "Delete");

    expect({
        added: added.map((event) => event.title),
        removed: removed.map((event) => event.key),
        changes: changes.map((change) => Object.keys(change.recurrenceOverrides ?? {})),
    }).toEqual({
        added: ["Meeting"],
        removed: ["meeting"],
        // the standup's Wednesday repeat is left out of its series
        changes: [["2026-11-18"]],
    });
});

test("preview an event on focus with its times, calendar and actions, a busy event showing none", async () => {
    // a meeting on the work calendar, focused until its card opens
    const { simulation } = testWorld();
    stubPopovers();
    const opened: string[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <EventCalendar
                events={[
                    {
                        key: "meeting",
                        title: "Meeting",
                        source: "work",
                        start: vienna("2026-11-18", "11:00"),
                        end: vienna("2026-11-18", "12:00"),
                    },
                ]}
                sources={[{ key: "work", label: "Work", color: "green" }]}
                defaultDate="2026-11-18"
                defaultView="day"
                timeZone={ZONE}
                onAction={(key) => opened.push(key)}
                onEventRemove={() => undefined}
            >
                <EventCalendarBody>
                    <EventCalendarTime />
                    <EventCalendarPreviewCard />
                </EventCalendarBody>
            </EventCalendar>
        </LocaleContext>
    ));
    container.querySelector<HTMLElement>("[data-slot=event-calendar-event]")?.focus();
    await simulation.sleep(1000);
    flush();
    const preview = container.ownerDocument.querySelector("[data-slot=event-calendar-preview]");
    const actions = [...(preview?.querySelectorAll<HTMLElement>("[data-action]") ?? [])];
    actions.find((action) => action.dataset["action"] === "open-item")?.click();

    expect({
        text: [...(preview?.children ?? [])]
            .filter((child) => child.tagName !== "DIV")
            .map((child) => plain(child.textContent)),
        actions: actions.map((action) => action.textContent),
        opened,
    }).toEqual({
        text: ["Meeting", "Wed 18 Nov, 11:00 – 12:00", "Work"],
        actions: ["Open", "Delete"],
        opened: ["meeting"],
    });
});

test("take a card of another view dropped on a day, with the day it landed on", () => {
    // a board's card beside a month under one provider's drags, the days laid out a hundred pixels each
    const received: CalendarDrop[] = [];
    let card: HTMLElement | undefined;
    const Card = () => {
        const drags = useDragManager();
        const draggable = new Draggable(
            {
                id: "card-copy",
                item: "copy",
                type: "card",
                kind: "move",
                label: () => "Write copy",
                element: () => card,
                home: () => ({ kind: "list", droppable: "board", index: 0 }),
            },
            drags.manager,
        ).attributes([" "]);

        return (
            <div ref={(created: HTMLDivElement) => (card = created)} {...draggable}>
                Write copy
            </div>
        );
    };
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <DragProvider>
                <Card />
                <EventCalendar
                    events={[]}
                    defaultDate="2026-11-18"
                    defaultView="month"
                    timeZone={ZONE}
                    dropTypes={["card"]}
                    onReceive={(drop) => received.push(drop)}
                >
                    <EventCalendarBody>
                        <EventCalendarMonth />
                    </EventCalendarBody>
                </EventCalendar>
            </DragProvider>
        </LocaleContext>
    ));
    const days = [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-day]")];
    for (const [index, day] of days.entries()) {
        day.getBoundingClientRect = () =>
            new DOMRect((index % 7) * 100, 100 + Math.floor(index / 7) * 100, 100, 100);
    }
    const month = container.querySelector<HTMLElement>("[data-slot=event-calendar-month]");
    if (month === null || card === undefined) {
        throw new TypeError("the month or the card did not render");
    }
    month.getBoundingClientRect = () => new DOMRect(0, 100, 700, 600);
    for (const [type, x, y] of [
        ["pointerdown", 10, 10],
        ["pointermove", 120, 150],
        ["pointermove", 250, 350],
        ["pointerup", 250, 350],
    ] as const) {
        card.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: y,
                pointerId: 4,
            }),
        );
        flush();
    }

    // the card lands on the third day of the month grid's third week
    expect(received).toEqual([{ item: "copy", type: "card", start: days[16]?.dataset["date"] }]);
});

test("move, switch views, create and select by the calendar's keys", () => {
    // a meeting on Wednesday, the week shown, keys pressed on the calendar itself
    const { container, created } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "meeting",
                title: "Meeting",
                start: vienna("2026-11-18", "11:00"),
                end: vienna("2026-11-18", "12:00"),
            },
        ],
        "week",
    );
    const calendar = container.querySelector<HTMLElement>("[data-slot=event-calendar]");
    const key = (name: string, options: KeyboardEventInit = {}) => {
        calendar?.dispatchEvent(
            new KeyboardEvent("keydown", {
                key: name,
                bubbles: true,
                cancelable: true,
                ...options,
            }),
        );
        flush();

        return container.querySelector("h2")?.textContent;
    };
    const titles = [key("j"), key("k"), key("m"), key("x"), key("d"), key("w")];
    key("c");
    key("a", { ctrlKey: true });
    const selected = container.querySelectorAll(
        "[data-slot=event-calendar-event][data-selected]",
    ).length;
    key("Escape");

    expect({
        titles,
        created: created.map(
            (span) =>
                `${TimeZone.date(ZONE, span.start)} ${TimeZone.time(ZONE, span.start)}-${TimeZone.time(ZONE, span.end)}`,
        ),
        selected,
        cleared: container.querySelectorAll("[data-slot=event-calendar-event][data-selected]")
            .length,
    }).toEqual({
        titles: [
            "23 – 29 Nov 2026",
            "16 – 22 Nov 2026",
            "November 2026",
            "18 – 21 Nov 2026",
            "Wednesday, 18 November 2026",
            "16 – 22 Nov 2026",
        ],
        // the calendar has no clock in tests, so a new event starts at the date's scroll time
        created: ["2026-11-18 07:00-08:00"],
        selected: 1,
        cleared: 0,
    });
});

test("mark time free by sweeps that join where they meet, write it out, and take a span back", async () => {
    // sweep nine to ten, then half past nine to eleven, in the availability mode
    const changes: (readonly { readonly start: number; readonly end: number }[])[] = [];
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: { writeText: (text: string) => Promise.resolve(void written.push(text)) },
    });
    const { container } = drawCalendar("en-GB", "2026-11-18", [], "day", {
        mode: "availability",
        onAvailabilityChange: (availability) => changes.push(availability),
    });
    const column = container.querySelector<HTMLElement>("[data-slot=event-calendar-column]");
    if (column === null) {
        throw new TypeError("the day column did not render");
    }
    column.getBoundingClientRect = () => new DOMRect(0, 0, 400, 24 * 48);
    const pointer = (type: string, y: number) => {
        column.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: 50,
                clientY: y,
                pointerId: 3,
            }),
        );
        flush();
    };
    for (const [from, to] of [
        [9, 10],
        [9.5, 11],
    ] as const) {
        pointer("pointerdown", from * 48 + 2);
        pointer("pointermove", to * 48);
        pointer("pointerup", to * 48);
        pointer("click", to * 48);
    }
    const blocks = [...container.querySelectorAll("[data-slot=event-calendar-availability]")].map(
        (block) => plain(block.textContent).replace("×", ""),
    );
    container.querySelector<HTMLElement>("[data-slot=event-calendar]")?.dispatchEvent(
        new KeyboardEvent("keydown", {
            key: "c",
            ctrlKey: true,
            shiftKey: true,
            bubbles: true,
            cancelable: true,
        }),
    );
    await Promise.resolve();
    container.querySelector<HTMLElement>("[data-slot=event-calendar-availability-remove]")?.click();
    flush();

    expect({
        changes: changes.map((spans) =>
            spans.map(
                (span) => `${TimeZone.time(ZONE, span.start)}-${TimeZone.time(ZONE, span.end)}`,
            ),
        ),
        blocks,
        written: written.map((text) => plain(text)),
    }).toEqual({
        changes: [["09:00-10:00"], ["09:00-11:00"], []],
        blocks: ["09:00 – 11:00"],
        written: ["Wed 18 Nov: 09:00 – 11:00\nTimes in Europe/Vienna"],
    });
});

test("write the hours in each zone beside the calendar's own, marking the pointer's time in all", () => {
    // write New York beside Vienna, the pointer at Vienna's half past nine
    const { container } = drawCalendar("en-GB", "2026-11-18", [], "day", {
        timeZones: [{ zone: "America/New_York" }],
    });
    const column = container.querySelector<HTMLElement>("[data-slot=event-calendar-column]");
    if (column === null) {
        throw new TypeError("the day column did not render");
    }
    column.getBoundingClientRect = () => new DOMRect(0, 0, 400, 24 * 48);
    column.dispatchEvent(
        new PointerEvent("pointermove", {
            bubbles: true,
            clientX: 50,
            clientY: 9.5 * 48,
            pointerId: 5,
        }),
    );
    flush();
    const zones = [
        ...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-zone-hours]"),
    ];

    expect({
        labels: [...container.querySelectorAll("[data-slot=event-calendar-zone]")].map(
            (zone) => zone.textContent,
        ),
        nines: zones.map((zone) => zone.querySelectorAll("span")[8]?.textContent),
        hovered: zones.map(
            (zone) => zone.querySelector("[data-slot=event-calendar-hover-label]")?.textContent,
        ),
        lines: container.querySelectorAll("[data-slot=event-calendar-hover-line]").length,
    }).toEqual({
        labels: ["New York", "Vienna"],
        // the ninth hour mark of Vienna's day is 09:00 there and 03:00 in New York
        nines: ["03", "09"],
        hovered: ["3:30", "9:30"],
        lines: 1,
    });
});

test("list a week's times, leaving out an event outside it and showing a repeat moved into it", () => {
    // a weekly meeting on Wednesdays, next week's moved to this Friday, and a one-off in December
    const events: readonly CalendarEvent[] = [
        {
            key: "sync",
            title: "Sync",
            start: vienna("2026-11-18", "09:00"),
            end: vienna("2026-11-18", "10:00"),
            recurrenceRules: [{ frequency: "weekly" }],
            recurrenceOverrides: {
                "2026-11-25": {
                    start: vienna("2026-11-20", "15:00"),
                    end: vienna("2026-11-20", "16:00"),
                },
            },
        },
        {
            key: "party",
            title: "Party",
            start: vienna("2026-12-10", "18:00"),
            end: vienna("2026-12-10", "22:00"),
        },
    ];

    expect(agendaOf(drawCalendar("en-GB", "2026-11-18", events, "agenda").container)).toEqual([
        ["Wed 18 Nov", "09:00 – 10:00Sync"],
        ["Fri 20 Nov", "15:00 – 16:00Sync"],
    ]);
});

test("turn an event into one at a time or over whole days, keeping its calendar and resource", () => {
    // a two-day offsite on the team calendar, and a timed review of the same room
    const offsite: CalendarEvent = {
        key: "offsite",
        title: "Offsite",
        start: "2026-11-18",
        end: "2026-11-20",
        source: "team",
        resource: "room",
    };
    const review: CalendarEvent = {
        key: "review",
        title: "Review",
        start: vienna("2026-11-18", "14:00"),
        end: vienna("2026-11-18", "15:30"),
        resource: "room",
    };

    expect([
        CalendarEvent.timedAt(offsite, vienna("2026-11-19", "09:00")),
        CalendarEvent.allDayFrom(review, "2026-11-21", ZONE),
    ]).toEqual([
        // a whole-day event lands as an hour
        {
            key: "offsite",
            title: "Offsite",
            source: "team",
            resource: "room",
            start: vienna("2026-11-19", "09:00"),
            end: vienna("2026-11-19", "10:00"),
        },
        // a timed event covering one day lands as that day
        {
            key: "review",
            title: "Review",
            resource: "room",
            start: "2026-11-21",
            end: "2026-11-21",
        },
    ]);
});

test("step a month from its last weekday to the next month's, and the days view from a hidden date's next shown day", () => {
    // a month from Friday 30 January, the weekend hidden, and four days from Saturday 21 November
    const month = drawCalendar("en-GB", "2026-01-30", [], "month", { weekends: false });
    const days = drawCalendar("en-GB", "2026-11-21", [], "days", { weekends: false });
    for (const { container } of [month, days]) {
        container.querySelector<HTMLElement>("[data-slot=event-calendar-next]")?.click();
        flush();
    }

    expect({
        month: month.container.querySelector("h2")?.textContent,
        days: [
            ...days.container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-column]"),
        ].map((column) => column.dataset["date"]),
    }).toEqual({
        // february 28 is a Saturday, so the step lands on Friday 27 rather than in March
        month: "February 2026",
        days: ["2026-11-27", "2026-11-30", "2026-12-01", "2026-12-02"],
    });
});

/** Draw a week of a daily standup with Monday's and Tuesday's repeats selected. */
function drawStandups(): ReturnType<typeof drawCalendar> & {
    readonly chips: () => HTMLElement[];
} {
    // select the first two repeats with clicks while shift is held
    const drawn = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "standup",
                title: "Standup",
                start: vienna("2026-11-16", "09:00"),
                end: vienna("2026-11-16", "09:15"),
                recurrenceRules: [{ frequency: "daily" }],
            },
        ],
        "week",
    );
    const chips = () => [
        ...drawn.container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-event]"),
    ];
    for (const chip of chips().slice(0, 2)) {
        chip.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true }));
        flush();
    }

    return { ...drawn, chips };
}

test("move two selected repeats of a series a day on, folded into one change of its overrides", () => {
    // lift Monday's repeat and move it to Tuesday, which takes Tuesday's to Wednesday
    const { container, changes, chips } = drawStandups();
    chips()[0]?.focus();
    press(container, " ");
    press(container, "ArrowRight");
    press(container, " ");
    const overrides = changes
        .filter((change) => CalendarEvent.isTimed(change))
        .map((change) =>
            Object.entries(change.recurrenceOverrides ?? {}).map(([date, override]) =>
                "excluded" in override
                    ? date
                    : `${date} ${TimeZone.date(ZONE, override.start)} ${TimeZone.time(ZONE, override.start)}`,
            ),
        );

    expect(overrides).toEqual([["2026-11-16 2026-11-17 09:00", "2026-11-17 2026-11-18 09:00"]]);
});

test("delete two selected repeats of a series in one change, and nothing by keys during a drag", () => {
    // press Delete while Monday's repeat is lifted, then drop it home and press Delete again
    const { container, changes, chips } = drawStandups();
    chips()[0]?.focus();
    press(container, " ");
    press(container, "Delete");
    const during = changes.length;
    press(container, "Escape");
    chips()[0]?.focus();
    press(container, "Delete");

    expect({
        during,
        changes: changes.map((change) => Object.keys(change.recurrenceOverrides ?? {})),
    }).toEqual({ during: 0, changes: [["2026-11-16", "2026-11-17"]] });
});

test("write free time under each resource's label, each resource's spans joined apart", async () => {
    // free mornings for Ana and Ben on Wednesday, copied by Mod+Shift+C
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: { writeText: (text: string) => Promise.resolve(void written.push(text)) },
    });
    const { container } = drawCalendar("en-GB", "2026-11-18", [], "day", {
        mode: "availability",
        resources: [
            { key: "ana", label: "Ana" },
            { key: "ben", label: "Ben" },
        ],
        defaultAvailability: [
            {
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-18", "10:00"),
                resource: "ana",
            },
            {
                start: vienna("2026-11-18", "10:00"),
                end: vienna("2026-11-18", "11:00"),
                resource: "ben",
            },
        ],
    });
    container.querySelector<HTMLElement>("[data-slot=event-calendar]")?.dispatchEvent(
        new KeyboardEvent("keydown", {
            key: "c",
            ctrlKey: true,
            shiftKey: true,
            bubbles: true,
            cancelable: true,
        }),
    );
    await Promise.resolve();

    expect(written.map((text) => plain(text))).toEqual([
        "Ana\nWed 18 Nov: 09:00 – 10:00\nBen\nWed 18 Nov: 10:00 – 11:00\nTimes in Europe/Vienna",
    ]);
});

test("refuse a chosen view that no view part offers", () => {
    // a calendar opening in the year view with a month part alone
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <EventCalendar events={[]} defaultView="year" defaultDate="2026-11-18" timeZone={ZONE}>
                <EventCalendarBody>
                    <EventCalendarMonth />
                </EventCalendarBody>
            </EventCalendar>
        </Errored>
    ));

    expect(container.textContent).toBe("TypeError: calendar view 'year' has no view part");
});

/** The ISO week: Monday first, the weekend on Saturday and Sunday. */
const ISO = { firstDay: 1, weekend: [6, 7], minimalDays: 4 } as const;

/** Read the top and height of each timed event in percent, to three places. */
function boxesOf(container: HTMLElement): number[][] {
    return [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-timed]")].map(
        (event) =>
            ["--x-top", "--x-height"].map(
                (name) =>
                    Math.round(Number.parseFloat(event.style.getPropertyValue(name)) * 1000) / 1000,
            ),
    );
}

test("place times on the wall clock as summer time ends east and west of UTC, a repeated hour a snap high", () => {
    // london's 00:45 to 01:15 before its clocks go back, and New York's 01:30 that happens twice
    const london = drawCalendar(
        "en-GB",
        "2026-10-25",
        [
            {
                key: "london",
                title: "London",
                start: Date.UTC(2026, 9, 24, 23, 45),
                end: Date.UTC(2026, 9, 25, 0, 15),
            },
        ],
        "day",
        { timeZone: "Europe/London" },
    );
    const york = drawCalendar(
        "en-US",
        "2026-11-01",
        [
            {
                key: "york",
                title: "New York",
                start: Date.UTC(2026, 10, 1, 5, 30),
                end: Date.UTC(2026, 10, 1, 6, 30),
            },
        ],
        "day",
        { timeZone: "America/New_York" },
    );

    expect({ london: boxesOf(london.container), york: boxesOf(york.container) }).toEqual({
        // a quarter to one is 0.75 of 24 hours down, half an hour 0.5 of 24 high
        london: [[3.125, 2.083]],
        // half past one on both passes sits at 1.5 hours, a quarter hour high
        york: [[6.25, 1.042]],
    });
});

test("stretch an overnight event's end past midnight by its handle, into the next day's column", () => {
    // from Wednesday 22:00 to Thursday 02:00, each day's column a hundred pixels wide and 48 an hour
    const { container, changes } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "night",
                title: "Night",
                start: vienna("2026-11-18", "22:00"),
                end: vienna("2026-11-19", "02:00"),
            },
        ],
        "week",
    );
    for (const [index, column] of [
        ...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-column]"),
    ].entries()) {
        column.getBoundingClientRect = () => new DOMRect(index * 100, 0, 100, 24 * 48);
    }
    const handle = container.querySelector<HTMLElement>("[data-slot=event-calendar-resize-end]");
    if (handle === null) {
        throw new TypeError("the end handle did not render");
    }

    // press Thursday's handle at two, then pull it to three
    for (const [type, y] of [
        ["pointerdown", 2 * 48],
        ["pointermove", 2 * 48 + 12],
        ["pointermove", 3 * 48],
        ["pointerup", 3 * 48],
    ] as const) {
        handle.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: 350,
                clientY: y,
                pointerId: 5,
                width: 1,
                height: 1,
            }),
        );
        flush();
    }

    expect(changes.map(wall)).toEqual(["2026-11-18 22:00-03:00"]);
});

test("change every repeat as one moved, dropping its own override, and offer no following on the first", () => {
    // a daily meeting from Monday, Wednesday's moved to three and then dragged to four
    const series: CalendarEvent = {
        key: "sync",
        title: "Sync",
        start: vienna("2026-11-16", "09:00"),
        end: vienna("2026-11-16", "10:00"),
        recurrenceRules: [{ frequency: "daily" }],
        recurrenceOverrides: {
            "2026-11-18": {
                start: vienna("2026-11-18", "15:00"),
                end: vienna("2026-11-18", "16:00"),
            },
        },
    };
    const repeat: CalendarEvent = {
        ...series,
        key: "sync@2026-11-18",
        start: vienna("2026-11-18", "15:00"),
        end: vienna("2026-11-18", "16:00"),
    };
    const moved: CalendarEvent = {
        ...repeat,
        start: vienna("2026-11-18", "16:00"),
        end: vienna("2026-11-18", "17:00"),
    };
    const change = new CalendarRepeatChange(series, "2026-11-18", repeat, moved, ZONE, ISO);
    const all = change.all();
    const first = new CalendarRepeatChange(series, "2026-11-16", series, series, ZONE, ISO);
    const removal = new CalendarRepeatRemoval(series, "2026-11-16", ZONE, ISO);
    const refusals = [() => first.following("next"), () => removal.following()].map((refuse) => {
        try {
            return refuse();
        } catch (error) {
            return error;
        }
    });

    expect({
        all: [wall(all), all.recurrenceOverrides],
        isFirst: [change.isFirst, first.isFirst, removal.isFirst],
        refusals,
    }).toEqual({
        // the series moves to four, where Wednesday's repeat was dropped, which follows it again
        all: ["2026-11-16 16:00-17:00", {}],
        isFirst: [false, true, true],
        refusals: [
            new TypeError("calendar repeat '2026-11-16' is its series' first"),
            new TypeError("calendar repeat '2026-11-16' is its series' first"),
        ],
    });
});

test("select all but shading, and leave a hidden calendar's selected events out of a removal", () => {
    // a meeting at work, a lunch at home, and shading for an offsite, everything selected
    const removed: CalendarEvent[] = [];
    const [hidden, setHidden] = createSignal<readonly string[]>([]);
    const { container } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "meeting",
                title: "Meeting",
                source: "work",
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-18", "10:00"),
            },
            {
                key: "lunch",
                title: "Lunch",
                source: "home",
                start: vienna("2026-11-18", "12:00"),
                end: vienna("2026-11-18", "13:00"),
            },
            {
                key: "offsite",
                title: "Offsite",
                display: "background",
                start: vienna("2026-11-18", "14:00"),
                end: vienna("2026-11-18", "17:00"),
            },
        ],
        "day",
        {
            sources: [
                { key: "work", label: "Work" },
                { key: "home", label: "Home" },
            ],
            get hiddenSources() {
                return hidden();
            },
            onEventRemove: (event) => removed.push(event),
        },
    );
    const root = container.querySelector<HTMLElement>("[data-slot=event-calendar]");
    const selected = () =>
        [
            ...container.querySelectorAll<HTMLElement>(
                "[data-slot=event-calendar-event][data-selected]",
            ),
        ].length;
    root?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "a", ctrlKey: true, bubbles: true, cancelable: true }),
    );
    flush();
    const all = selected();

    // hide the home calendar, then delete the selection
    setHidden(["home"]);
    flush();
    root?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true }),
    );
    flush();

    expect({ all, removed: removed.map((event) => event.key) }).toEqual({
        all: 2,
        removed: ["meeting"],
    });
});

/** Read a meeting on Wednesday morning by its key. */
function meetingOf(key: string): CalendarEvent {
    return {
        key,
        title: key,
        start: vienna("2026-11-18", "09:00"),
        end: vienna("2026-11-18", "10:00"),
    };
}

test("land drops in the calendar they start in when two calendars share one provider's drags", () => {
    // two weeks side by side under one provider, a meeting in each, the first one's moved a day on
    const changes: string[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <DragProvider>
                <For each={["first", "second"]}>
                    {(key) => (
                        <EventCalendar
                            events={[meetingOf(key)]}
                            defaultDate="2026-11-18"
                            defaultView="week"
                            timeZone={ZONE}
                            onEventChange={(change) => changes.push(`${key}: ${wall(change)}`)}
                        >
                            <EventCalendarBody>
                                <EventCalendarTime />
                            </EventCalendarBody>
                        </EventCalendar>
                    )}
                </For>
            </DragProvider>
        </LocaleContext>
    ));
    container.querySelector<HTMLElement>("[data-slot=event-calendar-event]")?.focus();
    for (const key of [" ", "ArrowRight", " "]) {
        press(container, key);
    }

    expect(changes).toEqual(["first: 2026-11-19 09:00-10:00"]);
});

test("change every repeat of a weekday series as one moved, its rules taking the moved repeat's weekday", () => {
    // a Monday, Wednesday and Friday standup, Wednesday's moved to Thursday
    const series: CalendarEvent = {
        key: "standup",
        title: "Standup",
        start: vienna("2026-11-16", "09:00"),
        end: vienna("2026-11-16", "09:15"),
        recurrenceRules: [{ frequency: "weekly", byDay: [{ day: 1 }, { day: 3 }, { day: 5 }] }],
    };
    const repeat = CalendarEvent.repeatOn(series, "2026-11-18", ZONE);
    const moved = CalendarEvent.shift(repeat, 24 * 60, ZONE);
    const all = new CalendarRepeatChange(series, "2026-11-18", repeat, moved, ZONE, ISO).all();

    expect([wall(all), all.recurrenceRules]).toEqual([
        "2026-11-17 09:00-09:15",
        [{ frequency: "weekly", byDay: [{ day: 1 }, { day: 4 }, { day: 5 }] }],
    ]);
});

test("go on from a moved repeat on its own zone's weekday, the calendar's zone a day behind", () => {
    // a Monday breakfast at eight in Tokyo, shown on Sundays in Los Angeles, one moved an hour later
    const series: CalendarEvent = {
        key: "breakfast",
        title: "Breakfast",
        timeZone: "Asia/Tokyo",
        start: TimeZone.instant("Asia/Tokyo", "2026-11-02", "08:00"),
        end: TimeZone.instant("Asia/Tokyo", "2026-11-02", "09:00"),
        recurrenceRules: [{ frequency: "weekly", byDay: [{ day: 1 }] }],
    };
    const zone = "America/Los_Angeles";
    const repeat = CalendarEvent.repeatOn(series, "2026-11-16", zone);
    const moved = CalendarEvent.shift(repeat, 60, zone);
    const [, continued] = new CalendarRepeatChange(
        series,
        "2026-11-16",
        repeat,
        moved,
        zone,
        ISO,
    ).following("later");
    const dates = CalendarEvent.occurrences(
        continued,
        { from: "2026-11-14", to: "2026-11-30" },
        zone,
        ISO,
    ).map((each) => CalendarEvent.first(each.event, zone));

    expect({ rules: continued.recurrenceRules, dates }).toEqual({
        rules: [{ frequency: "weekly", byDay: [{ day: 1 }] }],
        // tokyo's Mondays, the 16th, 23rd and 30th
        dates: ["2026-11-16", "2026-11-23", "2026-11-30"],
    });
});

test("show a far zone's daily repeats on every day of a week, those across the week's edges included", () => {
    // eight in the morning on Kiritimati, which is the morning before in Honolulu
    const event: CalendarEvent = {
        key: "call",
        title: "Call",
        timeZone: "Pacific/Kiritimati",
        start: TimeZone.instant("Pacific/Kiritimati", "2026-11-01", "08:00"),
        end: TimeZone.instant("Pacific/Kiritimati", "2026-11-01", "09:00"),
        recurrenceRules: [{ frequency: "daily" }],
    };
    const zone = "Pacific/Honolulu";
    const days = CalendarEvent.occurrences(
        event,
        { from: "2026-11-09", to: "2026-11-15" },
        zone,
        ISO,
    ).map((each) => CalendarEvent.span(each.event, zone).start);

    expect(days).toEqual([
        "2026-11-09",
        "2026-11-10",
        "2026-11-11",
        "2026-11-12",
        "2026-11-13",
        "2026-11-14",
        "2026-11-15",
    ]);
});

test("mark the month's day a keyboard drag aims at", () => {
    // a meeting on Wednesday lifted and moved a day on in the month
    const { container } = drawCalendar(
        "en-GB",
        "2026-11-18",
        [
            {
                key: "meeting",
                title: "Meeting",
                start: vienna("2026-11-18", "09:00"),
                end: vienna("2026-11-18", "10:00"),
            },
        ],
        "month",
    );
    container.querySelector<HTMLElement>("[data-slot=event-calendar-event]")?.focus();
    press(container, " ");
    press(container, "ArrowRight");
    const targets = [...container.querySelectorAll<HTMLElement>("[data-target]")].map(
        (cell) => cell.dataset["date"],
    );
    press(container, "Escape");

    expect(targets).toEqual(["2026-11-19"]);
});

test("remove repeats by the handlers the calendar has: a lone one by its choice, several by none", () => {
    // a daily standup in a calendar that hands removed repeats to a choice alone
    const removals: CalendarRepeatRemoval[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-GB"), [])}>
            <EventCalendar
                events={[
                    {
                        key: "standup",
                        title: "Standup",
                        start: vienna("2026-11-16", "09:00"),
                        end: vienna("2026-11-16", "09:15"),
                        recurrenceRules: [{ frequency: "daily" }],
                    },
                ]}
                defaultDate="2026-11-18"
                defaultView="week"
                timeZone={ZONE}
                onRepeatRemove={(removal) => removals.push(removal)}
            >
                <EventCalendarBody>
                    <EventCalendarTime />
                </EventCalendarBody>
            </EventCalendar>
        </LocaleContext>
    ));
    const chips = [...container.querySelectorAll<HTMLElement>("[data-slot=event-calendar-event]")];

    // delete one repeat, then select two and press Delete, which the calendar cannot hand on
    chips[2]?.focus();
    press(container, "Delete");
    for (const chip of chips.slice(0, 2)) {
        chip.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true }));
        flush();
    }
    chips[0]?.focus();
    press(container, "Delete");
    const later = removals[0]?.following();

    expect({
        removals: removals.map((removal) => removal.recurrenceId),
        rules: later?.recurrenceRules,
    }).toEqual({
        removals: ["2026-11-18"],
        rules: [{ frequency: "daily", until: "2026-11-17" }],
    });
});
