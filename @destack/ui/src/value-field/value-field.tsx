import { type NumberFormat, t } from "@destack/locale";
import type { IconName } from "@destack/icon/name";
import { type ActionValue, Choice } from "@destack/object";
import { canonicalize, type JsonValue, LocalReference, schema } from "@destack/schema";
import * as style from "@destack/style";
import { color, font } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createSignal,
    Errored,
    flush,
    For,
    Show,
    type JSX,
    omit,
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
import {
    Field,
    FieldAction,
    FieldDescription,
    FieldHeader,
    type FieldOrientation,
    useFieldSize,
} from "../field/field.tsx";
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
import { BINDABLE, Binding } from "./binding.ts";
import { ValueSet } from "./group.tsx";
import {
    ValueHeadingContext,
    ValueLabel,
    ValueMixed,
    ValueReset,
    type ValueState,
} from "./state.tsx";

/** The most choices with icons laid out side by side as segments, past which they go in a menu. */
const SEGMENTS = 4;

/** The most choices with words alone laid out side by side, as more do not fit a field's width. */
const WORD_SEGMENTS = 3;

/** The styles of inputs of values. */
const styles = style.create({
    frame: {
        display: "contents",
    },
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
export interface ValueInputProperties<Type extends ActionValue = ActionValue> {
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
    /** What it is called, shown above or beside it. */
    readonly label?: string | undefined;
    /** What it is called where no label shows, such as a part of another value. */
    readonly "aria-label"?: string | undefined;
    /** The id of the element describing the value, none without one. */
    readonly "aria-describedby"?: string | undefined;
    /** Whether a form needs a value. */
    readonly required?: boolean | undefined;
    /** Whether it takes no input. */
    readonly disabled?: boolean | undefined;
}

/** The inputs of kinds of values. */
export type ValueInputs = {
    readonly [Kind in ValueKind]: (
        properties: ValueInputProperties<ActionValue & { readonly kind: Kind }>,
    ) => JSX.Element;
};

/** The properties of the input of a named JSON value. */
export type NamedValueInputProperties = ValueInputProperties<JsonValueType>;

/** Read a field's value by its schema: none where unset, an error where it has another shape. */
export function readValue<Value>(
    value: JsonValue | undefined,
    definition: schema.Schema<Value>,
): Value | undefined {
    // leave an unset value unset, as its field shows what applies in its place
    if (value === undefined) {
        return undefined;
    }

    // refuse a value of another shape, such as one stored before its schema changed
    const read = definition.safeParse(value);
    if (!read.success) {
        const name = definition.meta()?.title ?? "value";
        throw new TypeError(`a '${name}' has another shape: ${read.error.message}`);
    }

    return read.data;
}

/** The input of a named JSON value, such as a color's or a paint's. */
export type NamedValueInput = (properties: NamedValueInputProperties) => JSX.Element;

/** The inputs an app replaces: by kind, and by the name of a JSON value's schema. */
interface Replacement {
    /** The inputs by kind. */
    readonly inputs: Partial<ValueInputs>;
    /** The inputs of named JSON values, by their schema's name. */
    readonly named: Readonly<Record<string, NamedValueInput>>;
}

/** The inputs an app replaces, none by default. */
const ValueFieldsContext = createContext<Replacement>({ inputs: {}, named: {} });

/** Replace the inputs of kinds of values, and of named JSON values, inside. */
export function ValueFieldProvider(properties: {
    /** The inputs, by the kind of value they take. */
    readonly inputs?: Partial<ValueInputs>;
    /** The inputs of named JSON values, by their schema's name, such as `paint`. */
    readonly named?: Readonly<Record<string, NamedValueInput>>;
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

/** The mark of a value that several objects edited at once hold differently. */
export const MIXED: unique symbol = Symbol("mixed");

/** Read the value several objects share: the one all hold, none where none does, else mixed. */
export function sharedValue(
    values: readonly (JsonValue | undefined)[],
): JsonValue | undefined | typeof MIXED {
    // compare each value's canonical JSON with the first's
    const keys = values.map((value) => (value === undefined ? "" : canonicalize(value)));

    return keys.every((key) => key === keys[0]) ? values[0] : MIXED;
}

/** The properties of a value's field: its label, its input's, and where its value stands. */
export interface ValueFieldProperties<Type extends ActionValue = ActionValue> extends Omit<
    ValueInputProperties<Type>,
    "value"
> {
    /** The value: one given, none to leave the inherited one in force, or mixed across objects. */
    readonly value: JsonValue | undefined | typeof MIXED;
    /** The value in force while none is given, such as a declared default, none for none. */
    readonly inherited?: JsonValue | undefined;
    /** The icon beside the label, none for none. */
    readonly icon?: IconName | undefined;
    /** One sentence on what it takes, below the field, or on its label in a dense panel. */
    readonly description?: string | undefined;
    /** Whether the label sits above the field or beside it, above by default. */
    readonly orientation?: FieldOrientation | undefined;
}

/** Render a value's field: its label and state, then its input, or a mark of mixed values. */
export function ValueField(properties: ValueFieldProperties): JSX.Element {
    // read where the value stands, and show the value given, else the one in force
    const type = untrack(() => properties.type);
    const isOwnLegend = type.kind === "json" && isSet(type);
    const fieldSize = useFieldSize();
    const state = (): ValueState =>
        properties.value === MIXED ? "mixed" : properties.value === undefined ? "unset" : "set";
    const shown = (): JsonValue | undefined => {
        const { value } = properties;

        return value === MIXED || value === undefined ? properties.inherited : value;
    };
    const name = (): string => properties.label ?? properties["aria-label"] ?? "";

    // edit mixed values from a press on their mark, focus in the input, until a value is given
    const [isEditing, setEditing] = createSignal(false);
    const give = (value: JsonValue | undefined): void => {
        setEditing(false);
        properties.onValueChange(value);
    };
    let frame: HTMLElement | undefined;
    const edit = (): void => {
        setEditing(true);
        flush();
        frame?.querySelector<HTMLElement>("input, select, textarea, button")?.focus();
    };

    // offer to give a given value back to the one in force
    const reset = (
        <Show when={state() !== "unset" && properties.required !== true}>
            <ValueReset
                label={name()}
                disabled={properties.disabled}
                onReset={() => give(undefined)}
            />
        </Show>
    );

    // give the state and reset to the legend of a set, none to a part without a label
    const heading = untrack(() => properties.label) === undefined ? null : { state, reset };

    // show the input, or the mark of mixed values until a person edits them
    const input = (label: string | undefined): JSX.Element => (
        <div ref={(element) => (frame = element)} {...style.attrs(styles.frame)}>
            <Show
                when={state() === "mixed" && !isEditing()}
                fallback={
                    <ValueControl
                        {...omit(properties, "inherited", "icon", "description", "orientation")}
                        label={label}
                        value={shown()}
                        onValueChange={give}
                    />
                }
            >
                <ValueMixed
                    aria-label={properties["aria-label"]}
                    disabled={properties.disabled}
                    onEdit={edit}
                />
            </Show>
        </div>
    );

    return (
        <Show
            when={properties.label !== undefined && !isOwnLegend}
            fallback={
                // a set draws its legend with the state, as a part without a label shows none
                <ValueHeadingContext value={heading}>
                    <Show
                        when={isOwnLegend && state() === "mixed" && !isEditing()}
                        fallback={input(properties.label)}
                    >
                        <ValueSet label={properties.label}>{input(undefined)}</ValueSet>
                    </Show>
                </ValueHeadingContext>
            }
        >
            {/* A control under its label, or beside it */}
            <Field
                data-state={state()}
                orientation={
                    type.kind === "boolean" ? "horizontal" : (properties.orientation ?? "vertical")
                }
            >
                {/* Label, state and reset */}
                <FieldHeader>
                    <ValueLabel
                        label={name()}
                        icon={properties.icon}
                        description={fieldSize === "sm" ? properties.description : undefined}
                        state={state()}
                    />
                    <FieldAction>{reset}</FieldAction>
                </FieldHeader>

                {/* Input, then its description in a roomy form */}
                {input(undefined)}
                <Show when={fieldSize === "default" && properties.description}>
                    {(said) => <FieldDescription>{said()}</FieldDescription>}
                </Show>
            </Field>
        </Show>
    );
}

/** Render the input a value takes, by its kind. */
function ValueControl(properties: ValueInputProperties): JSX.Element {
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
function TextInput(properties: ValueInputProperties): JSX.Element {
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
    properties: ValueInputProperties<ActionValue & { readonly kind: "number" }>,
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
function BooleanInput(properties: ValueInputProperties): JSX.Element {
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

/** What a named input draws: one control a field labels, or a set of parts under its own legend. */
type NamedInputKind = "control" | "set";

/** The kit's input of each named schema, such as a color's, each loaded as a chunk of its own. */
const NAMED_INPUTS: Readonly<
    Record<string, { readonly kind: NamedInputKind; readonly load: () => Promise<NamedValueInput> }>
> = {
    "style.color": { kind: "control", load: async () => (await import("./color.tsx")).ColorInput },
    "style.palette": { kind: "set", load: async () => (await import("./color.tsx")).PaletteInput },
    "style.gradient": {
        kind: "control",
        load: async () => (await import("./paint.tsx")).GradientInput,
    },
    "style.paint": { kind: "control", load: async () => (await import("./paint.tsx")).PaintInput },
    "style.fillStack": {
        kind: "set",
        load: async () => (await import("./paint.tsx")).FillStackInput,
    },
    "style.dimension": {
        kind: "control",
        load: async () => (await import("./dimension.tsx")).DimensionInput,
    },
    "style.cubicBezier": {
        kind: "control",
        load: async () => (await import("./bezier.tsx")).CubicBezierInput,
    },
    "style.fontFamily": {
        kind: "control",
        load: async () => (await import("./font.tsx")).FontFamilyInput,
    },
    "style.fontWeight": {
        kind: "control",
        load: async () => (await import("./font.tsx")).FontWeightInput,
    },
    "style.fontStyle": {
        kind: "control",
        load: async () => (await import("./font.tsx")).FontStyleInput,
    },
    "style.fontFeatures": {
        kind: "control",
        load: async () => (await import("./font.tsx")).FontFeaturesInput,
    },
    "style.fontVariations": {
        kind: "control",
        load: async () => (await import("./font.tsx")).FontVariationsInput,
    },
    "style.corners": { kind: "set", load: async () => (await import("./box.tsx")).CornersInput },
    "style.boxSides": { kind: "set", load: async () => (await import("./box.tsx")).BoxSidesInput },
    "style.size": { kind: "set", load: async () => (await import("./size.tsx")).SizeInput },
    "style.sizing": { kind: "set", load: async () => (await import("./size.tsx")).SizingInput },
    "style.position": {
        kind: "set",
        load: async () => (await import("./position.tsx")).PositionInput,
    },
    "style.opacity": {
        kind: "set",
        load: async () => (await import("./opacity.tsx")).OpacityInput,
    },
    "style.overflow": {
        kind: "control",
        load: async () => (await import("./overflow.tsx")).OverflowInput,
    },
    "style.stroke": { kind: "set", load: async () => (await import("./stroke.tsx")).StrokeInput },
    "style.shadow": { kind: "set", load: async () => (await import("./shadow.tsx")).ShadowInput },
    "style.shadowStack": {
        kind: "set",
        load: async () => (await import("./shadow.tsx")).ShadowStackInput,
    },
    "style.filter": { kind: "set", load: async () => (await import("./filter.tsx")).FilterInput },
    "style.filterStack": {
        kind: "set",
        load: async () => (await import("./filter.tsx")).FilterStackInput,
    },
    "style.typography": {
        kind: "set",
        load: async () => (await import("./typography.tsx")).TypographyInput,
    },
    "style.textFormat": {
        kind: "set",
        load: async () => (await import("./text.tsx")).TextFormatInput,
    },
    "style.textStroke": {
        kind: "set",
        load: async () => (await import("./text.tsx")).TextStrokeInput,
    },
    "style.transform": {
        kind: "set",
        load: async () => (await import("./transform.tsx")).TransformInput,
    },
    "style.layout": { kind: "set", load: async () => (await import("./layout.tsx")).LayoutInput },
    "style.motion": {
        kind: "set",
        load: async () => (await import("./motion.tsx")).MotionInput,
    },
    "style.timingFunction": {
        kind: "set",
        load: async () => (await import("./timing.tsx")).TimingFunctionInput,
    },
    "style.transition": {
        kind: "set",
        load: async () => (await import("./transition.tsx")).TransitionInput,
    },
    [BINDABLE]: {
        kind: "control",
        load: async () => (await import("./bindable.tsx")).BindableInput,
    },
    "schema.duration": {
        kind: "set",
        load: async () => (await import("./duration.tsx")).DurationInput,
    },
    "schema.dateRange": {
        kind: "control",
        load: async () => (await import("./range.tsx")).DateRangeInput,
    },
    "schema.recurrence": {
        kind: "set",
        load: async () => (await import("./recurrence.tsx")).RecurrenceInput,
    },
    "schema.money": { kind: "set", load: async () => (await import("./money.tsx")).MoneyInput },
    "schema.rating": {
        kind: "control",
        load: async () => (await import("./rating.tsx")).RatingInput,
    },
    "schema.url": { kind: "control", load: async () => (await import("./url.tsx")).UrlInput },
};

/** The kit's named inputs loaded so far, by schema name, so later fields of one render at once. */
const LOADED = new Map<string, NamedValueInput>();

/** Render the input of an instant. */
function TimeInput(properties: ValueInputProperties): JSX.Element {
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
function ChoiceInput(properties: ValueInputProperties): JSX.Element {
    // write each choice by its value, from the caller's list or else the type's
    const locale = useLocale();
    const choices = (): readonly Choice[] =>
        properties.choices ??
        (properties.type.kind === "choice"
            ? properties.type.choices.map((each) => Choice.of(each, locale))
            : []);
    const choose = (key: string | undefined): JsonValue | undefined =>
        choices().find((choice) => keyOf(choice.value) === key)?.value;
    const isIconic = (): boolean => choices().every((choice) => choice.icon !== undefined);

    return (
        <Show
            when={choices().length <= (isIconic() ? SEGMENTS : WORD_SEGMENTS)}
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
                        // show the choice's icon alone where every choice has one, its title when pointed at
                        const segment = (
                            attributes: JSX.HTMLAttributes<HTMLButtonElement> = {},
                        ) => (
                            <ToggleGroupItem
                                {...attributes}
                                value={keyOf(choice.value)}
                                aria-label={isIconic() ? choice.title : undefined}
                            >
                                <Show when={choice.icon}>
                                    <ChoiceIcon choice={choice} />
                                </Show>
                                <Show when={!isIconic()}>{choice.title}</Show>
                            </ToggleGroupItem>
                        );
                        const tip = (): string | undefined =>
                            isIconic() ? choice.title : choice.description;

                        return (
                            <Show when={tip()} fallback={segment()}>
                                {(said) => (
                                    <Tooltip>
                                        <TooltipTrigger render={segment} />
                                        <TooltipContent>{said()}</TooltipContent>
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
function FoundInput(properties: ValueInputProperties): JSX.Element {
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
function JsonInput(properties: ValueInputProperties<JsonValueType>): JSX.Element {
    // take an app's input of the schema's name, then the kit's once loaded
    const { named } = useContext(ValueFieldsContext);
    const name = untrack(() => nameOf(properties.type));
    const own = name === null ? undefined : named[name];
    const load = name === null ? undefined : NAMED_INPUTS[name]?.load;
    const loaded = name === null ? undefined : LOADED.get(name);
    if (own !== undefined) {
        const Named = own;

        return (
            <Unreadable label={properties["aria-label"]}>
                <Named {...properties} />
            </Unreadable>
        );
    }
    // show the kit's input at once where it loaded before
    else if (loaded !== undefined) {
        const Named = loaded;

        return (
            <Unreadable label={properties["aria-label"]}>
                <Named {...properties} />
            </Unreadable>
        );
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

                    return (
                        <Unreadable label={properties["aria-label"]}>
                            <Named {...properties} />
                        </Unreadable>
                    );
                }}
            </Show>
        );
    }
    // type JSON as text for any other schema
    else {
        return <JsonText {...properties} />;
    }
}

/** Render an input, or where it cannot read its value, a field saying so in its place. */
function Unreadable(properties: {
    /** What the value is called. */
    readonly label: string | undefined;
    /** The input. */
    readonly children: JSX.Element;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Errored
            fallback={() => (
                <Input
                    aria-label={properties.label}
                    aria-invalid="true"
                    readonly
                    value={locale.render(t`This value cannot be read`)}
                />
            )}
        >
            {properties.children}
        </Errored>
    );
}

/** Tell whether a JSON value's input renders a set of parts under a legend of its own. */
function isSet(type: JsonValueType): boolean {
    const binding = Binding.of(type.schema);

    return binding === undefined
        ? NAMED_INPUTS[LocalReference.root(type.schema) ?? ""]?.kind === "set"
        : Binding.isWhole(binding.type);
}

/** Read the name of a JSON value's input: its schema's, or the name of values bound to tokens. */
function nameOf(type: JsonValueType): string | null {
    return Binding.of(type.schema) === undefined ? LocalReference.root(type.schema) : BINDABLE;
}

/** Load every named input of the kit, so a field of any named schema renders at once. */
export async function preloadNamedInputs(): Promise<void> {
    await Promise.all(
        Object.entries(NAMED_INPUTS).map(([name, input]) => loadNamed(name, input.load)),
    );
}

/** Load the kit's input of a schema name, keeping it for the fields after. */
async function loadNamed(
    name: string,
    load: () => Promise<NamedValueInput>,
): Promise<NamedValueInput> {
    const input = await load();
    LOADED.set(name, input);

    return input;
}

/** Render JSON as text, kept while it is typed and given once it parses. */
function JsonText(properties: ValueInputProperties): JSX.Element {
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
function valueOf(properties: ValueInputProperties, key: string | undefined): JsonValue | undefined {
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
const DEFAULTS: ValueInputs = {
    text: TextInput,
    number: NumberInput,
    boolean: BooleanInput,
    time: TimeInput,
    choice: ChoiceInput,
    person: FoundInput,
    object: FoundInput,
    json: JsonInput,
};
