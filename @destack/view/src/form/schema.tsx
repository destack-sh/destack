import { t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
import { type JsonObject, type JsonValue, schema } from "@destack/schema";
import { Button } from "@destack/ui/button";
import { Checkbox } from "@destack/ui/checkbox";
import { Field, FieldGroup, FieldLabel } from "@destack/ui/field";
import { Input } from "@destack/ui/input";
import { Select } from "@destack/ui/select";
import type { JSX } from "../solid/component.ts";
import { For, Match, Switch } from "../solid/flow.ts";

/** A JSON Schema property a form renders a field for: its type, its choices and its title. */
const Property = schema.looseObject({
    /** The JSON type. */
    type: schema.union([schema.string(), schema.array(schema.string())]).exactOptional(),
    /** The values it takes, rendered as choices. */
    enum: schema.array(schema.json()).exactOptional(),
    /** The label. */
    title: schema.string().exactOptional(),
    /** The help text. */
    description: schema.string().exactOptional(),
});

/** The JSON Schema of an object a form edits: its properties and the ones it requires. */
const ObjectSchema = schema.looseObject({
    /** The properties by name. */
    properties: schema.record(schema.string(), Property).exactOptional(),
    /** The names of the properties it requires. */
    required: schema.array(schema.string()).exactOptional(),
});

/** One field a schema form renders: its name, label, control and whether it is required. */
interface FormField {
    /** The property's name. */
    readonly name: string;
    /** The label. */
    readonly title: string;
    /** The control the property's type takes. */
    readonly control: "text" | "number" | "boolean" | "choice" | "json";
    /** The choices of an enumerated property. */
    readonly choices: readonly JsonValue[];
    /** Whether the property is required. */
    readonly isRequired: boolean;
}

/** The properties of a schema form. */
export interface SchemaFormProperties {
    /** The JSON Schema of the object the form edits. */
    readonly schema: JsonObject;
    /** The submit button's label, Run by default. */
    readonly submit?: string;
    /** Handle the submitted object. */
    readonly onSubmit: (value: JsonObject) => void;
}

/** Render a form for an object a JSON Schema describes, one field per property, as a method's input asks for it. */
export function SchemaForm(properties: SchemaFormProperties): JSX.Element {
    // render a field per property of the schema
    const locale = useLocale();
    const fields = () => fieldsOf(properties.schema);

    // read each field's control into the object, leaving out empty optional ones
    const submit = (event: SubmitEvent) => {
        event.preventDefault();
        const form = event.currentTarget;
        if (form instanceof HTMLFormElement) {
            properties.onSubmit(valueOf(fields(), (name) => readControl(form, name)));
        }
    };

    return (
        <form data-slot="schema-form" onSubmit={submit}>
            <FieldGroup>
                <For each={fields()}>
                    {(field) => (
                        <Field>
                            <FieldLabel>{field.title}</FieldLabel>
                            <Switch>
                                {/* Choices */}
                                <Match when={field.control === "choice"}>
                                    <Select name={field.name} required={field.isRequired}>
                                        <For each={field.choices}>
                                            {(choice) => (
                                                <option value={JSON.stringify(choice)}>
                                                    {typeof choice === "string"
                                                        ? choice
                                                        : JSON.stringify(choice)}
                                                </option>
                                            )}
                                        </For>
                                    </Select>
                                </Match>

                                {/* Yes or no */}
                                <Match when={field.control === "boolean"}>
                                    <Checkbox name={field.name} />
                                </Match>

                                {/* Text, numbers and JSON */}
                                <Match when={true}>
                                    <Input
                                        name={field.name}
                                        type={field.control === "number" ? "number" : "text"}
                                        required={field.isRequired}
                                    />
                                </Match>
                            </Switch>
                        </Field>
                    )}
                </For>
            </FieldGroup>
            <Button type="submit">{properties.submit ?? locale.render(t`Run`)}</Button>
        </form>
    );
}

/** List the fields of an object's JSON Schema, in property order. */
export function fieldsOf(described: JsonObject): FormField[] {
    const parsed = ObjectSchema.parse(described);
    const required = new Set(parsed.required ?? []);

    return Object.entries(parsed.properties ?? {}).map(([name, property]) => ({
        name,
        title: property.title ?? name,
        control: controlOf(property),
        choices: property.enum ?? [],
        isRequired: required.has(name),
    }));
}

/** Choose the control a property's type takes: choices, a checkbox, a number, text, or JSON for anything else. */
function controlOf(property: schema.Infer<typeof Property>): FormField["control"] {
    const types = new Set([property.type ?? []].flat());

    // offer an enumeration's values as choices
    if (property.enum !== undefined) {
        return "choice";
    }
    // check a boolean
    else if (types.has("boolean")) {
        return "boolean";
    }
    // type a number or text
    else if (types.has("number") || types.has("integer")) {
        return "number";
    } else if (types.has("string")) {
        return "text";
    }

    return "json";
}

/** Read a form's control by name: a checkbox as whether it is checked, any other as its text. */
function readControl(form: HTMLFormElement, name: string): string | boolean | undefined {
    const control = form.elements.namedItem(name);
    if (control instanceof HTMLInputElement && control.type === "checkbox") {
        return control.checked;
    } else if (control instanceof HTMLInputElement || control instanceof HTMLSelectElement) {
        return control.value;
    }

    return undefined;
}

/** Read a submitted form's controls into the object its fields describe, leaving out empty optional fields. */
export function valueOf(
    fields: readonly FormField[],
    read: (name: string) => string | boolean | undefined,
): JsonObject {
    const entries = fields.flatMap((field): [string, JsonValue][] => {
        // read a checkbox as whether it is checked
        const text = read(field.name);
        if (field.control === "boolean") {
            return [[field.name, text === true]];
        } else if (typeof text !== "string" || (text === "" && !field.isRequired)) {
            return [];
        }

        // read a number, a choice's JSON and JSON as values, and text as it is
        if (field.control === "number") {
            return [[field.name, Number(text)]];
        } else if (field.control === "choice" || field.control === "json") {
            return [[field.name, schema.json().parse(JSON.parse(text))]];
        }

        return [[field.name, text]];
    });

    return Object.fromEntries(entries);
}
