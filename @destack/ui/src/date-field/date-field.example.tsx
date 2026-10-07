import { defineExample } from "@destack/package/declare";
import type { PlainDate } from "@destack/schema";
import { createSignal } from "@destack/view";
import { Field, FieldDescription, FieldLabel } from "../field/index.ts";
import { DateField } from "./date-field.tsx";

/** A date field that types a person's birthday, labelled and described by its field. */
export const dateFieldBirthday = defineExample({
    of: DateField,
    name: "birthday",
    description: "a date field that types a person's birthday, labelled and described by its field",
    render: () => {
        const [birthday, setBirthday] = createSignal<PlainDate>();

        return (
            <Field>
                <FieldLabel>Birthday</FieldLabel>
                <DateField value={birthday()} onValueChange={setBirthday} />
                <FieldDescription>Shown to the people you share notes with.</FieldDescription>
            </Field>
        );
    },
});

/** A date field holding the day a note was archived. */
export const dateFieldArchived = defineExample({
    of: DateField,
    name: "archived",
    description: "a date field holding the day a note was archived",
    render: () => (
        <DateField aria-label="Archived" defaultValue={{ year: 2026, month: 10, day: 7 }} />
    ),
});
