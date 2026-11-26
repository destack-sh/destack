import { type NumberFormat, t } from "@destack/locale";
import type { ActionValue, Choice } from "@destack/object";
import { type JsonValue, schema } from "@destack/schema";
import * as style from "@destack/style";
import { color, font } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createSignal,
    For,
    Show,
    type JSX,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { ChoiceIcon } from "../action/choice-icon.tsx";
import {
    Combobox,
    ComboboxContent,
    ComboboxEmpty,
    ComboboxInput,
    ComboboxItem,
} from "../combobox/index.ts";
import { DateTimeField } from "../date-time-field/index.ts";
import { Input } from "../input/index.ts";
import { NumberField, NumberFieldInput, NumberFieldScrubArea } from "../number-field/index.ts";
import { InputGroup, InputGroupAddon } from "../input-group/index.ts";
import { Icon } from "@destack/ui/icon";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../select/index.ts";
import { Switch } from "../switch/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { Tooltip, TooltipContent, TooltipTrigger } from "../tooltip/index.ts";
import { Textarea } from "../textarea/index.ts";

/** The most choices laid out side by side as segments, past which they go in a menu. */
const SEGMENTS = 4;

/** The styles of inputs of values. */
const styles = style.create({
    control: {
        width: "100%",
    },
    scrub: {
        display: "inline-flex",
        alignItems: "center",
        color: color.mutedForeground,
    },
    code: {
        fontFamily: font.code,
    },
});

/** The kinds of values a person gives. */
export type ValueKind = ActionValue["kind"];

/** The properties of the input of a value. */
export interface ValueFieldProperties<Type extends ActionValue = ActionValue> {
    /** What it takes. */
    readonly type: Type;
    /** The value, undefined while none is given. */
    readonly value: JsonValue | undefined;
    /** Handle the value changing, undefined for none. */
    readonly onValueChange: (value: JsonValue | undefined) => void;
    /** The choices a person picks from, none for what they type. */
    readonly choices?: readonly Choice[] | undefined;
    /** Find the choices of what a person types, which are then not narrowed here. */
    readonly onSearch?: ((text: string) => void) | undefined;
    /** What it is called, where no label beside it says so. */
    readonly "aria-label"?: string | undefined;
    /** The id of the element describing the value, none without one. */
    readonly "aria-describedby"?: string | undefined;
    /** Whether a form needs a value. */
    readonly required?: boolean | undefined;
    /** Whether it takes no input. */
    readonly disabled?: boolean | undefined;
}

/** The inputs of kinds of values. */
export type ValueFields = {
    readonly [Kind in ValueKind]: (
        properties: ValueFieldProperties<ActionValue & { readonly kind: Kind }>,
    ) => JSX.Element;
};

/** The inputs an app replaces, none by default. */
const ValueFieldsContext = createContext<Partial<ValueFields>>({});

/** Replace the inputs of kinds of values inside. */
export function ValueFieldProvider(properties: {
    /** The inputs, by the kind of value they take. */
    readonly inputs: Partial<ValueFields>;
    /** What uses them. */
    readonly children?: JSX.Element;
}): JSX.Element {
    // keep those replaced further out
    const outer = useContext(ValueFieldsContext);

    return (
        <ValueFieldsContext value={{ ...outer, ...properties.inputs }}>
            {properties.children}
        </ValueFieldsContext>
    );
}

/** Render the input a value takes, by its kind. */
export function ValueField(properties: ValueFieldProperties): JSX.Element {
    // take an app's input before the kit's
    const replaced = useContext(ValueFieldsContext);
    const type = untrack(() => properties.type);
    const inputs = { ...DEFAULTS, ...replaced };

    // choose among what an app lists, whatever kind it is of
    const isListed = untrack(() => (properties.choices ?? []).length > 0);
    if (isListed && replaced[type.kind] === undefined && !FOUND.has(type.kind)) {
        return <ChoiceInput {...properties} />;
    }

    // narrow what it takes to its kind
    switch (type.kind) {
        case "text":
            return <inputs.text {...properties} type={{ kind: "text" }} />;
        case "number":
            return <inputs.number {...properties} type={type} />;
        case "boolean":
            return <inputs.boolean {...properties} type={{ kind: "boolean" }} />;
        case "time":
            return <inputs.time {...properties} type={{ kind: "time" }} />;
        case "choice":
            return <inputs.choice {...properties} type={type} />;
        case "person":
            return <inputs.person {...properties} type={{ kind: "person" }} />;
        case "object":
            return <inputs.object {...properties} type={type} />;
        case "json":
            return <inputs.json {...properties} type={{ kind: "json" }} />;
    }
}

