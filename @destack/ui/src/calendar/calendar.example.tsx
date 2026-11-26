import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { PlainDate } from "@destack/schema";
import { createSignal, omit } from "@destack/view";
import { Calendar } from "./calendar.tsx";
import type { DateRange } from "./selection.ts";

/** The day the trip example takes as today, fixed so its render stays the same. */
const TRIP_TODAY = "2026-10-04";

/** Report whether a day lies before the trip example's today. */
function isPast(day: PlainDate): boolean {
    return PlainDate.compare(day, TRIP_TODAY) < 0;
}

/** A calendar that picks the days of a trip, past days disabled. */
export const calendarTripDays = defineExample({
    of: Calendar,
    name: "tripDays",
    description: t`A calendar that picks the days of a trip, past days disabled.`,
    properties: {
        numberOfMonths: 1,
        showWeekNumber: false,
    },
    render: (properties) => {
        const [trip, setTrip] = createSignal<DateRange | undefined>(undefined);

        return (
            <Calendar
                {...omit(properties, "mode", "selected", "defaultSelected", "onSelect")}
                mode="range"
                selected={trip()}
                onSelect={setTrip}
                isDateUnavailable={isPast}
                today={TRIP_TODAY}
            />
        );
    },
});

/** A calendar at the end of October with the thirtieth selected and the fourth as today. */
export const calendarLateOctober = defineExample({
    of: Calendar,
    name: "lateOctober",
    description: t`A calendar at the end of October with the thirtieth selected and the fourth as today.`,
    render: (properties) => (
        <Calendar
            numberOfMonths={properties.numberOfMonths}
            captionLayout={properties.captionLayout}
            showWeekNumber={properties.showWeekNumber}
            defaultSelected={"2026-10-30"}
            today={"2026-10-04"}
        />
    ),
});

/** A calendar that picks a stay across two months shown side by side. */
export const calendarStayTwoMonths = defineExample({
    of: Calendar,
    name: "stayTwoMonths",
    description: t`A calendar that picks a stay across two months shown side by side.`,
    properties: { numberOfMonths: 2 },
    render: (properties) => (
        <Calendar
            numberOfMonths={properties.numberOfMonths}
            captionLayout={properties.captionLayout}
            showWeekNumber={properties.showWeekNumber}
            mode="range"
            defaultSelected={{
                from: "2026-10-28",
                to: "2026-11-03",
            }}
            today={"2026-10-04"}
        />
    ),
});

/** A birthday calendar with month and year pickers, numbered weeks and no later day than today. */
export const calendarBirthday = defineExample({
    of: Calendar,
    name: "birthday",
    description: t`A birthday calendar with month and year pickers, numbered weeks and no later day than today.`,
    properties: { captionLayout: "dropdown", showWeekNumber: true },
    render: (properties) => (
        <Calendar
            numberOfMonths={properties.numberOfMonths}
            captionLayout={properties.captionLayout}
            showWeekNumber={properties.showWeekNumber}
            showOutsideDays={false}
            max={"2026-10-04"}
            today={"2026-10-04"}
            defaultMonth={"1990-06-01"}
        />
    ),
});
