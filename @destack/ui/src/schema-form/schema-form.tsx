import { t } from "@destack/locale";
import {
    fromJsonSchema,
    type JsonObject,
    type JsonValue,
    type PlainDate,
    PlainTime,
    schema,
} from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import {
    createSignal,
    For,
    type FormField,
    Repeat,
    type JSX,
    Show,
    untrack,
    useLocale,
    useSchemaForm,
} from "@destack/view";
import { Button } from "../button/index.ts";
import { Day } from "../calendar/index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { DatePicker } from "../date-picker/index.ts";
import {
    Field,
    FieldDescription,
    FieldError,
    FieldGroup,
    FieldLabel,
    FieldLegend,
    FieldSet,
} from "../field/index.ts";
import { FieldContext, useFieldControl } from "../field/control.ts";
import { Input } from "../input/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import { TimeField } from "../time-field/index.ts";
import {
    emptyOf,
    listOf,
    objectOf,
    type SchemaArrayField,
    type SchemaChoiceField,
    type SchemaField,
    type SchemaNumberField,
    type SchemaObjectField,
    type SchemaTextField,
    type SchemaVariantField,
    schemaFields,
    variantOf,
} from "./field.ts";

/** The native input type of each text control. */
const INPUT_TYPES = {
    text: "text",
    email: "email",
    url: "url",
    json: "text",
} as const;

/** The minutes of an hour. */
const HOUR_MINUTES = 60;

/** A `YYYY-MM-DD` date, as the JSON Schema `date` format writes it. */
const DATE_TEXT = /^\d{4}-\d{2}-\d{2}$/u;

/** The pattern of an RFC 3339 full time, capturing its hour and minute. */
const TIME_TEXT = /^(\d{2}):(\d{2}):\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/iu;

/** The styles of a schema form's lists and fields. */
const styles = style.create({
    moment: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[2],
    },
    row: {
        display: "flex",
        alignItems: "flex-end",
        gap: space[2],
    },
    item: {
        flexGrow: 1,
    },
});

/** The properties of a schema form. */
export interface SchemaFormProperties {
    /** The JSON Schema of the object the form edits, such as a method's input, read once as the form renders. */
    readonly schema: JsonObject;
    /** The submit button's label, Submit by default. */
    readonly submit?: string;
    /** Render a field's control instead of the form, such as a picker for a reference, or keep the form's with undefined. */
    readonly renderField?: (field: SchemaField, held: SchemaHeld) => JSX.Element | undefined;
    /** Handle the object once every field is kept. */
    readonly onSubmit: (value: JsonObject) => void;
}

/** The value a field of a schema form holds, how to edit it and why it is refused. */
export type SchemaHeld = FormField<JsonValue | undefined>;

/** The values a schema form holds, by property. */
type SchemaValues = Record<string, JsonValue | undefined>;

/** Render a form for an object a JSON Schema describes, one field per property, refusing values the schema refuses in the person's language. */
export function SchemaForm(properties: SchemaFormProperties): JSX.Element {
    // check the object against the schema, each property starting from its start value
    const locale = useLocale();
    const fields = schemaFields(properties.schema);
    const form = useSchemaForm<SchemaValues>(fromJsonSchema(properties.schema), {
        values: () => Object.fromEntries(fields.map((field) => [field.name, field.start])),
        submit: (value) => {
            const kept = pruned(value);
            properties.onSubmit(kept);

            return { predicted: Promise.resolve(kept), confirmed: Promise.resolve() };
        },
    });

    // render each property's field over the form's own value and refusals
    const problemAt = (pointer: string): string | undefined => form.problems().get(pointer);
    const render = (field: SchemaField, held: SchemaHeld, pointer: string): JSX.Element =>
        properties.renderField?.(field, held) ?? (
            <FieldControl
                field={field}
                held={held}
                pointer={pointer}
                problemAt={problemAt}
                render={render}
            />
        );

    return (
        <form
            data-slot="schema-form"
            novalidate
            onSubmit={(event) => {
                event.preventDefault();
                form.submit();
            }}
        >
            <FieldGroup>
                <For each={fields}>
                    {(field) => {
                        const held = form.field(field.name);
                        const pointer = pointerOf("", field.name);

                        return render(
                            field,
                            {
                                value: () => held.value(),
                                set: (value) => held.set(value),
                                problem: () => problemAt(pointer),
                            },
                            pointer,
                        );
                    }}
                </For>
                <Button type="submit">{properties.submit ?? locale.render(t`Submit`)}</Button>
            </FieldGroup>
        </form>
    );
}

