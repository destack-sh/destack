import { type NumberFormat, t } from "@destack/locale";
import { type ActionValue, Choice } from "@destack/object";
import { type JsonValue, LocalReference, schema } from "@destack/schema";
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
import { Load } from "../collection/index.ts";

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

/** What a JSON input takes: JSON its schema takes. */
type JsonValueType = ActionValue & { readonly kind: "json" };

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

/** The properties of the input of a named JSON value. */
export type NamedValueFieldProperties = ValueFieldProperties<JsonValueType>;

/** The input of a named JSON value, such as a color's or a paint's. */
export type NamedValueField = (properties: NamedValueFieldProperties) => JSX.Element;

/** The inputs an app replaces: by kind, and by the name of a JSON value's schema. */
interface Replacement {
    /** The inputs by kind. */
    readonly inputs: Partial<ValueFields>;
    /** The inputs of named JSON values, by their schema's name. */
    readonly named: Readonly<Record<string, NamedValueField>>;
}

/** The inputs an app replaces, none by default. */
const ValueFieldsContext = createContext<Replacement>({ inputs: {}, named: {} });

/** Replace the inputs of kinds of values, and of named JSON values, inside. */
export function ValueFieldProvider(properties: {
    /** The inputs, by the kind of value they take. */
    readonly inputs?: Partial<ValueFields>;
    /** The inputs of named JSON values, by their schema's name, such as `paint`. */
    readonly named?: Readonly<Record<string, NamedValueField>>;
    /** What uses them. */
    readonly children?: JSX.Element;
}): JSX.Element {
    // keep those replaced further out
    const outer = useContext(ValueFieldsContext);
    const replaced = (): Replacement => ({
        inputs: { ...outer.inputs, ...properties.inputs },
        named: { ...outer.named, ...properties.named },
    });

    return <ValueFieldsContext value={replaced()}>{properties.children}</ValueFieldsContext>;
}