/** Render the input of a text. */
function TextInput(properties: ValueFieldProperties): JSX.Element {
    return (
        <Input
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            required={properties.required}
            disabled={properties.disabled}
            value={typeof properties.value === "string" ? properties.value : ""}
            onInput={({ currentTarget }) =>
                properties.onValueChange(
                    currentTarget.value === "" ? undefined : currentTarget.value,
                )
            }
        />
    );
}

/** Render the input of a number, typed or scrubbed by dragging its leading handle sideways. */
function NumberInput(
    properties: ValueFieldProperties<ActionValue & { readonly kind: "number" }>,
): JSX.Element {
    return (
        <NumberField
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            value={typeof properties.value === "number" ? properties.value : undefined}
            min={properties.type.minimum}
            max={properties.type.maximum}
            step={properties.type.step}
            smallStep={properties.type.step}
            format={formatOf(properties.type)}
            required={properties.required}
            disabled={properties.disabled}
            onValueChange={(changed) => properties.onValueChange(changed)}
        >
            {/* Handle, then the typed number */}
            <InputGroup>
                <InputGroupAddon>
                    <NumberFieldScrubArea xstyle={styles.scrub}>
                        <Icon name="arrows-left-right" />
                    </NumberFieldScrubArea>
                </InputGroupAddon>
                <NumberFieldInput />
            </InputGroup>
        </NumberField>
    );
}

/** Render the input of a yes or no. */
function BooleanInput(properties: ValueFieldProperties): JSX.Element {
    return (
        <Switch
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            checked={properties.value === true}
            disabled={properties.disabled}
            onCheckedChange={(isOn) => properties.onValueChange(isOn)}
        />
    );
}

/** Render the input of an instant. */
function TimeInput(properties: ValueFieldProperties): JSX.Element {
    return (
        <DateTimeField
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            value={typeof properties.value === "number" ? properties.value : undefined}
            required={properties.required}
            disabled={properties.disabled}
            onValueChange={(value) => properties.onValueChange(value)}
        />
    );
}

/** Render the input of a choice, as a list. */
function ChoiceInput(properties: ValueFieldProperties): JSX.Element {
    // write each choice by its value
    const locale = useLocale();
    const choices = () => properties.choices ?? [];

    return (
        <Show
            when={choices().length <= SEGMENTS}
            fallback={
                <Select
                    xstyle={styles.control}
                    aria-label={properties["aria-label"]}
                    aria-describedby={properties["aria-describedby"]}
                    required={properties.required}
                    disabled={properties.disabled}
                    value={keyOf(properties.value)}
                    onValueChange={(key) => properties.onValueChange(valueOf(properties, key))}
                >
                    <SelectTrigger>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value="">
                            {locale.render(t.context("option")`Choose…`)}
                        </SelectItem>
                        <For each={choices()}>
                            {(choice) => (
                                <SelectItem value={keyOf(choice.value)}>{choice.title}</SelectItem>
                            )}
                        </For>
                    </SelectContent>
                </Select>
            }
        >
            {/* Segments, for a few choices side by side */}
            <ToggleGroup
                variant="segmented"
                size="sm"
                xstyle={styles.control}
                aria-label={properties["aria-label"]}
                aria-describedby={properties["aria-describedby"]}
                disabled={properties.disabled === true}
                disallowEmptySelection={properties.required === true}
                value={keyOf(properties.value)}
                onValueChange={(key) => properties.onValueChange(valueOf(properties, key))}
            >
                <For each={choices()}>
                    {(choice) => {
                        // show the choice's icon and title, and its description when pointed at
                        const segment = (
                            attributes: JSX.HTMLAttributes<HTMLButtonElement> = {},
                        ) => (
                            <ToggleGroupItem {...attributes} value={keyOf(choice.value)}>
                                <Show when={choice.icon}>
                                    <ChoiceIcon choice={choice} />
                                </Show>
                                {choice.title}
                            </ToggleGroupItem>
                        );

                        return (
                            <Show when={choice.description} fallback={segment()}>
                                {(description) => (
                                    <Tooltip>
                                        <TooltipTrigger render={segment} />
                                        <TooltipContent>{description()}</TooltipContent>
                                    </Tooltip>
                                )}
                            </Show>
                        );
                    }}
                </For>
            </ToggleGroup>
        </Show>
    );
}

