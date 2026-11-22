import { Locale, Localization } from "@destack/locale";
import { TimeZone } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush, LocaleContext } from "@destack/view";
import { render } from "@destack/view/test";
import {
    type CalendarEvent,
    type CalendarRepeatMove,
    EventCalendar,
    EventCalendarBody,
    EventCalendarHeader,
    EventCalendarNext,
    type EventCalendarOptions,
    EventCalendarTitle,
    type EventCalendarView,
} from "./index.ts";

/** The time zone the test calendar runs in, which leaves summer time on 25 October 2026. */
const ZONE = "Europe/Vienna";

/** The marks that isolate interpolated names in rendered messages. */
const ISOLATES = /[⁦-⁩]/gu;

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
                {...options}
                events={events}
                defaultDate={date}
                defaultView={view}
                timeZone={ZONE}
                onEventChange={(change) => changes.push(change)}
                onCreate={(span) => created.push(span)}
            />
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
                    : `${(element.textContent ?? "").replaceAll(ISOLATES, "")} ${element.dataset["start"] ?? ""}..${element.dataset["end"] ?? ""}`,
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
        (event.textContent ?? "").replaceAll(ISOLATES, ""),
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

test("move a timed event two days across the end of summer time, keeping its local start and end", () => {
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
    container.querySelector<HTMLElement>("[data-slot=event-calendar-resize]")?.focus();
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
    const handles = container.querySelectorAll("[data-slot=event-calendar-resize]").length;

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

    return container.querySelectorAll("[data-slot=event-calendar-scroller] [aria-hidden] span")
        .length;
}

test("label the inner hours of the day summer time ends, one more than on other days", () => {
    expect([countHourLabels("2026-10-25"), countHourLabels("2026-10-26")]).toEqual([24, 23]);
});

test("drag a timed event by its middle an hour down, moving it by the travel, as its landing shows", () => {
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
    const isLanding = container.querySelector("[data-slot=event-calendar-landing]") !== null;
    pointer("pointerup", 10.5 * 48);

    expect({ changes: changes.map(wall), isLanding }).toEqual({
        changes: ["2026-11-18 10:00-11:00"],
        isLanding: true,
    });
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
                event.textContent?.replaceAll(ISOLATES, "").replaceAll(/\s/gu, " "),
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
                slotDuration={30}
                onEventChange={(change) => changes.push(change)}
            >
                <EventCalendarHeader>
                    <EventCalendarTitle />
                    <EventCalendarNext />
                </EventCalendarHeader>
                <EventCalendarBody>
                    {(event) => (
                        <button type="button" data-slot="own-event" ref={event.ref}>
                            own {event.event.title}
                        </button>
                    )}
                </EventCalendarBody>
            </EventCalendar>
        </LocaleContext>
    ));
    const header = [
        ...(container.querySelector("[data-slot=event-calendar-header]")?.children ?? []),
    ];
    const own = container.querySelector("[data-slot=own-event]")?.textContent;

    // step the meeting's end a slot later
    container.querySelector<HTMLElement>("[data-slot=event-calendar-resize]")?.focus();
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
            event.textContent?.replaceAll(ISOLATES, "").replaceAll(/\s/gu, " "),
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
    const moves: CalendarRepeatMove[] = [];
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
