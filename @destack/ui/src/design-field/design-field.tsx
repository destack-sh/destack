import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { type Message, t } from "@destack/locale";
import { field } from "@destack/object";
import { type JsonObject, JsonValue, schema, toJsonSchema } from "@destack/schema";
import {
    BLUR_KINDS,
    Blur,
    BoxSides,
    Color,
    Corners,
    Dimension,
    DIMENSION_UNITS,
    type DimensionUnit,
    LAYOUT_ALIGNS,
    LAYOUT_DIRECTIONS,
    LAYOUT_JUSTIFIES,
    Layout,
    type Shadow,
    ShadowStack,
    Stroke,
    STROKE_ALIGNS,
    STROKE_STYLES,
    Transform,
    Transition,
    Typography,
} from "@destack/style";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import {
    createControllableSignal,
    createSignal,
    For,
    type JSX,
    omit,
    untrack,
    useLocale,
} from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { Button } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { Input } from "../input/index.ts";
import { NumberField } from "../number-field/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import { Switch } from "../switch/index.ts";
import { Toggle } from "../toggle/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { type NamedValueField, ValueField } from "../value-field/index.ts";

/** The units a dimension field offers by default: pixels, the root's font size and a share. */
const UNITS: readonly DimensionUnit[] = ["px", "rem", "%"];

/** The easings a transition offers, as CSS names them. */
const EASINGS = ["linear", "ease", "ease-in", "ease-out", "ease-in-out"] as const;

/** Milliseconds per second, as CSS times convert. */
const MILLISECONDS = 1000;

/** The weights typography offers, as fonts name them in hundreds. */
const WEIGHT_STEP = 100;

/** The stroke an input starts at, a thin black border, as design tools start one. */
const START_STROKE: Stroke = { color: "#000000", width: "1px", align: "inside", style: "solid" };

/** The shadow an input adds, a soft drop below the box. */
const START_SHADOW: Shadow = {
    color: "#00000033",
    x: "0",
    y: "4px",
    blur: "12px",
    spread: "0",
    isInset: false,
};

/** The blur an input starts at, a slight one of the box. */
const START_BLUR: Blur = { kind: "layer", radius: "4px" };

/** The typography an input starts at, the browser's body text. */
const START_TYPOGRAPHY: Typography = {
    family: "sans-serif",
    size: "1rem",
    weight: 400,
    lineHeight: 1.5,
    letterSpacing: "0",
};

/** The transform an input starts at, which leaves the box as it is. */
const START_TRANSFORM: Transform = { x: "0", y: "0", rotate: 0, scaleX: 1, scaleY: 1 };

/** The layout an input starts at, children side by side from the start. */
const START_LAYOUT: Layout = {
    direction: "row",
    gap: "0",
    align: "start",
    justify: "start",
    isWrapping: false,
    padding: BoxSides.all("0"),
};

/** The transition an input starts at, a quick ease out. */
const START_TRANSITION: Transition = { duration: "200ms", delay: "0ms", easing: "ease-out" };

/** The lengths of a shadow an input shows, with what each is called. */
const SHADOW_LENGTHS = [
    ["x", t.context("label")`Across`],
    ["y", t.context("label")`Down`],
    ["blur", t.context("label")`Blur`],
    ["spread", t.context("label")`Spread`],
] as const;

/** The types of the values the composite inputs hand to their parts' fields. */
const TYPES = {
    color: describe(Color.schema),
    dimension: describe(Dimension),
    boxSides: describe(BoxSides),
};

