import { type JSX, Show } from "@destack/view";
import { type EventCalendarView, useEventCalendar } from "./control.ts";

/** Offer a view part's views to the calendar, and render the part while one of them shows. */
export function OfferedView(properties: {
    /** The views the part shows. */
    readonly views: readonly EventCalendarView[];
    /** Render the part, mounted only while one of its views shows. */
    readonly children: () => JSX.Element;
}): JSX.Element {
    const control = useEventCalendar();
    control.addViews(properties.views);

    return <Show when={properties.views.includes(control.view())}>{properties.children()}</Show>;
}