/** The properties of one field's control. */
interface FieldControlProperties<Field extends SchemaField = SchemaField> {
    /** The field. */
    readonly field: Field;
    /** Its value, how to edit it and why it is refused. */
    readonly held: SchemaHeld;
    /** The JSON Pointer of its value, which its inner values' refusals sit under. */
    readonly pointer: string;
    /** Read why the value at a pointer is refused, absent while it is kept. */
    readonly problemAt: (pointer: string) => string | undefined;
    /** Render an inner field, through the form's override first. */
    readonly render: (field: SchemaField, held: SchemaHeld, pointer: string) => JSX.Element;
}

/** Render the control a field takes. */
function FieldControl(properties: FieldControlProperties): JSX.Element {
    const field = properties.field;

    // render the control of the field's kind
    switch (field.control) {
        case "constant":
            return undefined;
        case "boolean":
            return <BooleanControl {...properties} field={field} />;
        case "choice":
            return <ChoiceControl {...properties} field={field} />;
        case "choices":
            return <ChoicesControl {...properties} field={field} />;
        case "object":
            return <ObjectControl {...properties} field={field} />;
        case "array":
            return <ArrayControl {...properties} field={field} />;
        case "variant":
            return <VariantControl {...properties} field={field} />;
        case "number":
            return <NumberControl {...properties} field={field} />;
        case "date":
            return <DateControl {...properties} field={field} />;
        case "time":
            return <TimeControl {...properties} field={field} />;
        case "date-time":
            return <MomentControl {...properties} field={field} />;
        case "text":
        case "email":
        case "url":
        case "json":
            return <TextControl {...properties} field={field} control={field.control} />;
    }
}

/** Render a text input in the type its format takes, holding JSON for a schema without a control. */
function TextControl(
    properties: FieldControlProperties<SchemaTextField> & {
        /** The control, one a native input types. */
        readonly control: keyof typeof INPUT_TYPES;
    },
): JSX.Element {
    const field = properties.field;

    return (
        <Labelled field={field} held={properties.held}>
            <Input
                type={INPUT_TYPES[properties.control]}
                aria-required={field.isRequired ? "true" : undefined}
                value={textOf(properties.held.value())}
                onInput={(event) => properties.held.set(valueOf(field, event.currentTarget.value))}
            />
        </Labelled>
    );
}

/** Render a date picker writing a `YYYY-MM-DD` date. */
function DateControl(properties: FieldControlProperties<SchemaTextField>): JSX.Element {
    const field = properties.field;

    return (
        <Labelled field={field} held={properties.held}>
            <DatePicker
                value={dayOf(properties.held.value())}
                onValueChange={(day) =>
                    properties.held.set(day === undefined ? clearedOf(field) : Day.key(day))
                }
            />
        </Labelled>
    );
}

/** Render a time field writing the UTC time of day of the local time typed, at today's offset. */
function TimeControl(properties: FieldControlProperties<SchemaTextField>): JSX.Element {
    const field = properties.field;

    return (
        <Labelled field={field} held={properties.held}>
            <TimeField
                value={wallTimeOf(properties.held.value())}
                onValueChange={(time) =>
                    properties.held.set(time === undefined ? clearedOf(field) : fullTimeOf(time))
                }
            />
        </Labelled>
    );
}

