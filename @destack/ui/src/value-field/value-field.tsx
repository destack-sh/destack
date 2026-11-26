import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { type NumberFormat, t } from "@destack/locale";
import type { ActionValue, Choice } from "@destack/object";
import { type JsonValue, schema } from "@destack/schema";
import * as style from "@destack/style";
import { color, font, radius, space } from "@destack/theme/tokens.stylex";
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
import { Button } from "../button/index.ts";
import {
    ColorArea,
    ColorField,
    ColorPicker,
    ColorSlider,
    ColorSwatch,
} from "../color-picker/index.ts";
import { Input } from "../input/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Sortable, SortableContainer, SortableItem, type SortableMove } from "../sortable/index.ts";
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

/** The color a list of colors adds first, before it has one to repeat. */
const FIRST_COLOR = "#ffffff";

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
    color: {
        justifyContent: "flex-start",
        fontFamily: font.code,
    },
    picker: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    palette: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    blend: (backgroundImage: string) => ({
        height: space[2],
        borderRadius: radius.full,
        backgroundImage,
    }),
    row: {
        display: "flex",
        gap: space[1],
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
        case "color":
            return <inputs.color {...properties} type={{ kind: "color" }} />;
        case "colors":
            return <inputs.colors {...properties} type={type} />;
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

/** Render the input of a color: its swatch and text, which open a picker. */
function ColorInput(properties: ValueFieldProperties): JSX.Element {
    const value = (): string | undefined =>
        typeof properties.value === "string" ? properties.value : undefined;

    return (
        <Popover>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="outline"
                        aria-label={properties["aria-label"]}
                        aria-describedby={properties["aria-describedby"]}
                        disabled={properties.disabled}
                        xstyle={[styles.control, styles.color]}
                    >
                        <ColorSwatch color={value()} />
                        <bdi>{value()}</bdi>
                    </Button>
                )}
            />
            <PopoverContent aria-label={properties["aria-label"]} xstyle={styles.picker}>
                <ColorChooser
                    value={value()}
                    onValueChange={(next) => properties.onValueChange(next)}
                />
            </PopoverContent>
        </Popover>
    );
}

