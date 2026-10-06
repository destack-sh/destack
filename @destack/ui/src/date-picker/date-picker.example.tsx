import { defineExample } from "@destack/package/declare";
import type { PlainDate } from "@destack/schema";
import { createSignal } from "solid-js";
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