/** Render the input a value takes, by its kind. */
export function ValueField(properties: ValueFieldProperties): JSX.Element {
    // take an app's input before the kit's
    const { inputs: replaced } = useContext(ValueFieldsContext);
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
            return <inputs.json {...properties} type={type} />;
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

/** The kit's input of each named schema, such as a color's, each loaded as a chunk of its own. */
const NAMED_INPUTS: Readonly<Record<string, () => Promise<NamedValueField>>> = {
    "style.color": async () => (await import("./color.tsx")).ColorInput,
    "style.palette": async () => (await import("./color.tsx")).PaletteInput,
    "style.gradient": async () => (await import("./paint.tsx")).GradientInput,
    "style.paint": async () => (await import("./paint.tsx")).PaintInput,
    "style.fillStack": async () => (await import("./paint.tsx")).FillStackInput,
    "style.dimension": async () => (await import("./dimension.tsx")).DimensionInput,
    "style.cubicBezier": async () => (await import("./bezier.tsx")).CubicBezierInput,
    "style.fontFamily": async () => (await import("./font.tsx")).FontFamilyInput,
    "style.fontWeight": async () => (await import("./font.tsx")).FontWeightInput,
    "style.fontStyle": async () => (await import("./font.tsx")).FontStyleInput,
    "style.fontFeatures": async () => (await import("./font.tsx")).FontFeaturesInput,
    "style.fontVariations": async () => (await import("./font.tsx")).FontVariationsInput,
    "style.corners": async () => (await import("./box.tsx")).CornersInput,
    "style.boxSides": async () => (await import("./box.tsx")).BoxSidesInput,
    "style.size": async () => (await import("./size.tsx")).SizeInput,
    "style.position": async () => (await import("./position.tsx")).PositionInput,
    "style.opacity": async () => (await import("./opacity.tsx")).OpacityInput,
    "style.overflow": async () => (await import("./overflow.tsx")).OverflowInput,
    "style.stroke": async () => (await import("./stroke.tsx")).StrokeInput,
    "style.shadow": async () => (await import("./shadow.tsx")).ShadowInput,
    "style.shadowStack": async () => (await import("./shadow.tsx")).ShadowStackInput,
    "style.filter": async () => (await import("./filter.tsx")).FilterInput,
    "style.filterStack": async () => (await import("./filter.tsx")).FilterStackInput,
    "style.typography": async () => (await import("./typography.tsx")).TypographyInput,
    "style.textFormat": async () => (await import("./text.tsx")).TextFormatInput,
    "style.textStroke": async () => (await import("./text.tsx")).TextStrokeInput,
    "style.transform": async () => (await import("./transform.tsx")).TransformInput,
    "style.layout": async () => (await import("./layout.tsx")).LayoutInput,
    "style.transition": async () => (await import("./transition.tsx")).TransitionInput,
    "schema.duration": async () => (await import("./duration.tsx")).DurationInput,
    "schema.dateRange": async () => (await import("./range.tsx")).DateRangeInput,
    "schema.recurrence": async () => (await import("./recurrence.tsx")).RecurrenceInput,
    "schema.money": async () => (await import("./money.tsx")).MoneyInput,
    "schema.rating": async () => (await import("./rating.tsx")).RatingInput,
    "schema.url": async () => (await import("./url.tsx")).UrlInput,
};

/** The kit's named inputs loaded so far, by schema name, so later fields of one render at once. */
const LOADED = new Map<string, NamedValueField>();

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
    // write each choice by its value, from the caller's list or else the type's
    const locale = useLocale();
    const choices = (): readonly Choice[] =>
        properties.choices ??
        (properties.type.kind === "choice"
            ? properties.type.choices.map((each) => Choice.of(each, locale))
            : []);
    const choose = (key: string | undefined): JsonValue | undefined =>
        choices().find((choice) => keyOf(choice.value) === key)?.value;

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
                    onValueChange={(key) => properties.onValueChange(choose(key))}
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
                onValueChange={(key) => properties.onValueChange(choose(key))}
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

/** Render the input of JSON: its named schema's own, such as a color's, else the JSON typed. */
function JsonInput(properties: ValueFieldProperties<JsonValueType>): JSX.Element {
    // take an app's input of the schema's name, then the kit's once loaded
    const { named } = useContext(ValueFieldsContext);
    const name = untrack(() => LocalReference.root(properties.type.schema));
    const own = name === null ? undefined : named[name];
    const load = name === null ? undefined : NAMED_INPUTS[name];
    const loaded = name === null ? undefined : LOADED.get(name);
    if (own !== undefined) {
        const Named = own;

        return <Named {...properties} />;
    }
    // show the kit's input at once where it loaded before
    else if (loaded !== undefined) {
        const Named = loaded;

        return <Named {...properties} />;
    }
    // load the kit's input, showing nothing meanwhile, its failure thrown to the error boundary
    else if (name !== null && load !== undefined) {
        const input = new Load(loadNamed(name, load));
        const settled = () => (input.state() === "loading" ? undefined : input);

        return (
            <Show when={settled()}>
                {(ready) => {
                    // take the input once, as it arrives once
                    const Named = untrack(() => ready().value());

                    return <Named {...properties} />;
                }}
            </Show>
        );
    }
    // type JSON as text for any other schema
    else {
        return <JsonText {...properties} />;
    }
}

/** Load every named input of the kit, so a field of any named schema renders at once. */
export async function preloadNamedInputs(): Promise<void> {
    await Promise.all(Object.entries(NAMED_INPUTS).map(([name, load]) => loadNamed(name, load)));
}

/** Load the kit's input of a schema name, keeping it for the fields after. */
async function loadNamed(
    name: string,
    load: () => Promise<NamedValueField>,
): Promise<NamedValueField> {
    const input = await load();
    LOADED.set(name, input);

    return input;
}

/** Render JSON as text, kept while it is typed and given once it parses. */
function JsonText(properties: ValueFieldProperties): JSX.Element {
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
