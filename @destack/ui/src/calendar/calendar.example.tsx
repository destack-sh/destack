import { defineExample } from "@destack/package/declare";
import { PlainDate } from "@destack/schema";
import { createSignal } from "solid-js";
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