/** Render a date picker beside a time field, writing the instant of the local date and time as RFC 3339 in UTC. */
function MomentControl(properties: FieldControlProperties<SchemaTextField>): JSX.Element {
    // start from the local date and time of the instant held
    const locale = useLocale();
    const field = properties.field;
    const start = untrack(() => localMomentOf(properties.held.value()));
    let day = start?.day;
    let time = start?.time;

    // write the instant once both are typed, else empty the field
    const write = (): void => {
        properties.held.set(
            day === undefined || time === undefined ? clearedOf(field) : instantOf(day, time),
        );
    };

    return (
        <Labelled field={field} held={properties.held}>
            <MomentGroup>
                <DatePicker
                    aria-label={locale.render(t`Date`)}
                    {...(start === undefined ? {} : { defaultValue: start.day })}
                    onValueChange={(next) => {
                        day = next;
                        write();
                    }}
                />
                <TimeField
                    aria-label={locale.render(t`Time`)}
                    {...(start === undefined ? {} : { defaultValue: start.time })}
                    onValueChange={(next) => {
                        time = next;
                        write();
                    }}
                />
            </MomentGroup>
        </Labelled>
    );
}

/** Render a group of a date and a time, named by its field's label, its controls apart from the field. */
function MomentGroup(properties: { readonly children: JSX.Element }): JSX.Element {
    const field = useFieldControl();
    field?.group();

    return (
        <div
            role="group"
            data-slot="schema-form-moment"
            {...field?.groupAttributes()}
            {...style.attrs(styles.moment)}
        >
            <FieldContext value={null}>{properties.children}</FieldContext>
        </div>
    );
}

/** Render a number input within its field's bounds. */
function NumberControl(properties: FieldControlProperties<SchemaNumberField>): JSX.Element {
    const field = properties.field;

    return (
        <Labelled field={field} held={properties.held}>
            <Input
                type="number"
                inputmode={field.step === 1 ? "numeric" : "decimal"}
                min={field.minimum}
                max={field.maximum}
                step={field.step}
                aria-required={field.isRequired ? "true" : undefined}
                value={textOf(properties.held.value())}
                onInput={(event) => properties.held.set(valueOf(field, event.currentTarget.value))}
            />
        </Labelled>
    );
}

/** Render a checkbox labelled beside it. */
function BooleanControl(properties: FieldControlProperties): JSX.Element {
    const field = properties.field;

    return (
        <Field orientation="horizontal" invalid={properties.held.problem() !== undefined}>
            <Checkbox
                checked={properties.held.value() === true}
                onCheckedChange={(isChecked) => properties.held.set(isChecked)}
            />
            <FieldLabel>{field.title}</FieldLabel>
            <Show when={field.description}>
                {(description) => <FieldDescription>{description()}</FieldDescription>}
            </Show>
            <FieldError>{properties.held.problem()}</FieldError>
        </Field>
    );
}

/** Render a select of one value of an enumeration. */
function ChoiceControl(properties: FieldControlProperties<SchemaChoiceField>): JSX.Element {
    const locale = useLocale();
    const field = properties.field;

    return (
        <Labelled field={field} held={properties.held}>
            <Select
                placeholder={locale.render(t`Choose`)}
                aria-required={field.isRequired ? "true" : undefined}
                value={choiceOf(field, properties.held.value())}
                onValueChange={(index) =>
                    properties.held.set(
                        index === undefined ? undefined : field.choices[Number(index)]?.value,
                    )
                }
            >
                <For each={field.choices}>
                    {(choice, index) => (
                        <SelectItem value={String(index())}>{choice.label}</SelectItem>
                    )}
                </For>
            </Select>
        </Labelled>
    );
}

