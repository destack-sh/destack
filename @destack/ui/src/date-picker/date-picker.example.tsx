import { t } from "@destack/locale";
import { PlainDate } from "@destack/schema";
import { defineExample } from "@destack/package/declare";
import { createSignal, omit } from "@destack/view";
import { Field, FieldError, FieldLabel } from "../field/index.ts";
import { DatePicker } from "./date-picker.tsx";

/** The day the examples count as today, fixed so their renders stay the same. */
const EXAMPLE_TODAY = "2026-10-04";

/** A date picker that sets a task's due day. */
export const datePickerDueDay = defineExample({
    of: DatePicker,
    name: "dueDay",
    description: t`A date picker that sets a task's due day.`,
    properties: {
        placeholder: "Pick a date",
        "aria-label": "Due date",
    },
    render: (properties) => {
        const [due, setDue] = createSignal<PlainDate>();

        return (
            <DatePicker
                {...omit(properties, "mode", "selected", "defaultSelected", "onSelect")}
                today={EXAMPLE_TODAY}
                selected={due()}
                onSelect={setDue}
            />
        );
    },
});

/** A date picker that filters notes by a span. */
export const datePickerEditedSpan = defineExample({
    of: DatePicker,
    name: "editedSpan",
    description: t`A date picker that filters notes by a span, offering the last week and month beside its calendar.`,
    render: () => (
        <DatePicker
            today={EXAMPLE_TODAY}
            mode="range"
            aria-label="Edited"
            presets={[
                {
                    label: "Last 7 days",
                    value: { from: "2026-09-28", to: "2026-10-04" },
                },
                {
                    label: "Last 30 days",
                    value: { from: "2026-09-05", to: "2026-10-04" },
                },
            ]}
        />
    ),
});

/** A date picker that types or picks the day a trip starts and ends. */
export const datePickerTrip = defineExample({
    of: DatePicker,
    name: "trip",
    description: t`A date picker that types or picks the day a trip starts and ends.`,
    render: () => (
        <DatePicker
            today={EXAMPLE_TODAY}
            mode="range"
            aria-label="Trip"
            defaultSelected={{
                from: "2026-10-12",
                to: "2026-10-16",
            }}
        />
    ),
});

/** A date picker that marks the days off in a month, counted on its trigger. */
export const datePickerDaysOff = defineExample({
    of: DatePicker,
    name: "daysOff",
    description: t`A date picker that marks the days off in a month, counted on its trigger.`,
    render: () => (
        <DatePicker
            today={EXAMPLE_TODAY}
            mode="multiple"
            aria-label="Days off"
            defaultSelected={["2026-10-05", "2026-10-06"]}
        />
    ),
});

/** The due day picker with its calendar open below its fields. */
export const datePickerDueDayOpen = defineExample({
    of: DatePicker,
    name: "dueDayOpen",
    description: t`The due day picker with its calendar open below its fields.`,
    render: () => (
        <DatePicker
            today={EXAMPLE_TODAY}
            aria-label="Due date"
            defaultSelected={"2026-10-12"}
            defaultOpen
        />
    ),
});

/** A due day in a field that reports the day is missing. */
export const datePickerDueDayInvalid = defineExample({
    of: DatePicker,
    name: "dueDayInvalid",
    description: t`A due day in a field that reports the day is missing.`,
    render: () => (
        <Field invalid>
            <FieldLabel>Due date</FieldLabel>
            <DatePicker today={EXAMPLE_TODAY} />
            <FieldError errors={[{ message: "pick a due date" }]} />
        </Field>
    ),
});

/** A due day in a field that takes no input while its form saves. */
export const datePickerDueDayDisabled = defineExample({
    of: DatePicker,
    name: "dueDayDisabled",
    description: t`A due day in a field that takes no input while its form saves.`,
    render: () => (
        <Field disabled>
            <FieldLabel>Due date</FieldLabel>
            <DatePicker today={EXAMPLE_TODAY} />
        </Field>
    ),
});
