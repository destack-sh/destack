import { present } from "@destack/schema";

/** The fields of a wall-clock minute, as `Intl.DateTimeFormat` names them. */
const FIELDS = ["year", "month", "day", "hour", "minute"] as const;

/** A minute as a time zone's wall clock shows it: its date, hour and minute. */
export class WallTime {
    /** The year. */
    readonly year: number;
    /** The month, from 1. */
    readonly month: number;
    /** The day of the month, from 1. */
    readonly day: number;
    /** The hour, from 0 to 23. */
    readonly hour: number;
    /** The minute. */
    readonly minute: number;

    /** Take a wall-clock minute by its fields. */
    constructor(fields: Pick<WallTime, (typeof FIELDS)[number]>) {
        // set the date, hour and minute
        this.year = fields.year;
        this.month = fields.month;
        this.day = fields.day;
        this.hour = fields.hour;
        this.minute = fields.minute;
    }

    /** Read the minute a time zone's wall clock shows at an instant. */
    static of(instant: number, timeZone: string): WallTime {
        // read the date and time fields the time zone shows
        const parts = new Intl.DateTimeFormat("en-CA", {
            timeZone,
            year: "numeric",
            month: "numeric",
            day: "numeric",
            hour: "numeric",
            minute: "numeric",
            hourCycle: "h23",
        }).formatToParts(new Date(instant));
        const part = (type: (typeof FIELDS)[number]) =>
            Number(
                present(
                    parts.find((each) => each.type === type),
                    `the ${type}`,
                ).value,
            );

        return new WallTime({
            year: part("year"),
            month: part("month"),
            day: part("day"),
            hour: part("hour"),
            minute: part("minute"),
        });
    }

    /** Read how far a time zone's clock is ahead of UTC at an instant, in milliseconds. */
    static offset(instant: number, timeZone: string): number {
        return WallTime.of(instant, timeZone).utc() - instant;
    }

    /** Read the instant this wall-clock minute would be in UTC. */
    utc(): number {
        return Date.UTC(this.year, this.month - 1, this.day, this.hour, this.minute);
    }

    /** Write the minute as its date and time, such as 2026-10-09 17:30. */
    format(): string {
        // pad each field to its width
        const [month, day, hour, minute] = [this.month, this.day, this.hour, this.minute].map(
            (field) => String(field).padStart(2, "0"),
        );

        return `${this.year}-${month}-${day} ${hour}:${minute}`;
    }
}