/** Render a checkbox per value of an enumeration a list holds several of. */
function ChoicesControl(properties: FieldControlProperties<SchemaChoiceField>): JSX.Element {
    // follow the chosen values in the list
    const field = properties.field;
    const chosen = (): readonly JsonValue[] => listOf(properties.held.value());
    const isChosen = (value: JsonValue): boolean =>
        chosen().some((entry) => JSON.stringify(entry) === JSON.stringify(value));

    return (
        <Grouped field={field} held={properties.held}>
            <For each={field.choices}>
                {(choice) => (
                    <Field orientation="horizontal">
                        <Checkbox
                            checked={isChosen(choice.value)}
                            onCheckedChange={(isChecked) =>
                                properties.held.set(
                                    // keep the schema's order of the chosen values
                                    field.choices
                                        .filter((entry) =>
                                            entry === choice ? isChecked : isChosen(entry.value),
                                        )
                                        .map((entry) => entry.value),
                                )
                            }
                        />
                        <FieldLabel>{choice.label}</FieldLabel>
                    </Field>
                )}
            </For>
        </Grouped>
    );
}

/** Render a fieldset of an object's fields, expanding an optional object once it exists, with buttons adding and removing it. */
function ObjectControl(properties: FieldControlProperties<SchemaObjectField>): JSX.Element {
    // expand a required object, and an optional one while it holds a value
    const locale = useLocale();
    const field = properties.field;
    const isShown = (): boolean =>
        field.isRequired || objectOf(properties.held.value()) !== undefined;

    return (
        <Grouped field={field} held={properties.held}>
            <Show
                when={isShown()}
                fallback={
                    <div>
                        <Button
                            type="button"
                            variant="outline"
                            onClick={() => properties.held.set(emptyOf(field.fields))}
                        >
                            {locale.render(t`Add ${field.title}`)}
                        </Button>
                    </div>
                }
            >
                <Fields {...properties} />
                <Show when={!field.isRequired}>
                    <div>
                        <Button
                            type="button"
                            variant="outline"
                            onClick={() => properties.held.set(undefined)}
                        >
                            {locale.render(t`Remove ${field.title}`)}
                        </Button>
                    </div>
                </Show>
            </Show>
        </Grouped>
    );
}

/** Render the fields of an object, each editing its property inside the object. */
function Fields(properties: FieldControlProperties<SchemaObjectField>): JSX.Element {
    const object = (): JsonObject =>
        objectOf(properties.held.value()) ?? emptyOf(properties.field.fields);

    return (
        <For each={properties.field.fields}>
            {(inner) => {
                const pointer = pointerOf(properties.pointer, inner.name);

                return properties.render(
                    inner,
                    {
                        value: () => object()[inner.name],
                        set: (value) => properties.held.set(withEntry(object(), inner.name, value)),
                        problem: () => properties.problemAt(pointer),
                    },
                    pointer,
                );
            }}
        </For>
    );
}

/** Render a fieldset of a list's items, each with a button removing it, and a button adding one. */
function ArrayControl(properties: FieldControlProperties<SchemaArrayField>): JSX.Element {
    // follow the items, each titled by its position
    const locale = useLocale();
    const field = properties.field;
    const items = (): readonly JsonValue[] => listOf(properties.held.value());
    const itemOf = (index: number): SchemaField => ({
        ...field.item,
        name: String(index),
        title: `${field.title} ${String(index + 1)}`,
    });

    return (
        <Grouped field={field} held={properties.held}>
            {/* Items, one row per position */}
            <Repeat count={items().length}>
                {(index) => {
                    const pointer = pointerOf(properties.pointer, String(index));

                    return (
                        <div data-slot="schema-form-item" {...style.attrs(styles.row)}>
                            <div {...style.attrs(styles.item)}>
                                {properties.render(
                                    itemOf(index),
                                    {
                                        value: () => items()[index],
                                        set: (value) =>
                                            properties.held.set(items().with(index, value ?? null)),
                                        problem: () => properties.problemAt(pointer),
                                    },
                                    pointer,
                                )}
                            </div>
                            <Button
                                type="button"
                                variant="outline"
                                aria-label={locale.render(
                                    t`Remove ${field.title} ${String(index + 1)}`,
                                )}
                                onClick={() => properties.held.set(items().toSpliced(index, 1))}
                            >
                                {locale.render(t`Remove`)}
                            </Button>
                        </div>
                    );
                }}
            </Repeat>

            {/* Add */}
            <div>
                <Button
                    type="button"
                    variant="outline"
                    onClick={() => properties.held.set([...items(), field.item.start ?? null])}
                >
                    {locale.render(t`Add ${field.title}`)}
                </Button>
            </div>
        </Grouped>
    );
}

