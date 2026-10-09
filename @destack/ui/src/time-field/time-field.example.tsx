import { defineExample } from "@destack/package/declare";
import type { PlainTime } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { DatePicker } from "../date-picker/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { TimeField } from "./time-field.tsx";

/** The row of a day and a time. */
const styles = style.create({
    row: { display: "flex", gap: space[2] },
});

/** A time field that sets the hour a daily reminder rings. */
export const timeFieldReminder = defineExample({
    of: TimeField,
    name: "reminder",
    description: "a time field that sets the hour a daily reminder rings",
    render: () => {
        const [time, setTime] = createSignal<PlainTime>("08:30");

        return (
            <Field>
                <FieldLabel>Reminder</FieldLabel>
                <TimeField value={time()} onValueChange={setTime} />
            </Field>
        );
    },
});

/** A date picker beside a time field that schedule a note's reminder at a day and time. */
export const timeFieldBesideDatePicker = defineExample({
    of: TimeField,
    name: "beside-date-picker",
    description:
        "a date picker beside a time field that schedule a note's reminder at a day and time",
    render: () => (
        <div {...style.attrs(styles.row)}>
            <DatePicker
                aria-label="Remind on"
                defaultSelected={{ year: 2026, month: 10, day: 9 }}
            />
            <TimeField aria-label="Remind at" defaultValue="09:00" />
        </div>
    ),
});
