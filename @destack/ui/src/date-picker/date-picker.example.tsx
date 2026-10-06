import { defineExample } from "@destack/package/declare";
import type { PlainDate } from "@destack/schema";
import { createSignal } from "@destack/view";
import { Day } from "../calendar/index.ts";
import { DatePicker } from "./date-picker.tsx";

/** A date picker that sets a task's due day. */
export const datePickerDueDay = defineExample({
    of: DatePicker,
    name: "due-day",
    description: "a date picker that sets a task's due day",
    render: () => {
        const [due, setDue] = createSignal<PlainDate>();

        return <DatePicker aria-label="Due date" value={due()} onValueChange={setDue} />;
    },
});

/** A date picker that filters notes by a span, offering the last week and month beside its calendar. */
export const datePickerEditedSpan = defineExample({
    of: DatePicker,
    name: "edited-span",
    description:
        "a date picker that filters notes by a span, offering the last week and month beside its calendar",
    render: () => (
        <DatePicker
            mode="range"
            aria-label="Edited"
            presets={[
                {
                    label: "Last 7 days",
                    value: { from: Day.parse("2026-09-28"), to: Day.parse("2026-10-04") },
                },
                {
                    label: "Last 30 days",
                    value: { from: Day.parse("2026-09-05"), to: Day.parse("2026-10-04") },
                },
            ]}
        />
    ),
});