/** Render a select of an object's kind, then the chosen kind's fields. */
function VariantControl(properties: FieldControlProperties<SchemaVariantField>): JSX.Element {
    // follow the kind the value holds, else the one last chosen
    const locale = useLocale();
    const field = properties.field;
    const [chosen, setChosen] = createSignal(
        untrack(() => variantOf(field, properties.held.value())),
    );
    const current = (): number | undefined => variantOf(field, properties.held.value()) ?? chosen();

    return (
        <Grouped field={field} held={properties.held}>
            <Field>
                <FieldLabel>{locale.render(t`Kind`)}</FieldLabel>
                <Select
                    placeholder={locale.render(t`Choose`)}
                    value={current() === undefined ? undefined : String(current())}
                    onValueChange={(index) => {
                        // start the chosen kind from its own starting fields
                        const variant = index === undefined ? undefined : Number(index);
                        setChosen(variant);
                        const fields = variant === undefined ? [] : field.variants[variant]?.fields;
                        properties.held.set(
                            variant === undefined ? undefined : emptyOf(fields ?? []),
                        );
                    }}
                >
                    <For each={field.variants}>
                        {(variant, index) => (
                            <SelectItem value={String(index())}>{variant.title}</SelectItem>
                        )}
                    </For>
                </Select>
            </Field>
            <Show when={current() === undefined ? undefined : field.variants[current() ?? 0]}>
                {(variant) => <Fields {...properties} field={variant()} />}
            </Show>
        </Grouped>
    );
}

/** The properties of a labelled field around one control. */
interface LabelledProperties {
    /** The field. */
    readonly field: SchemaField;
    /** Its value and refusal. */
    readonly held: SchemaHeld;
    /** The control. */
    readonly children: JSX.Element;
}

/** Render a field with its label, control, help and refusal. */
function Labelled(properties: LabelledProperties): JSX.Element {
    return (
        <Field invalid={properties.held.problem() !== undefined}>
            <FieldLabel>{properties.field.title}</FieldLabel>
            {properties.children}
            <Show when={properties.field.description}>
                {(description) => <FieldDescription>{description()}</FieldDescription>}
            </Show>
            <FieldError>{properties.held.problem()}</FieldError>
        </Field>
    );
}

/** Render a fieldset with its legend, help, contents and refusal. */
function Grouped(properties: LabelledProperties): JSX.Element {
    return (
        <FieldSet>
            <FieldLegend variant="label">{properties.field.title}</FieldLegend>
            <Show when={properties.field.description}>
                {(description) => <FieldDescription>{description()}</FieldDescription>}
            </Show>
            <FieldGroup>{properties.children}</FieldGroup>
            <FieldError>{properties.held.problem()}</FieldError>
        </FieldSet>
    );
}

/** Write a pointer to a property or item inside the value at a pointer, as RFC 6901 escapes it. */
function pointerOf(parent: string, key: string): string {
    return `${parent}/${key.replaceAll("~", "~0").replaceAll("/", "~1")}`;
}

/** Replace one entry of an object, dropping it for an absent value. */
function withEntry(object: JsonObject, key: string, value: JsonValue | undefined): JsonObject {
    const { [key]: _dropped, ...rest } = object;

    return value === undefined ? rest : { ...rest, [key]: value };
}

/** Read the index of the choice a field holds, absent for none. */
function choiceOf(field: SchemaChoiceField, value: JsonValue | undefined): string | undefined {
    const index = field.choices.findIndex(
        (choice) => JSON.stringify(choice.value) === JSON.stringify(value),
    );

    return index === -1 ? undefined : String(index);
}

/** Write a field's value as its control's text: JSON, or the text. */
function textOf(value: JsonValue | undefined): string {
    // show nothing for an absent or null value
    if (value === undefined || value === null) {
        return "";
    }
    // show a string as it is and anything else as JSON
    else if (typeof value === "string") {
        return value;
    }

    return JSON.stringify(value);
}

