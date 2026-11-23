import { PlainDate } from "@destack/schema";
import { type JSX, Show } from "@destack/view";
import { Calendar, type CalendarProperties } from "../calendar/index.ts";
import { useEventCalendar } from "./control.ts";

/** The properties of a calendar's navigator: a month's look, its date the calendar's. */
export type EventCalendarNavigatorProperties = Omit<
    CalendarProperties,
    "mode" | "selected" | "defaultSelected" | "onSelect" | "defaultMonth" | "today"
>;

/** Render a month that moves the calendar to the day pressed. */
export function EventCalendarNavigator(properties: EventCalendarNavigatorProperties): JSX.Element {
    // show the month of the calendar's date, again whenever the date leaves it
    const control = useEventCalendar();
    const month = () => PlainDate.with(control.date(), { day: 1 });

    return (
        <Show when={month()} keyed>
            {(shown) => (
                <Calendar
                    data-slot="event-calendar-navigator"
                    showWeekNumber={control.options.weekNumbers === true}
                    {...properties}
                    defaultMonth={shown}
                    selected={control.date()}
                    onSelect={(day) => control.setDate(day)}
                    {...todayOf(control.today())}
                />
            )}
        </Show>
    );
}

/** Read the navigator's today, none until the calendar knows it. */
function todayOf(today: PlainDate | undefined): { readonly today?: PlainDate } {
    return today === undefined ? {} : { today };
}
