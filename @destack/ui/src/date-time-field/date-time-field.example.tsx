import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { DateTimeField } from "./date-time-field.tsx";

/** When a note reminds, in a zone. */
export const dateTimeFieldReminder = defineExample({
    of: DateTimeField,
    name: "reminder",
    description: t`When a note reminds, in a zone.`,
    render: (properties) => (
        <Field>
            <FieldLabel>Remind at</FieldLabel>
            <DateTimeField
                defaultValue={Date.UTC(2026, 10, 24, 9, 30)}
                timeZone="Europe/Zurich"
                {...properties}
            />
        </Field>
    ),
});

/** When a booking starts, within its season, in a form that needs it. */
export const dateTimeFieldBooking = defineExample({
    of: DateTimeField,
    name: "booking",
    description: t`When a booking starts, within its season, in a form that needs it.`,
    render: (properties) => (
        <form onSubmit={(event) => event.preventDefault()}>
            <Field>
                <FieldLabel>Arrival</FieldLabel>
                <DateTimeField
                    name="arrival"
                    required
                    min={Date.UTC(2026, 5, 1)}
                    max={Date.UTC(2026, 8, 30)}
                    hourCycle="h23"
                    timeZone="Europe/Zurich"
                    {...properties}
                />
            </Field>
            <Button type="submit">Book</Button>
            <Button type="reset" variant="ghost">
                Start over
            </Button>
        </form>
    ),
});