/** Render the input of a person or an object, found by typing. */
function FoundInput(properties: ValueFieldProperties): JSX.Element {
    const locale = useLocale();

    return (
        <Combobox
            value={properties.value === undefined ? undefined : keyOf(properties.value)}
            onValueChange={(key: string | undefined) =>
                properties.onValueChange(valueOf(properties, key))
            }
            {...(properties.onSearch === undefined ? {} : { filter: null })}
            onInputValueChange={(text) => properties.onSearch?.(text)}
        >
            <ComboboxInput
                aria-label={properties["aria-label"]}
                aria-describedby={properties["aria-describedby"]}
                required={properties.required}
                disabled={properties.disabled}
                placeholder={locale.render(t.context("label")`Choose…`)}
            />
            <ComboboxContent>
                <ComboboxEmpty>{locale.render(t.context("title")`No results.`)}</ComboboxEmpty>
                <For each={properties.choices ?? []}>
                    {(choice) => (
                        <ComboboxItem value={keyOf(choice.value)} textValue={choice.title}>
                            <ChoiceIcon choice={choice} />
                            {choice.title}
                        </ComboboxItem>
                    )}
                </For>
            </ComboboxContent>
        </Combobox>
    );
}

/** Render the input of a value without a form of its own, written as JSON. */
function JsonInput(properties: ValueFieldProperties): JSX.Element {
    // show the text typed while it is of the value, else the value's own
    const [typed, setTyped] = createSignal<string | undefined>(undefined, { ownedWrite: true });
    const shown = (): string => {
        const text = typed();
        const { value } = properties;

        return text !== undefined && keyOf(readJson(text).value) === keyOf(value)
            ? text
            : value === undefined
              ? ""
              : JSON.stringify(value, null, 2);
    };
    const type = ({ currentTarget }: { readonly currentTarget: HTMLTextAreaElement }): void => {
        // mark text that is no JSON, and give nothing for it
        const { value } = currentTarget;
        const read = readJson(value);
        setTyped(value);
        currentTarget.setCustomValidity(read.error);
        properties.onValueChange(read.value);
    };

    return (
        <Textarea
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            required={properties.required}
            disabled={properties.disabled}
            spellcheck="false"
            xstyle={styles.code}
            value={shown()}
            onInput={type}
        />
    );
}

/** Read how a number is written, whole ones without a decimal separator. */
function formatOf({ format, step }: ActionValue & { readonly kind: "number" }): NumberFormat {
    return {
        ...(step === 1 && format?.style === undefined ? { maximumFractionDigits: 0 } : {}),
        ...format,
    };
}

/** Write a value as the key of its option, empty for none. */
function keyOf(value: JsonValue | undefined): string {
    return value === undefined ? "" : JSON.stringify(value);
}

/** Read the value of the choice a key is of, undefined for none. */
function valueOf(properties: ValueFieldProperties, key: string | undefined): JsonValue | undefined {
    return properties.choices?.find((choice) => keyOf(choice.value) === key)?.value;
}

/** Read text as JSON, with what is wrong about text that is none. */
function readJson(text: string): {
    readonly value: JsonValue | undefined;
    readonly error: string;
} {
    // take empty text as none
    if (text.trim() === "") {
        return { value: undefined, error: "" };
    }
    try {
        return { value: schema.json().parse(JSON.parse(text)), error: "" };
    } catch (error) {
        return { value: undefined, error: error instanceof Error ? error.message : String(error) };
    }
}

/** The kinds a person finds by typing, however many there are. */
const FOUND: ReadonlySet<ValueKind> = new Set<ValueKind>(["person", "object"]);

/** The inputs of kinds of values, unless an app replaces them. */
const DEFAULTS: ValueFields = {
    text: TextInput,
    number: NumberInput,
    boolean: BooleanInput,
    time: TimeInput,
    choice: ChoiceInput,
    person: FoundInput,
    object: FoundInput,
    json: JsonInput,
};
