import { CalendarDay, CalendarLayout, type ObjectType, type TimeWindow } from "@destack/object";
import { Locale } from "@destack/locale";
import type { JsonObject } from "@destack/schema";
import { Cell } from "../cell.ts";
import { Frame } from "../frame.ts";
import type { Page } from "../page.ts";

/** The margin a drawing starts at, beside the gutter's words. */
const MARGIN = "  ";

/** The days of a week. */
const WEEK_DAYS = 7;

/** The cells a day takes in a calendar's month: two digits and a gap. */
const DAY_CELL = 3;

/** A calendar drawn for a terminal: its month, and an agenda of the days with objects. */
export const CalendarPage = {
    /** Draw a calendar as a page: its month above the agenda's days and objects. */
    of(
        frame: Frame,
        rows: readonly JsonObject[],
        layout: CalendarLayout,
        type: ObjectType,
        window?: TimeWindow,
    ): Page {
        // group the rows under their days, below the month
        const days = CalendarLayout.days(layout, rows, frame.terminal.timeZone, window)
            .filter((day) => day.rows.length > 0)
            .map((day) => ({
                key: day.date,
                heading: CalendarPage.date(frame, day),
                rows: day.rows,
            }));
        const header = [...CalendarPage.month(frame, rows, layout, type, window), ""];

        return frame.grouped(days, type, [layout.by], header, rows);
    },

    /** Draw a calendar page as its month beside its agenda, as khal does. */
    draw(frame: Frame, page: Page): string {
        // pair the month's lines with the agenda's, the month padded to its width
        const month = page.header
            .filter((line) => line !== "")
            .map((line) => line.slice(MARGIN.length));
        const width = Math.max(...month.map((line) => Cell.width(line)));
        const agenda = page.lines.map((line) => line.text.slice(MARGIN.length));
        const rule = frame.terminal.style("dim", "│");
        const lines = Array.from({ length: Math.max(month.length, agenda.length) }, (_, index) => {
            const left = month[index] ?? "";
            const padded = `${left}${" ".repeat(Math.max(0, width - Cell.width(left)))}`;

            return `${MARGIN}${padded}  ${rule} ${agenda[index] ?? ""}`.trimEnd();
        });

        return [...lines, "", page.footer].join("\n");
    },

    /** Draw a calendar's month: its name, the weekdays from the locale's first, the days. */
    month(
        frame: Frame,
        rows: readonly JsonObject[],
        layout: CalendarLayout,
        type: ObjectType,
        window: TimeWindow | undefined,
    ): string[] {
        // read the days, and the day the frame draws at
        const { terminal } = frame;
        const days = CalendarLayout.days(layout, rows, terminal.timeZone, window);
        const [start] = days;
        const end = days.at(-1);
        if (start === undefined || end === undefined) {
            return [`${MARGIN}${terminal.style("dim", `no ${type.plural} with ${layout.by}`)}`];
        }
        const [today] = CalendarLayout.days(
            layout,
            [{ [layout.by]: frame.now }],
            terminal.timeZone,
        );

        // head the month by its name and the weekdays from the locale's first one
        const firstDay = Locale.firstDay(terminal.locale);
        const span = new Intl.DateTimeFormat(terminal.locale, {
            month: "long",
            year: "numeric",
            timeZone: "UTC",
        });
        const weekday = new Intl.DateTimeFormat(terminal.locale, {
            weekday: "short",
            timeZone: "UTC",
        });
        const names = Array.from({ length: WEEK_DAYS }, (_, index) =>
            weekday
                .format(Date.UTC(2024, 0, firstDay + index))
                .slice(0, DAY_CELL - 1)
                .padEnd(DAY_CELL),
        );
        const lines = [
            `${MARGIN}${terminal.style("bold", span.formatRange(CalendarDay.instant(start), CalendarDay.instant(end)))}`,
            `${MARGIN}${terminal.style("dim", names.join("").trimEnd())}`,
        ];

        // pad the first week to its weekday, then draw each day, bold with objects and inverted today
        const offset = (CalendarDay.weekday(start) - firstDay + WEEK_DAYS) % WEEK_DAYS;
        const cells = [...Array.from({ length: offset }, () => undefined), ...days];
        for (let week = 0; week < cells.length; week += WEEK_DAYS) {
            const drawn = cells.slice(week, week + WEEK_DAYS).map((day) => {
                const number =
                    day === undefined
                        ? ""
                        : String(Number(day.date.slice(8))).padStart(DAY_CELL - 1);
                const styled =
                    day?.date === today?.date
                        ? terminal.style("inverse", number)
                        : day !== undefined && day.rows.length > 0
                          ? terminal.style("bold", number)
                          : terminal.style("dim", number);

                return `${styled}${" ".repeat(DAY_CELL - Cell.width(number))}`;
            });
            lines.push(`${MARGIN}${drawn.join("")}`.trimEnd());
        }

        return lines;
    },

    /** Write a calendar day as a person reads it, such as Fri, Nov 20. */
    date(frame: Frame, day: CalendarDay): string {
        const format = new Intl.DateTimeFormat(frame.terminal.locale, {
            weekday: "short",
            day: "numeric",
            month: "short",
            timeZone: "UTC",
        });

        return format.format(CalendarDay.instant(day));
    },

    /** Write each row's day and title for a program. */
    records(
        frame: Frame,
        rows: readonly JsonObject[],
        layout: CalendarLayout,
        type: ObjectType,
        window?: TimeWindow,
    ): string[][] {
        return CalendarLayout.days(layout, rows, frame.terminal.timeZone, window).flatMap((day) =>
            day.rows.map((row) => [day.date, Frame.titleText(type, row)]),
        );
    },
};
