import { PlainDate } from "@destack/schema";
import type { JSX } from "@solidjs/web";
import { createSignal } from "solid-js";
import { Calendar } from "./calendar.tsx";
import { Day } from "./day.ts";
import type { DateRange } from "./selection.ts";

/** Report whether a day lies before today. */
function isPast(day: PlainDate): boolean {
    return PlainDate.compare(day, Day.today()) < 0;
}

/** Show a calendar that picks the days of a trip, past days disabled. */
export function CalendarExample(): JSX.Element {
    const [trip, setTrip] = createSignal<DateRange | undefined>(undefined);

    return <Calendar mode="range" value={trip()} onValueChange={setTrip} isDisabled={isPast} />;
}