/** Render a palette: swatches a drag reorders, each picked or removed in its popover. */
function ColorsInput(
    properties: ValueFieldProperties<ActionValue & { readonly kind: "colors" }>,
): JSX.Element {
    // key each color by its value and how often it came before, as colors repeat
    const locale = useLocale();
    const colors = (): readonly string[] =>
        schema.array(schema.string()).safeParse(properties.value).data ?? [];
    const entries = (): readonly { readonly key: string; readonly color: string }[] => {
        const seen = new Map<string, number>();

        return colors().map((each) => {
            const count = seen.get(each) ?? 0;
            seen.set(each, count + 1);

            return { key: `${each}#${String(count)}`, color: each };
        });
    };

    // write the colors back changed, within the fewest and the most
    const write = (next: readonly string[]): void => properties.onValueChange([...next]);
    const isFewest = (): boolean => colors().length <= (properties.type.minimum ?? 0);
    const isMost = (): boolean => colors().length >= (properties.type.maximum ?? Infinity);
    const move = ({ id, previous }: SortableMove): void => {
        // place the moved color after the one it lands after
        const rest = entries().filter((entry) => entry.key !== id);
        const moved = entries().filter((entry) => entry.key === id);
        const index =
            previous === undefined ? 0 : rest.findIndex((entry) => entry.key === previous) + 1;
        write(
            [...rest.slice(0, index), ...moved, ...rest.slice(index)].map((entry) => entry.color),
        );
    };

    return (
        <div
            role="group"
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            {...style.attrs(styles.palette)}
        >
            {/* Swatches */}
            <div {...style.attrs(styles.row)}>
                <Sortable onMove={move} indicator="gap">
                    <SortableContainer id="colors" orientation="horizontal" xstyle={styles.row}>
                        <For each={entries()}>
                            {(entry, index) => (
                                <SortableItem
                                    id={entry.key}
                                    activator="item"
                                    sensors={["pointer"]}
                                    disabled={properties.disabled === true}
                                    label={entry.color}
                                >
                                    <Popover>
                                        <PopoverTrigger
                                            render={(attributes) => (
                                                <Button
                                                    {...attributes}
                                                    variant="outline"
                                                    isIconOnly
                                                    disabled={properties.disabled}
                                                    aria-label={locale.render(
                                                        t.context("label")`Color ${index() + 1}`,
                                                    )}
                                                >
                                                    <ColorSwatch color={entry.color} />
                                                </Button>
                                            )}
                                        />
                                        <PopoverContent
                                            aria-label={locale.render(
                                                t.context("label")`Color ${index() + 1}`,
                                            )}
                                            xstyle={styles.picker}
                                        >
                                            <ColorChooser
                                                value={entry.color}
                                                onValueChange={(next) =>
                                                    write(colors().with(index(), next))
                                                }
                                            />
                                            <div {...style.attrs(styles.row)}>
                                                <Button
                                                    variant="ghost"
                                                    isIconOnly
                                                    disabled={index() === 0}
                                                    aria-label={locale.render(
                                                        t.context("action")`Move earlier`,
                                                    )}
                                                    onClick={() =>
                                                        write(swap(colors(), index() - 1))
                                                    }
                                                >
                                                    <Icon
                                                        icon={
                                                            locale.direction === "rtl"
                                                                ? caretRight
                                                                : caretLeft
                                                        }
                                                    />
                                                </Button>
                                                <Button
                                                    variant="ghost"
                                                    isIconOnly
                                                    disabled={index() === colors().length - 1}
                                                    aria-label={locale.render(
                                                        t.context("action")`Move later`,
                                                    )}
                                                    onClick={() => write(swap(colors(), index()))}
                                                >
                                                    <Icon
                                                        icon={
                                                            locale.direction === "rtl"
                                                                ? caretLeft
                                                                : caretRight
                                                        }
                                                    />
                                                </Button>
                                                <Button
                                                    variant="ghost"
                                                    disabled={isFewest()}
                                                    onClick={() =>
                                                        write(colors().toSpliced(index(), 1))
                                                    }
                                                >
                                                    <Icon icon={minusIcon} />
                                                    {locale.render(
                                                        t.context("action")`Remove color`,
                                                    )}
                                                </Button>
                                            </div>
                                        </PopoverContent>
                                    </Popover>
                                </SortableItem>
                            )}
                        </For>
                    </SortableContainer>
                </Sortable>
                <Button
                    variant="ghost"
                    isIconOnly
                    disabled={properties.disabled === true || isMost()}
                    aria-label={locale.render(t.context("action")`Add color`)}
                    onClick={() => write([...colors(), colors().at(-1) ?? FIRST_COLOR])}
                >
                    <Icon icon={plusIcon} />
                </Button>
            </div>

            {/* Blend */}
            <div {...style.attrs(styles.blend(blendOf(colors())))} />
        </div>
    );
}

/** Render a color's area, hue, opacity and text, picking one color. */
function ColorChooser(properties: {
    /** The color, undefined while none is picked. */
    readonly value: string | undefined;
    /** Handle another color being picked. */
    readonly onValueChange: (value: string) => void;
}): JSX.Element {
    return (
        <ColorPicker value={properties.value} onValueChange={properties.onValueChange}>
            <ColorArea />
            <ColorSlider channel="hue" />
            <ColorSlider channel="alpha" />
            <ColorField />
        </ColorPicker>
    );
}

/** Swap a color with the one after it. */
function swap(colors: readonly string[], index: number): readonly string[] {
    return [
        ...colors.slice(0, index),
        ...colors.slice(index, index + 2).toReversed(),
        ...colors.slice(index + 2),
    ];
}

/** Write colors as the CSS gradient that blends them evenly, flat for one alone. */
function blendOf(colors: readonly string[]): string {
    const stops = colors.length === 1 ? [...colors, ...colors] : colors;

    return `linear-gradient(to right in oklab, ${stops.join(", ")})`;
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
    color: ColorInput,
    colors: ColorsInput,
    json: JsonInput,
};
