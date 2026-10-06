import { defineExample } from "@destack/package/declare";
import { PlainDate } from "@destack/schema";
import { createSignal } from "@destack/view";
import { Calendar } from "./calendar.tsx";
import { Day } from "./day.ts";
import type { DateRange } from "./selection.ts";

/** Report whether a day lies before today. */
function isPast(day: PlainDate): boolean {
    return PlainDate.compare(day, Day.today()) < 0;
}

/** A calendar that picks the days of a trip, past days disabled. */
export const calendarTripDays = defineExample({
    of: Calendar,
    name: "trip-days",
    description: "a calendar that picks the days of a trip, past days disabled",
    render: () => {
        const [trip, setTrip] = createSignal<DateRange | undefined>(undefined);

        return <Calendar mode="range" value={trip()} onValueChange={setTrip} isDisabled={isPast} />;
    },
});

/** A calendar at the end of October with the thirtieth selected and the fourth as today. */
export const calendarLateOctober = defineExample({
    of: Calendar,
    name: "late-october",
    description:
        "a calendar at the end of October with the thirtieth selected and the fourth as today",
    render: () => (
        <Calendar defaultValue={Day.parse("2026-10-30")} today={Day.parse("2026-10-04")} />
    ),
});

/** A calendar that picks a stay across two months shown side by side. */
export const calendarStayTwoMonths = defineExample({
    of: Calendar,
    name: "stay-two-months",
    description: "a calendar that picks a stay across two months shown side by side",
    render: () => (
        <Calendar
            mode="range"
            months={2}
            defaultValue={{ from: Day.parse("2026-10-28"), to: Day.parse("2026-11-03") }}
            today={Day.parse("2026-10-04")}
        />
    ),
});

/** A birthday calendar with month and year pickers, numbered weeks and no later day than today. */
export const calendarBirthday = defineExample({
    of: Calendar,
    name: "birthday",
    description:
        "a birthday calendar with month and year pickers, numbered weeks and no later day than today",
    render: () => (
        <Calendar
            captionLayout="dropdown"
            weekNumbers
            outsideDays={false}
            max={Day.parse("2026-10-04")}
            today={Day.parse("2026-10-04")}
            defaultMonth={Day.parse("1990-06-01")}
        />
    ),
});