/** The properties people change in a dimension field's controls and inspector. */
const DIMENSION_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses the number and the unit, so the length cannot change now.`)
        .optional(),
};

/** The styles of the design fields. */
const styles = style.create({
    dimension: {
        display: "flex",
        gap: space[1],
        minWidth: 0,
    },
    number: {
        flex: 1,
        minWidth: "4rem",
    },
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

/** The properties of a dimension field besides its declared ones: its length and units. */
export type DimensionFieldAttributes = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children" | "onChange"
> &
    ElementPartProperties & {
        /** The length, which makes it controlled. */
        readonly value?: string | undefined;
        /** The length at first while uncontrolled, none by default. */
        readonly defaultValue?: string;
        /** Handle each change of the length. */
        readonly onValueChange?: (value: string) => void;
        /** The units it offers, pixels, rem and percent by default. */
        readonly units?: readonly DimensionUnit[];
        /** What it is called, where no label beside it says so. */
        readonly "aria-label"?: string | undefined;
    };

/** Render a length or a percentage: its number, and its unit picked beside it. */
export const DimensionField = defineComponent({
    name: "dimensionField",
    properties: DIMENSION_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<DimensionFieldAttributes, typeof DIMENSION_FIELD_PROPERTIES>,
    ): JSX.Element {
        // follow the controlled length or the field's own, and read its number and unit
        const locale = useLocale();
        const [value, setValue] = createControllableSignal<string>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? "0",
            defaultValue: untrack(() => properties.defaultValue) ?? "0",
            onChange: (next) => properties.onValueChange?.(next),
        });
        const split = () => Dimension.split(value());
        const units = (): readonly DimensionUnit[] => {
            const offered = properties.units ?? UNITS;

            return offered.includes(split().unit) ? offered : [...offered, split().unit];
        };
        const rest = omit(
            properties,
            "value",
            "defaultValue",
            "onValueChange",
            "units",
            "disabled",
            "aria-label",
        );

        return renderPart("div", "dimension-field", rest, styles.dimension, {
            role: "group" as const,
            get "aria-label"() {
                return properties["aria-label"];
            },
            get children() {
                return (
                    <>
                        {/* Number */}
                        <NumberField
                            aria-label={locale.render(t.context("label")`Amount`)}
                            xstyle={styles.number}
                            value={split().value}
                            disabled={properties.disabled === true}
                            onValueChange={(next) =>
                                next !== undefined && setValue(Dimension.of(next, split().unit))
                            }
                        />

                        {/* Unit */}
                        <Select
                            aria-label={locale.render(t.context("label")`Unit`)}
                            disabled={properties.disabled === true}
                            value={split().unit}
                            onValueChange={(unit) => {
                                const known = DIMENSION_UNITS.find((each) => each === unit);
                                if (known !== undefined) {
                                    setValue(Dimension.of(split().value, known));
                                }
                            }}
                        >
                            <For each={units()}>
                                {(unit) => <SelectItem value={unit}>{unit}</SelectItem>}
                            </For>
                        </Select>
                    </>
                );
            },
        });
    },
});

/** The properties of a dimension field, its length included. */
export type DimensionFieldProperties = ComponentProperties<typeof DimensionField>;

/** The properties of the input of a named JSON value. */
type NamedProperties = Parameters<NamedValueField>[0];

/** Render the input of a length: its dimension field. */
export function DimensionInput(properties: NamedProperties): JSX.Element {
    return (
        <DimensionField
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            disabled={properties.disabled}
            value={Dimension.safeParse(properties.value).data}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render the input of a box's corners: one radius for all, or each corner's own. */
export function CornersInput(properties: NamedProperties): JSX.Element {
    // read the corners, square ones where none are given
    const corners = (): Corners => Corners.safeParse(properties.value).data ?? Corners.all("0");
    const labels = {
        topLeft: t.context("label")`Top left`,
        topRight: t.context("label")`Top right`,
        bottomRight: t.context("label")`Bottom right`,
        bottomLeft: t.context("label")`Bottom left`,
    } satisfies Record<keyof Corners, Message>;

    return (
        <SidesParts
            label={properties["aria-label"]}
            keys={["topLeft", "topRight", "bottomRight", "bottomLeft"]}
            labels={labels}
            each={t.context("action")`Each corner`}
            value={corners()}
            disabled={properties.disabled === true}
            all={(length) => Corners.all(length)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render the input of a box's sides: one length for all, or each side's own. */
export function BoxSidesInput(properties: NamedProperties): JSX.Element {
    // read the sides, none where none are given
    const sides = (): BoxSides => BoxSides.safeParse(properties.value).data ?? BoxSides.all("0");
    const labels = {
        top: t.context("label")`Top`,
        right: t.context("label")`Right`,
        bottom: t.context("label")`Bottom`,
        left: t.context("label")`Left`,
    } satisfies Record<keyof BoxSides, Message>;

    return (
        <SidesParts
            label={properties["aria-label"]}
            keys={["top", "right", "bottom", "left"]}
            labels={labels}
            each={t.context("action")`Each side`}
            value={sides()}
            disabled={properties.disabled === true}
            all={(length) => BoxSides.all(length)}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render four lengths of a box: one for all while they match, else each one's own. */
function SidesParts<Value extends Readonly<Record<string, string>>>(properties: {
    /** What the four are called. */
    readonly label: string | undefined;
    /** The four, in the order CSS writes them. */
    readonly keys: readonly (keyof Value & string)[];
    /** What each of the four is called. */
    readonly labels: Readonly<Record<keyof Value & string, Message>>;
    /** What the toggle to set each one apart is called. */
    readonly each: Message;
    /** The four lengths. */
    readonly value: Value;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Give all four one length. */
    readonly all: (length: string) => Value;
    /** Handle the lengths changing. */
    readonly onValueChange: (value: Value) => void;
}): JSX.Element {
    // show one length while all match, and each one's own once they differ or a person asks
    const locale = useLocale();
    const lengths = () => properties.keys.map((key) => properties.value[key]);
    const [isEach, setEach] = createSignal(untrack(() => new Set(lengths()).size > 1));

    return (
        <div role="group" aria-label={properties.label} {...style.attrs(styles.group)}>
            {/* One length, or each */}
            <div {...style.attrs(styles.row)}>
                <DimensionField
                    aria-label={properties.label}
                    disabled={properties.disabled || isEach()}
                    value={lengths()[0]}
                    onValueChange={(length) => properties.onValueChange(properties.all(length))}
                />
                <Toggle
                    pressed={isEach()}
                    disabled={properties.disabled}
                    aria-label={locale.render(properties.each)}
                    onPressedChange={setEach}
                >
                    {locale.render(properties.each)}
                </Toggle>
            </div>

            {/* Each one's own */}
            <div hidden={!isEach()} {...style.attrs(styles.grid)}>
                <For each={properties.keys}>
                    {(key) => (
                        <DimensionField
                            aria-label={locale.render(properties.labels[key])}
                            disabled={properties.disabled}
                            value={properties.value[key]}
                            onValueChange={(length) =>
                                properties.onValueChange({ ...properties.value, [key]: length })
                            }
                        />
                    )}
                </For>
            </div>
        </div>
    );
}

/** Render the input of a stroke: its color, width, where it runs and how it is drawn. */
export function StrokeInput(properties: NamedProperties): JSX.Element {
    // read the stroke, a thin black border where none is given
    const locale = useLocale();
    const stroke = (): Stroke => Stroke.safeParse(properties.value).data ?? START_STROKE;
    const change = (next: Partial<Stroke>): void =>
        properties.onValueChange({ ...stroke(), ...next });

    return (
        <div role="group" aria-label={properties["aria-label"]} {...style.attrs(styles.group)}>
            {/* Color and width */}
            <div {...style.attrs(styles.row)}>
                <ValueField
                    type={TYPES.color}
                    aria-label={locale.render(t.context("label")`Color`)}
                    disabled={properties.disabled}
                    value={stroke().color}
                    onValueChange={(color) => typeof color === "string" && change({ color })}
                />
                <DimensionField
                    aria-label={locale.render(t.context("label")`Width`)}
                    disabled={properties.disabled === true}
                    value={stroke().width}
                    onValueChange={(width) => change({ width })}
                />
            </div>

            {/* Where it runs, and how it is drawn */}
            <div {...style.attrs(styles.row)}>
                <Choices
                    label={t.context("label")`Position`}
                    choices={STROKE_ALIGNS}
                    value={stroke().align}
                    disabled={properties.disabled === true}
                    onValueChange={(align) => change({ align: Stroke.shape.align.parse(align) })}
                />
                <Choices
                    label={t.context("label")`Style`}
                    choices={STROKE_STYLES}
                    value={stroke().style}
                    disabled={properties.disabled === true}
                    onValueChange={(line) => change({ style: Stroke.shape.style.parse(line) })}
                />
            </div>
        </div>
    );
}

/** Render the input of shadows: each one's color, offset, blur and spread, first on top. */
export function ShadowStackInput(properties: NamedProperties): JSX.Element {
    // read the shadows, none where none are given
    const locale = useLocale();
    const shadows = (): readonly Shadow[] => ShadowStack.safeParse(properties.value).data ?? [];
    const write = (next: readonly Shadow[]): void => properties.onValueChange([...next]);
    const update = (index: number, change: Partial<Shadow>): void => {
        // change one shadow, leaving the others
        const shadow = shadows()[index];
        if (shadow !== undefined) {
            write(shadows().with(index, { ...shadow, ...change }));
        }
    };

    return (
        <div role="group" aria-label={properties["aria-label"]} {...style.attrs(styles.group)}>
            {/* Shadows, first on top */}
            <For each={shadows()}>
                {(shadow, index) => (
                    <div
                        role="group"
                        aria-label={locale.render(t.context("label")`Shadow ${index() + 1}`)}
                        {...style.attrs(styles.group)}
                    >
                        <div {...style.attrs(styles.row)}>
                            <ValueField
                                type={TYPES.color}
                                aria-label={locale.render(t.context("label")`Color`)}
                                disabled={properties.disabled}
                                value={shadow.color}
                                onValueChange={(color) =>
                                    typeof color === "string" && update(index(), { color })
                                }
                            />
                            <Switch
                                aria-label={locale.render(t.context("label")`Inside`)}
                                disabled={properties.disabled}
                                checked={shadow.isInset}
                                onCheckedChange={(isInset) => update(index(), { isInset })}
                            />
                            <Button
                                variant="ghost"
                                isIconOnly
                                disabled={properties.disabled}
                                aria-label={locale.render(t.context("action")`Remove shadow`)}
                                onClick={() => write(shadows().toSpliced(index(), 1))}
                            >
                                <Icon icon={minusIcon} />
                            </Button>
                        </div>
                        <div {...style.attrs(styles.grid)}>
                            <For each={SHADOW_LENGTHS}>
                                {([key, label]) => (
                                    <DimensionField
                                        aria-label={locale.render(label)}
                                        units={["px", "rem"]}
                                        disabled={properties.disabled === true}
                                        value={shadow[key]}
                                        onValueChange={(length) =>
                                            update(index(), { [key]: length })
                                        }
                                    />
                                )}
                            </For>
                        </div>
                    </div>
                )}
            </For>

            {/* Add one on top */}
            <Button
                variant="ghost"
                disabled={properties.disabled}
                onClick={() => write([START_SHADOW, ...shadows()])}
            >
                <Icon icon={plusIcon} />
                {locale.render(t.context("action")`Add shadow`)}
            </Button>
        </div>
    );
}

/** Render the input of a blur: what it softens, and how far. */
export function BlurInput(properties: NamedProperties): JSX.Element {
    // read the blur, a slight one where none is given
    const locale = useLocale();
    const blur = (): Blur => Blur.safeParse(properties.value).data ?? START_BLUR;

    return (
        <div role="group" aria-label={properties["aria-label"]} {...style.attrs(styles.row)}>
            <Choices
                label={t.context("label")`Kind`}
                choices={BLUR_KINDS}
                value={blur().kind}
                disabled={properties.disabled === true}
                onValueChange={(kind) =>
                    properties.onValueChange({ ...blur(), kind: Blur.shape.kind.parse(kind) })
                }
            />
            <DimensionField
                aria-label={locale.render(t.context("label")`Radius`)}
                units={["px", "rem"]}
                disabled={properties.disabled === true}
                value={blur().radius}
                onValueChange={(radius) => properties.onValueChange({ ...blur(), radius })}
            />
        </div>
    );
}

/** Render the input of typography: its family, size, weight, line height and letter spacing. */
export function TypographyInput(properties: NamedProperties): JSX.Element {
    // read the typography, the browser's body text where none is given
    const locale = useLocale();
    const typography = (): Typography =>
        Typography.safeParse(properties.value).data ?? START_TYPOGRAPHY;
    const change = (next: Partial<Typography>): void =>
        properties.onValueChange({ ...typography(), ...next });

    return (
        <div role="group" aria-label={properties["aria-label"]} {...style.attrs(styles.group)}>
            {/* Family */}
            <Input
                aria-label={locale.render(t.context("label")`Font family`)}
                disabled={properties.disabled}
                value={typography().family}
                onChange={(event) => {
                    const family = event.currentTarget.value.trim();
                    if (family.length > 0) {
                        change({ family });
                    }
                }}
            />

            {/* Size, weight, line height and letter spacing */}
            <div {...style.attrs(styles.grid)}>
                <DimensionField
                    aria-label={locale.render(t.context("label")`Size`)}
                    units={["px", "rem", "em"]}
                    disabled={properties.disabled === true}
                    value={typography().size}
                    onValueChange={(size) => change({ size })}
                />
                <NumberField
                    aria-label={locale.render(t.context("label")`Weight`)}
                    min={WEIGHT_STEP}
                    max={WEIGHT_STEP * 9}
                    step={WEIGHT_STEP}
                    disabled={properties.disabled}
                    value={typography().weight}
                    onValueChange={(weight) => weight !== undefined && change({ weight })}
                />
                <NumberField
                    aria-label={locale.render(t.context("label")`Line height`)}
                    min={0.5}
                    max={4}
                    step={0.05}
                    disabled={properties.disabled}
                    value={typography().lineHeight}
                    onValueChange={(lineHeight) =>
                        lineHeight !== undefined && change({ lineHeight })
                    }
                />
                <DimensionField
                    aria-label={locale.render(t.context("label")`Letter spacing`)}
                    units={["px", "em"]}
                    disabled={properties.disabled === true}
                    value={typography().letterSpacing}
                    onValueChange={(letterSpacing) => change({ letterSpacing })}
                />
            </div>
        </div>
    );
}

/** Render the input of a transform: its move, turn and scale. */
export function TransformInput(properties: NamedProperties): JSX.Element {
    // read the transform, none where none is given
    const locale = useLocale();
    const transform = (): Transform =>
        Transform.safeParse(properties.value).data ?? START_TRANSFORM;
    const change = (next: Partial<Transform>): void =>
        properties.onValueChange({ ...transform(), ...next });

    return (
        <div role="group" aria-label={properties["aria-label"]} {...style.attrs(styles.grid)}>
            <DimensionField
                aria-label={locale.render(t.context("label")`Across`)}
                disabled={properties.disabled === true}
                value={transform().x}
                onValueChange={(x) => change({ x })}
            />
            <DimensionField
                aria-label={locale.render(t.context("label")`Down`)}
                disabled={properties.disabled === true}
                value={transform().y}
                onValueChange={(y) => change({ y })}
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Rotation`)}
                min={-360}
                max={360}
                format={{ style: "unit", unit: "degree" }}
                disabled={properties.disabled}
                value={transform().rotate}
                onValueChange={(rotate) => rotate !== undefined && change({ rotate })}
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Width scale`)}
                step={0.1}
                disabled={properties.disabled}
                value={transform().scaleX}
                onValueChange={(scaleX) => scaleX !== undefined && change({ scaleX })}
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Height scale`)}
                step={0.1}
                disabled={properties.disabled}
                value={transform().scaleY}
                onValueChange={(scaleY) => scaleY !== undefined && change({ scaleY })}
            />
        </div>
    );
}

