import type { Message } from "@destack/locale";
import type { ActionValue, FieldChoice } from "@destack/object";
import { type JsonObject, JsonValue, type schema, toJsonSchema } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, useLocale } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { FieldContext, useFieldControl } from "../field/control.ts";
import { ValueField } from "./value-field.tsx";

/** The layouts of a value's parts: a column, a row that wraps, or a grid of two. */
export const valueStyles = style.create({
    group: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    row: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
    grid: {
        display: "grid",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        gap: space[2],
    },
});

/** The properties of the group of a value's parts. */
export interface ValueGroupProperties {
    /** What the value is called, where no field's label says so. */
    readonly "aria-label"?: string | undefined;
    /** The id of the element describing the value, none without one. */
    readonly "aria-describedby"?: string | undefined;
    /** The group's look, such as a row or a grid. */
    readonly xstyle?: style.Styles;
    /** The parts, each named on its own. */
    readonly children: JSX.Element;
}

/** Render the group of a value's parts, labelled by its field, each part named on its own. */
export function ValueGroup(properties: ValueGroupProperties): JSX.Element {
    // label the group by the nearest field, as a field labels a group of controls
    const field = useFieldControl();
    field?.group();

    return (
        <div
            role="group"
            {...field?.groupAttributes()}
            aria-label={properties["aria-label"]}
            aria-describedby={
                properties["aria-describedby"] ?? field?.groupAttributes()["aria-describedby"]
            }
            {...style.attrs(properties.xstyle)}
        >
            {/* Parts, outside the field so none takes its id */}
            <FieldContext value={null}>{properties.children}</FieldContext>
        </div>
    );
}

/** Render the button that opens a value's picker, the control its field labels and describes. */
export function ValueTrigger(properties: ButtonProperties): JSX.Element {
    // take the field's id and description, beside the button's own
    const field = useFieldControl();

    return (
        <Button
            variant="outline"
            {...properties}
            id={field?.id ?? properties.id}
            aria-describedby={
                properties["aria-describedby"] ?? field?.attributes()["aria-describedby"]
            }
        />
    );
}

/** Type the values of an enum as choices, each titled and described as given. */
export function choiceType<Key extends string>(
    options: readonly Key[],
    titles: Readonly<Record<Key, { readonly title: Message; readonly description?: Message }>>,
): { readonly kind: "choice"; readonly choices: readonly FieldChoice[] } {
    return { kind: "choice", choices: options.map((value) => ({ value, ...titles[value] })) };
}

/** Describe a value type's schema as the JSON Schema a JSON input carries. */
export function jsonType(definition: schema.Schema): {
    readonly kind: "json";
    readonly schema: JsonObject;
} {
    return { kind: "json", schema: JsonValue.object(toJsonSchema(definition)) };
}

/** Render the field of one part of a value, giving back only what the part's schema takes. */
export function ValuePart<Value extends JsonValue>(properties: {
    /** What the part takes, such as a choice of an enum's values or a named JSON value. */
    readonly type: ActionValue;
    /** The part's schema, which reads what its field gives. */
    readonly schema: schema.Schema<Value>;
    /** What the part is called. */
    readonly label: Message;
    /** The part's value. */
    readonly value: Value;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the part changing to a value its schema takes. */
    readonly onValueChange: (value: Value) => void;
}): JSX.Element {
    // give back what the field gives, where the part's schema reads it
    const locale = useLocale();
    const give = (next: JsonValue | undefined): void => {
        const read = properties.schema.safeParse(next);
        if (read.success) {
            properties.onValueChange(read.data);
        }
    };

    return (
        <ValueField
            type={properties.type}
            aria-label={locale.render(properties.label)}
            required
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={give}
        />
    );
}
