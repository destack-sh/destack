import type { ObjectValue, StateStage } from "@destack/object";
import type { Terminal } from "../output/index.ts";

/** The nearest a time lies to now and still reads relative, a week in milliseconds. */
const RELATIVE_SPAN = 604_800_000;

/** The units a relative time counts in, each from its length in milliseconds up. */
const RELATIVE_UNITS = [
    ["minute", 60_000],
    ["hour", 3_600_000],
    ["day", 86_400_000],
] as const;

/** The mark of each state stage, as Linear's status icons draw them. */
const STAGE_MARKS: Readonly<Record<StateStage, string>> = {
    initial: "○",
    active: "◐",
    final: "●",
};

/** The microseconds in a millisecond. */
const MICROSECONDS_PER_MILLISECOND = 1000;

/** The milliseconds in a second. */
const MILLISECONDS_PER_SECOND = 1000;

/** What a person reads for an absent value. */
const ABSENT = "–";

/** What a program reads for an absent value. */
const ABSENT_PROGRAM = "-";

/** The styles, zone and locale a cell writes its value in. */
type Surface = Pick<Terminal, "style" | "tone" | "isTerminal" | "timeZone" | "locale">;

/** One value as a terminal draws it: its text, its alignment and the paint fitted text takes. */
export interface Cell {
    /** The value as plain text. */
    readonly text: string;
    /** The side the text lines up on, numbers at the end. */
    readonly align: "start" | "end";
    /** Paint fitted text: dim, bold or in a tone. */
    readonly paint: (fitted: string) => string;
}

/** One value as a terminal draws it. */
export const Cell = {
    /** Write plain text as a cell. */
    plain(text: string): Cell {
        return { text: flatten(text), align: "start", paint: (fitted) => fitted };
    },

    /** Write an object's value for a person, or whole for a program, at a moment. */
    of(value: ObjectValue, surface: Surface, now: number): Cell {
        // write a program's value whole
        const dim = (fitted: string) => surface.style("dim", fitted);
        if (!surface.isTerminal) {
            return Cell.plain(program(value));
        }
        // write absence dim
        else if (value.kind === "absent") {
            return { text: ABSENT, align: "start", paint: dim };
        }
        // write a number in the locale, at the end
        else if (value.kind === "number") {
            return {
                ...Cell.plain(new Intl.NumberFormat(surface.locale).format(value.value)),
                align: "end",
            };
        }
        // write a time near now relative, any other as a date
        else if (value.kind === "time") {
            return Cell.plain(time(value.value, surface, now));
        }
        // write a boolean as a check, false dim
        else if (value.kind === "boolean") {
            return value.value ? Cell.plain("✓") : { text: ABSENT, align: "start", paint: dim };
        }
        // mark a state by its stage, toned by it
        else if (value.kind === "state") {
            const text = `${STAGE_MARKS[value.stage]} ${value.value}`;
            const paint =
                value.stage === "initial"
                    ? dim
                    : (fitted: string) =>
                          surface.tone(value.stage === "final" ? "done" : "accent", fitted);

            return { text, align: "start", paint };
        }
        // write structure as JSON, dim
        else if (value.kind === "json") {
            return { ...Cell.plain(JSON.stringify(value.value)), paint: dim };
        }

        return Cell.plain(value.value);
    },

    /** Write a duration in milliseconds as people read it, such as 120ms or 1.20s. */
    duration(milliseconds: number): string {
        // write under a millisecond in microseconds, under a second in milliseconds
        if (milliseconds < 1) {
            return `${Math.round(milliseconds * MICROSECONDS_PER_MILLISECOND)}µs`;
        } else if (milliseconds < MILLISECONDS_PER_SECOND) {
            return `${milliseconds.toFixed(milliseconds < 10 ? 1 : 0)}ms`;
        }

        return `${(milliseconds / MILLISECONDS_PER_SECOND).toFixed(2)}s`;
    },

    /** Measure text's visible width in terminal cells. */
    width(text: string): number {
        return Bun.stringWidth(text);
    },

    /** Cut text wider than a width at a grapheme, with an ellipsis. */
    cut(text: string, width: number): string {
        // leave text that fits
        if (Cell.width(text) <= width) {
            return text;
        }

        // take graphemes, which the root locale splits, while they and the ellipsis fit
        let kept = "";
        for (const { segment } of new Intl.Segmenter("und").segment(text)) {
            if (Cell.width(kept + segment) > width - 1) {
                break;
            }
            kept += segment;
        }

        return `${kept}…`;
    },

    /** Fit a cell to a width, cut or padded on its alignment's other side, then painted. */
    fit(cell: Cell, width: number): string {
        const fitted = Cell.cut(cell.text, width);
        const space = " ".repeat(Math.max(0, width - Cell.width(fitted)));

        return cell.align === "end"
            ? `${space}${cell.paint(fitted)}`
            : `${cell.paint(fitted)}${space}`;
    },
};

/** Write a value whole for a program: times in ISO 8601, structure as JSON. */
function program(value: ObjectValue): string {
    // write absence as a dash, a time in ISO 8601 and structure as JSON
    if (value.kind === "absent") {
        return ABSENT_PROGRAM;
    } else if (value.kind === "time") {
        return new Date(value.value).toISOString();
    } else if (value.kind === "json") {
        return JSON.stringify(value.value);
    }

    return String(value.value);
}

/** Write a time near a moment relative to it, any other as a date in the locale and zone. */
function time(instant: number, surface: Surface, now: number): string {
    // count a time within a week in its largest whole unit
    const delta = instant - now;
    const distance = Math.abs(delta);
    if (distance < RELATIVE_SPAN) {
        const [unit, length] =
            RELATIVE_UNITS.findLast(([, each]) => distance >= each) ?? RELATIVE_UNITS[0];
        const relative = new Intl.RelativeTimeFormat(surface.locale, { numeric: "auto" });

        return relative.format(Math.round(delta / length), unit);
    }

    // write the date, with the year when it differs from now's and the time when it is not midnight
    const zone = { timeZone: surface.timeZone, hourCycle: "h23" } as const;
    const clock = new Intl.DateTimeFormat("en", { ...zone, hour: "2-digit", minute: "2-digit" });
    const year = new Intl.DateTimeFormat("en", { ...zone, year: "numeric" });
    const isMidnight = clock.format(instant) === "00:00";
    const isThisYear = year.format(instant) === year.format(now);
    const date = new Intl.DateTimeFormat(surface.locale, {
        ...zone,
        weekday: "short",
        day: "numeric",
        month: "short",
        ...(isThisYear ? {} : { year: "numeric" }),
        ...(isMidnight ? {} : { hour: "2-digit", minute: "2-digit" }),
    });

    return date.format(instant);
}

/** Write text on one line, its tabs and line breaks as spaces, so each row stays one record. */
function flatten(written: string): string {
    return written.replaceAll(/[\t\r\n]/gu, " ");
}