/** Render the input of a layout: its direction, gap, alignment, wrapping and padding. */
export function LayoutInput(properties: NamedProperties): JSX.Element {
    // read the layout, children side by side where none is given
    const locale = useLocale();
    const layout = (): Layout => Layout.safeParse(properties.value).data ?? START_LAYOUT;
    const change = (next: Partial<Layout>): void =>
        properties.onValueChange({ ...layout(), ...next });

    return (
        <div role="group" aria-label={properties["aria-label"]} {...style.attrs(styles.group)}>
            {/* Direction, gap and wrapping */}
            <div {...style.attrs(styles.row)}>
                <Choices
                    label={t.context("label")`Direction`}
                    choices={LAYOUT_DIRECTIONS}
                    value={layout().direction}
                    disabled={properties.disabled === true}
                    onValueChange={(direction) =>
                        change({ direction: Layout.shape.direction.parse(direction) })
                    }
                />
                <DimensionField
                    aria-label={locale.render(t.context("label")`Gap`)}
                    disabled={properties.disabled === true}
                    value={layout().gap}
                    onValueChange={(gap) => change({ gap })}
                />
                <Switch
                    aria-label={locale.render(t.context("label")`Wrap`)}
                    disabled={properties.disabled}
                    checked={layout().isWrapping}
                    onCheckedChange={(isWrapping) => change({ isWrapping })}
                />
            </div>

            {/* Alignment across and along */}
            <div {...style.attrs(styles.row)}>
                <Choices
                    label={t.context("label")`Align`}
                    choices={LAYOUT_ALIGNS}
                    value={layout().align}
                    disabled={properties.disabled === true}
                    onValueChange={(align) => change({ align: Layout.shape.align.parse(align) })}
                />
                <Choices
                    label={t.context("label")`Justify`}
                    choices={LAYOUT_JUSTIFIES}
                    value={layout().justify}
                    disabled={properties.disabled === true}
                    onValueChange={(justify) =>
                        change({ justify: Layout.shape.justify.parse(justify) })
                    }
                />
            </div>

            {/* Padding */}
            <ValueField
                type={TYPES.boxSides}
                aria-label={locale.render(t.context("label")`Padding`)}
                disabled={properties.disabled}
                value={layout().padding}
                onValueChange={(padding) => {
                    const sides = BoxSides.safeParse(padding).data;
                    if (sides !== undefined) {
                        change({ padding: sides });
                    }
                }}
            />
        </div>
    );
}

