import { defineExample } from "@destack/package/declare";
import { createSignal } from "solid-js";
import { Input } from "../input/index.ts";
import { Textarea } from "../textarea/index.ts";
import {
    Field,
    FieldDescription,
    FieldError,
    FieldGroup,
    FieldLabel,
    FieldLegend,
    FieldSet,
} from "./field.tsx";

/** A notebook form whose name field reports an error until it has a value. */
export const fieldNotebookForm = defineExample({
    of: Field,
    name: "notebook-form",
    description: "a notebook form whose name field reports an error until it has a value",
    render: () => {
        const [name, setName] = createSignal("");
        const error = (): string | undefined => (name() === "" ? "enter a name" : undefined);

        return (
            <FieldSet>
                <FieldLegend>Notebook</FieldLegend>
                <FieldGroup>
                    <Field invalid={error() !== undefined}>
                        <FieldLabel>Name</FieldLabel>
                        <Input
                            value={name()}
                            onInput={(event) => setName(event.currentTarget.value)}
                        />
                        <FieldError errors={[{ message: error() }]} />
                    </Field>
                    <Field>
                        <FieldLabel>Description</FieldLabel>
                        <Textarea />
                        <FieldDescription>
                            Shown to everyone you share the notebook with.
                        </FieldDescription>
                    </Field>
                </FieldGroup>
            </FieldSet>
        );
    },
});