/** Read a control's text as its field's value: a number, JSON or the text, null or absent while empty. */
function valueOf(field: SchemaTextField | SchemaNumberField, text: string): JsonValue | undefined {
    // leave an empty control null when the field takes null, else unset
    if (text === "") {
        return clearedOf(field);
    }
    // read a number, or JSON where it parses
    else if (field.control === "number") {
        return Number(text);
    } else if (field.control === "json") {
        const parsed = schema.json().safeParse(parsedJson(text));

        return parsed.success ? parsed.data : text;
    }

    return text;
}

/** Read the value of an emptied control: null when the field takes null, else unset. */
function clearedOf(field: SchemaField): null | undefined {
    return field.isNullable ? null : undefined;
}

/** Read a `YYYY-MM-DD` date, undefined for anything else. */
function dayOf(value: JsonValue | undefined): PlainDate | undefined {
    return typeof value === "string" && DATE_TEXT.test(value) ? Day.parse(value) : undefined;
}

/** Read the time of day an RFC 3339 full time shows on its own clock, undefined for anything else. */
function wallTimeOf(value: JsonValue | undefined): PlainTime | undefined {
    const parts = typeof value === "string" ? TIME_TEXT.exec(value) : null;

    return parts === null ? undefined : `${parts[1] ?? "00"}:${parts[2] ?? "00"}`;
}

/** Write a time of day as an RFC 3339 full time at the person's current offset from UTC. */
function fullTimeOf(time: PlainTime): string {
    // write the offset as the sign, hours and minutes east of UTC
    const east = -new Date().getTimezoneOffset();
    const sign = east < 0 ? "-" : "+";
    const offset = `${sign}${pad(Math.floor(Math.abs(east) / HOUR_MINUTES))}:${pad(Math.abs(east) % HOUR_MINUTES)}`;

    return `${time}:00${offset}`;
}

/** Read the local date and time of day of an instant, undefined for anything else. */
function localMomentOf(
    value: JsonValue | undefined,
): { readonly day: PlainDate; readonly time: PlainTime } | undefined {
    // read the instant
    const instant = new Date(typeof value === "string" ? value : "");
    if (Number.isNaN(instant.getTime())) {
        return undefined;
    }

    // split it on the local clock
    const day = {
        year: instant.getFullYear(),
        month: instant.getMonth() + 1,
        day: instant.getDate(),
    };

    return { day, time: `${pad(instant.getHours())}:${pad(instant.getMinutes())}` };
}

/** Write the instant of a local date and time of day as RFC 3339 in UTC. */
function instantOf(day: PlainDate, time: PlainTime): string {
    // set the local date and time, keeping years below 100 as they are
    const instant = new Date(0);
    const minutes = PlainTime.minutes(time);
    instant.setFullYear(day.year, day.month - 1, day.day);
    instant.setHours(Math.floor(minutes / HOUR_MINUTES), minutes % HOUR_MINUTES, 0, 0);

    return instant.toISOString();
}

/** Write a part of a date or time in two digits. */
function pad(part: number): string {
    return String(part).padStart(2, "0");
}

/** Parse JSON text, absent where it is no JSON. */
function parsedJson(text: string): unknown {
    try {
        return JSON.parse(text);
    } catch {
        return undefined;
    }
}

/** Drop the absent entries of objects at every depth, as JSON holds none. */
function pruned(value: SchemaValues): JsonObject {
    return Object.fromEntries(
        Object.entries(value).flatMap(([key, entry]) =>
            entry === undefined ? [] : [[key, prunedValue(entry)]],
        ),
    );
}

/** Drop the absent entries of the objects inside a value. */
function prunedValue(value: JsonValue): JsonValue {
    // descend into lists and objects
    const object = objectOf(value);
    if (typeof value === "object" && value !== null && object === undefined) {
        return listOf(value).map(prunedValue);
    } else if (object !== undefined) {
        return pruned(object);
    }

    return value;
}
