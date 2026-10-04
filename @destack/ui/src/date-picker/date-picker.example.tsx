import type { PlainDate } from "@destack/schema";
import type { JSX } from "@solidjs/web";
import { createSignal } from "solid-js";
import { DatePicker } from "./date-picker.tsx";

/** Show a date picker that sets a task's due day. */
export function DatePickerExample(): JSX.Element {
    const [due, setDue] = createSignal<PlainDate>();

    return <DatePicker aria-label="Due date" value={due()} onValueChange={setDue} />;
}
