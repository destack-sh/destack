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
    properties: {
        disabled: false,
    },
    render: (properties) => {
        const [birthday, setBirthday] = createSignal<PlainDate>();

        return (
            <Field>
                <FieldLabel>Birthday</FieldLabel>
                <DateField {...properties} value={birthday()} onValueChange={setBirthday} />
                <FieldDescription>Shown to the people you share notes with.</FieldDescription>
            </Field>
        );
    },
});

/** A date field with the day a note was archived. */
export const dateFieldArchived = defineExample({
    of: DateField,
    name: "archived",
    description: "a date field with the day a note was archived",
    render: () => <DateField aria-label="Archived" defaultValue={"2026-10-07"} />,
});

/** The archive date marked invalid. */
export const dateFieldArchivedInvalid = defineExample({
    of: DateField,
    name: "archived-invalid",
    description: "the archive date marked invalid",
    render: () => (
        <DateField aria-label="Archived" aria-invalid="true" defaultValue={"2026-10-07"} />
    ),
});

/** The archive date unavailable. */
export const dateFieldArchivedDisabled = defineExample({
    of: DateField,
    name: "archived-disabled",
    description: "the archive date unavailable",
    render: () => <DateField aria-label="Archived" disabled defaultValue={"2026-10-07"} />,
});
