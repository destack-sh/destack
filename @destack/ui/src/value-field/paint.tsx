import { useFieldControl } from "../field/control.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Icon } from "../icon/index.ts";
import { Button } from "../button/index.ts";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { present } from "@destack/schema";
import {
    type Bindable,
    BLEND_MODES,
    BlendMode,
    type Fill,
    FillStack,
    Gradient,
    Paint,
    PAINT_KINDS,
} from "@destack/style";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { For, type JSX, Show, useLocale } from "@destack/view";
import { ColorPart } from "./color.tsx";
import { Select, SelectItem } from "../select/index.ts";
import { GradientPicker, PaintPicker, PaintSwatch } from "../paint-picker/index.ts";
import { ValueSet, ValueStack, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** The layer a stack adds on top, black as design tools add one. */
const NEW_LAYER: Fill = { paint: Paint.black, blend: "normal", isVisible: true };

/** The styles of the inputs of paints. */
const styles = style.create({
    picker: {
        width: "20rem",
    },
    stack: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
    },
});

/** Render the input of a paint: its swatch and kind, which open a picker. */
export function PaintInput(properties: NamedValueInputProperties): JSX.Element {
    // read the paint, none while it is not one
    const field = useFieldControl();
    const locale = useLocale();
    const paint = (): Paint | undefined => readValue(properties.value, Paint);

    return (
        <Popover>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="outline"
                        id={field?.id ?? attributes.id}
                        aria-label={properties["aria-label"]}
                        aria-describedby={
                            properties["aria-describedby"] ??
                            field?.attributes()["aria-describedby"]
                        }
                        disabled={properties.disabled}
                        xstyle={valueStyles.summary}
                    >
                        <>
                            <PaintSwatch paint={paint()} />
                            <Show when={paint()}>
                                {(shown) => <>{locale.render(PAINT_KINDS[shown().kind].title)}</>}
                            </Show>
                        </>
                    </Button>
                )}
            />
            <PopoverContent
                title={properties["aria-label"]}
                side="left"
                align="start"
                xstyle={styles.picker}
            >
                <PaintPicker
                    value={paint()}
                    onValueChange={(next) => properties.onValueChange(next)}
                />
            </PopoverContent>
        </Popover>
    );
}

/** Render the input of a gradient: its swatch, which opens a picker. */
export function GradientInput(properties: NamedValueInputProperties): JSX.Element {
    // read the gradient, none while it is not one, and paint its swatch with it
    const field = useFieldControl();
    const gradient = () => readValue(properties.value, Gradient.schema);
    const paint = (): Paint | undefined => {
        const value = gradient();

        return value === undefined ? undefined : { kind: "gradient", gradient: value };
    };

    return (
        <Popover>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="outline"
                        id={field?.id ?? attributes.id}
                        aria-label={properties["aria-label"]}
                        aria-describedby={
                            properties["aria-describedby"] ??
                            field?.attributes()["aria-describedby"]
                        }
                        disabled={properties.disabled}
                        xstyle={valueStyles.summary}
                    >
                        <PaintSwatch paint={paint()} />
                    </Button>
                )}
            />
            <PopoverContent
                title={properties["aria-label"]}
                side="left"
                align="start"
                xstyle={styles.picker}
            >
                <GradientPicker
                    value={gradient()}
                    onValueChange={(next) => properties.onValueChange(next)}
                />
            </PopoverContent>
        </Popover>
    );
}

/** Render a box's fill: its layers top first, each reordered, picked, blended or hidden. */
export function FillStackInput(properties: NamedValueInputProperties): JSX.Element {
    // read the stack, and write it back changed
    const locale = useLocale();
    const stack = (): readonly Fill[] => readValue(properties.value, FillStack) ?? [];
    const write = (next: readonly Fill[]): void => properties.onValueChange([...next]);
    const update = (index: number, change: Partial<Fill>): void => {
        const fill = present(stack()[index], "the changed layer");
        write(stack().with(index, { ...fill, ...change }));
    };
    const isDisabled = (): boolean => properties.disabled === true;

    return (
        <ValueSet
            actions={
                <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label={locale.render(t.context("action")`Add layer`)}
                    disabled={isDisabled()}
                    onClick={() => write([NEW_LAYER, ...stack()])}
                >
                    <Icon icon={plusIcon} />
                </Button>
            }
            label={properties.label}
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            xstyle={styles.stack}
        >
            {/* Layers, top first */}
            <ValueStack
                items={stack()}
                label={(index) => locale.render(t.context("label")`Layer ${index + 1}`)}
                isVisible={(fill) => fill.isVisible}
                onVisibleChange={(index, isVisible) => update(index, { isVisible })}
                disabled={isDisabled()}
                onValueChange={write}
            >
                {(fill, index) => {
                    const label = () => locale.render(t.context("label")`Layer ${index + 1}`);

                    return (
                        /* A color as a color row, else the paint in a popover */
                        <Show
                            when={solidOf(fill().paint)}
                            fallback={
                                <Popover>
                                    <PopoverTrigger
                                        render={(attributes) => (
                                            <Button
                                                {...attributes}
                                                variant="ghost"
                                                aria-label={label()}
                                                disabled={isDisabled()}
                                                xstyle={valueStyles.summary}
                                            >
                                                <>
                                                    <PaintSwatch paint={fill().paint} />
                                                    {locale.render(
                                                        PAINT_KINDS[fill().paint.kind].title,
                                                    )}
                                                </>
                                            </Button>
                                        )}
                                    />
                                    <PopoverContent
                                        title={label()}
                                        side="left"
                                        align="start"
                                        xstyle={styles.picker}
                                    >
                                        <FillPicker
                                            fill={fill()}
                                            disabled={isDisabled()}
                                            onFillChange={(change) => update(index, change)}
                                        />
                                    </PopoverContent>
                                </Popover>
                            }
                        >
                            {(color) => (
                                <ColorPart
                                    label={label()}
                                    disabled={isDisabled()}
                                    value={color()}
                                    custom={
                                        <FillPicker
                                            fill={fill()}
                                            disabled={isDisabled()}
                                            onFillChange={(change) => update(index, change)}
                                        />
                                    }
                                    onValueChange={(next) =>
                                        update(index, { paint: { kind: "solid", color: next } })
                                    }
                                />
                            )}
                        </Show>
                    );
                }}
            </ValueStack>
        </ValueSet>
    );
}

/** Render a layer's paint and blend mode, in a color's custom tab or a paint's popover. */
function FillPicker(properties: {
    /** The layer. */
    readonly fill: Fill;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle the paint or blend mode changing. */
    readonly onFillChange: (change: Partial<Fill>) => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <>
            {/* Paint */}
            <PaintPicker
                value={properties.fill.paint}
                onValueChange={(paint) => properties.onFillChange({ paint })}
            />

            {/* Blend mode */}
            <Select
                aria-label={locale.render(t.context("label")`Blend mode`)}
                disabled={properties.disabled}
                value={properties.fill.blend}
                onValueChange={(key) => {
                    const blend = BlendMode.safeParse(key).data;
                    if (blend !== undefined) {
                        properties.onFillChange({ blend });
                    }
                }}
            >
                <For each={Object.entries(BLEND_MODES)}>
                    {([key, mode]) => (
                        <SelectItem value={key}>{locale.render(mode.title)}</SelectItem>
                    )}
                </For>
            </Select>
        </>
    );
}

/** Read the color of a solid paint, none for a gradient or an image. */
function solidOf(paint: Paint): Bindable<string> | undefined {
    return paint.kind === "solid" ? paint.color : undefined;
}