/** Render the input of a transition: how long it takes and waits, and how it eases. */
export function TransitionInput(properties: NamedProperties): JSX.Element {
    // read the transition, a quick ease out where none is given
    const locale = useLocale();
    const transition = (): Transition =>
        Transition.safeParse(properties.value).data ?? START_TRANSITION;
    const change = (next: Partial<Transition>): void =>
        properties.onValueChange({ ...transition(), ...next });

    return (
        <div role="group" aria-label={properties["aria-label"]} {...style.attrs(styles.row)}>
            <NumberField
                aria-label={locale.render(t.context("label")`Duration`)}
                min={0}
                step={50}
                format={{ style: "unit", unit: "millisecond" }}
                disabled={properties.disabled}
                value={millisecondsOf(transition().duration)}
                onValueChange={(time) =>
                    time !== undefined && change({ duration: `${String(time)}ms` })
                }
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Delay`)}
                min={0}
                step={50}
                format={{ style: "unit", unit: "millisecond" }}
                disabled={properties.disabled}
                value={millisecondsOf(transition().delay)}
                onValueChange={(time) =>
                    time !== undefined && change({ delay: `${String(time)}ms` })
                }
            />
            <Select
                aria-label={locale.render(t.context("label")`Easing`)}
                disabled={properties.disabled}
                value={transition().easing}
                onValueChange={(easing) => easing !== undefined && change({ easing })}
            >
                <For each={[...EASINGS]}>
                    {(easing) => <SelectItem value={easing}>{easing}</SelectItem>}
                </For>
            </Select>
        </div>
    );
}

/** Render a few choices of a value type as a segmented toggle, titled in the person's language. */
function Choices(properties: {
    /** What the choice is called. */
    readonly label: Message;
    /** The choices, by value, with their titles. */
    readonly choices: Readonly<Record<string, { readonly title: Message }>>;
    /** The value chosen. */
    readonly value: string;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle another value being chosen. */
    readonly onValueChange: (value: string) => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <ToggleGroup
            variant="segmented"
            size="sm"
            aria-label={locale.render(properties.label)}
            disabled={properties.disabled}
            disallowEmptySelection
            value={properties.value}
            onValueChange={(key) => typeof key === "string" && properties.onValueChange(key)}
        >
            <For each={Object.entries(properties.choices)}>
                {([key, choice]) => (
                    <ToggleGroupItem value={key}>{locale.render(choice.title)}</ToggleGroupItem>
                )}
            </For>
        </ToggleGroup>
    );
}

/** Read a CSS time in milliseconds. */
function millisecondsOf(time: string): number {
    return time.endsWith("ms") ? Number.parseFloat(time) : Number.parseFloat(time) * MILLISECONDS;
}

/** Describe a value type's schema as the JSON Schema a JSON input carries. */
function describe(definition: schema.Schema): {
    readonly kind: "json";
    readonly schema: JsonObject;
} {
    return { kind: "json", schema: JsonValue.object(toJsonSchema(definition)) };
}
