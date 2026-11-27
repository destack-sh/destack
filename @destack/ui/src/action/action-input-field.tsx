import type { ActionInput, Choice } from "@destack/object";
import type { JsonValue } from "@destack/schema";
import { type JSX, useLocale } from "@destack/view";
import type { FieldOrientation } from "../field/index.ts";
import { type MIXED, ValueField } from "../value-field/index.ts";

/** The properties of an input's field. */
export interface ActionInputFieldProperties {
    /** The input, with its name, label, icon, description, value type, presence and default. */
    readonly input: ActionInput;
    /** The choices a person picks it from, none for one they type. */
    readonly choices?: readonly Choice[] | undefined;
    /** Whether the label sits above the field or beside it, above by default. */
    readonly orientation?: FieldOrientation | undefined;
    /** The value given, none to leave the default in force, or mixed across objects. */
    readonly value: JsonValue | undefined | typeof MIXED;
    /** Handle a person giving a value, undefined to give none. */
    readonly onValueChange: (value: JsonValue | undefined) => void;
}

/** Render the field of an action's input, by its label, its value's type and its default. */
export function ActionInputField(properties: ActionInputFieldProperties): JSX.Element {
    // label it by its own label, else by its type's title or its name
    const locale = useLocale();
    const input = (): ActionInput => properties.input;
    const label = (): string => {
        const { label: given, value } = input();

        return given !== undefined
            ? locale.render(given)
            : value.kind === "object"
              ? locale.render(value.type.declared.title, { count: 1 })
              : input().name;
    };
    const description = (): string | undefined => {
        const said = input().description;

        return said === undefined ? undefined : locale.render(said);
    };

    return (
        <ValueField
            label={label()}
            icon={input().icon}
            description={description()}
            orientation={properties.orientation}
            type={input().value}
            value={properties.value}
            inherited={input().default}
            onValueChange={properties.onValueChange}
            choices={properties.choices}
            required={input().presence === "required"}
        />
    );
}
