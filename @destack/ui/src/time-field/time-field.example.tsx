import { t } from "@destack/locale";
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
    description: t`A time field that sets the hour a daily reminder rings.`,
    render: (properties) => {
        const [time, setTime] = createSignal<PlainTime>("08:30");

        return (
            <Field>
                <FieldLabel>Reminder</FieldLabel>
                <TimeField {...properties} value={time()} onValueChange={setTime} />
            </Field>
        );
    },
});

/** A date picker beside a time field that schedule a note's reminder at a day and time. */
export const timeFieldBesideDatePicker = defineExample({
    of: TimeField,
    name: "besideDatePicker",
    description: t`A date picker beside a time field that schedule a note's reminder at a day and time.`,
    properties: {
        "aria-label": "Remind at",
        defaultValue: "09:00",
    },
    render: (properties) => (
        <div {...style.attrs(styles.row)}>
            <DatePicker aria-label="Remind on" defaultSelected={"2026-10-09"} />
            <TimeField {...properties} />
        </div>
    ),
});
