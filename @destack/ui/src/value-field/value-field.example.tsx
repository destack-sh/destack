import type { JsonValue } from "@destack/schema";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { Field, FieldLabel } from "../field/index.ts";
import { ValueField, ValueFieldProvider } from "./value-field.tsx";

/** The styles of the examples. */
const styles = style.create({
    form: {
        display: "grid",
        gap: space[4],
        width: "100%",
        maxWidth: "22rem",
    },
});

/** The people a person picks from. */
const PEOPLE = ["Ada", "Grace", "Barbara"].map((name) => ({ value: name, title: name }));

/** The input of each kind of value, with what it gives. */
export const valueFieldKinds = defineExample({
    of: ValueField,
    name: "kinds",
    description: "the input of each kind of value, with what it gives",
    layout: "padded",
    render: () => {
        // keep what the inputs give
        const [given, setGiven] = createSignal<Readonly<Record<string, JsonValue | undefined>>>({
            share: 0.25,
            pinned: true,
            person: "Ada",
        });
        const of = (name: string) => ({
            get value() {
                return given()[name];
            },
            onValueChange: (value: JsonValue | undefined) =>
                setGiven((current) => ({ ...current, [name]: value })),
        });

        return (
            <div {...style.attrs(styles.form)}>
                <Field>
                    <FieldLabel>Title</FieldLabel>
                    <ValueField type={{ kind: "text" }} {...of("title")} />
                </Field>
                <Field>
                    <FieldLabel>Share</FieldLabel>
                    <ValueField
                        type={{
                            kind: "number",
                            minimum: 0,
                            maximum: 1,
                            step: 0.05,
                            format: { style: "percent" },
                        }}
                        {...of("share")}
                    />
                </Field>
                <Field orientation="horizontal">
                    <FieldLabel>Pinned</FieldLabel>
                    <ValueField type={{ kind: "boolean" }} {...of("pinned")} />
                </Field>
                <Field>
                    <FieldLabel>Due</FieldLabel>
                    <ValueField type={{ kind: "time" }} {...of("due")} />
                </Field>
                <Field>
                    <FieldLabel>State</FieldLabel>
                    <ValueField
                        type={{ kind: "choice", choices: [{ value: "open" }, { value: "done" }] }}
                        choices={[
                            { value: "open", title: "Open" },
                            { value: "done", title: "Done" },
                        ]}
                        {...of("state")}
                    />
                </Field>
                <Field>
                    <FieldLabel>Person</FieldLabel>
                    <ValueField type={{ kind: "person" }} choices={PEOPLE} {...of("person")} />
                </Field>
                <output>{JSON.stringify(given())}</output>
            </div>
        );
    },
});

/** A number an app takes by an input of its own. */
export const valueFieldReplaced = defineExample({
    of: ValueFieldProvider,
    name: "replaced",
    description: "a number an app takes by an input of its own",
    render: () => (
        <ValueFieldProvider
            inputs={{
                number: (properties) => (
                    <input
                        type="range"
                        aria-label="Stars"
                        min={properties.type.minimum}
                        max={properties.type.maximum}
                        onInput={(event) =>
                            properties.onValueChange(Number(event.currentTarget.value))
                        }
                    />
                ),
            }}
        >
            <ValueField
                type={{ kind: "number", minimum: 0, maximum: 5, step: 1 }}
                value={undefined}
                onValueChange={() => {}}
            />
        </ValueFieldProvider>
    ),
});
