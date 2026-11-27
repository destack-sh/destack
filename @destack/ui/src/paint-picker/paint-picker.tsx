import { t } from "@destack/locale";
import { field } from "@destack/object";
import {
    Gradient,
    GRADIENT_HUES,
    GRADIENT_SHAPES,
    GRADIENT_SPACES,
    type GradientJson,
    type GradientShape,
    IMAGE_FITS,
    Paint,
    PAINT_KINDS,
    type PaintKind,
    type PaintLayer,
    BoxPoint,
} from "@destack/style";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import {
    createControllableSignal,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import minusIcon from "@destack/icon/phosphor/minus";
import { Button } from "../button/index.ts";
import {
    checkers,
    ColorArea,
    ColorField,
    ColorPicker,
    ColorSlider,
} from "../color-picker/index.ts";
import { Icon } from "../icon/index.ts";
import { Input } from "../input/index.ts";
import { NumberField } from "../number-field/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";

/** The share of the strip an arrow key moves a stop by. */
const STEP = 0.01;

/** The share of the strip Shift with an arrow moves a stop by. */
const LARGE_STEP = 0.1;

/** The shape a strip draws its gradient in: along it, to the right. */
const ALONG_STRIP = { kind: "linear", angle: 90 } as const;

/** The paint a picker starts at while uncontrolled, black as design tools start. */
const BLACK: Paint = { kind: "solid", color: "#000000" };

/** How a percent reads in a field, such as a stop's position or a center. */
const PERCENT = { style: "percent", maximumFractionDigits: 0 } as const;

/** How an angle reads in a field. */
const DEGREES = { style: "unit", unit: "degree", maximumFractionDigits: 0 } as const;

/** The properties people change in a picker's controls and inspector. */
const PICKER_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses every part, so the value cannot change now.`)
        .optional(),
};

/** The properties people change in a paint swatch's controls and inspector. */
const PAINT_SWATCH_PROPERTIES = {
    paint: field
        .json(Paint)
        .labeled(t.context("label")`Paint`)
        .described(t`The color, gradient or image it shows.`)
        .optional(),
};

/** The styles of a paint picker and its parts. */
const styles = style.create({
    picker: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
    },
    row: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
    strip: {
        position: "relative",
        height: space[5],
        borderRadius: radius[2],
        boxShadow: `inset 0 0 0 ${stroke.border} ${color.border}`,
        cursor: "copy",
        touchAction: "none",
    },
    fill: (backgroundImage: string) => ({
        position: "absolute",
        inset: 0,
        borderRadius: "inherit",
        backgroundImage,
    }),
    stop: (position: string, background: string) => ({
        position: "absolute",
        insetBlock: `calc(-1 * ${space[1]})`,
        insetInlineStart: position,
        width: space[3],
        translate: "-50% 0",
        borderRadius: radius[1],
        background,
        boxShadow: `0 0 0 ${stroke.ring} ${color.background}, 0 0 0 calc(${stroke.ring} * 2) ${color.border}`,
        cursor: "grab",
    }),
    selected: {
        boxShadow: `0 0 0 ${stroke.ring} ${color.background}, 0 0 0 calc(${stroke.ring} * 2) ${color.ring}`,
        zIndex: 1,
    },
    swatch: {
        display: "inline-block",
        flexShrink: 0,
        width: space[5],
        height: space[5],
        borderRadius: radius[1],
        boxShadow: `inset 0 0 0 ${stroke.border} ${color.border}`,
        overflow: "hidden",
    },
    swatchFill: (layer: PaintLayer) => ({
        display: "block",
        width: "100%",
        height: "100%",
        backgroundImage: layer.image,
        backgroundSize: layer.size,
        backgroundPosition: layer.position,
        backgroundRepeat: layer.repeat,
    }),
    number: {
        width: "6rem",
    },
});

/** The properties of a picker of one value, the native element's attributes included. */
type PickerAttributes<Value> = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children" | "onChange"
> &
    ElementPartProperties & {
        /** The value, which makes it controlled. */
        readonly value?: Value | undefined;
        /** The value at first while uncontrolled. */
        readonly defaultValue?: Value;
        /** Handle each change of the value, as a stop is dragged. */
        readonly onValueChange?: (value: Value) => void;
    };

/** The properties of a gradient picker besides its declared ones: its gradient. */
export type GradientPickerAttributes = PickerAttributes<GradientJson>;

/** The properties of a paint picker besides its declared ones: its paint. */
export type PaintPickerAttributes = PickerAttributes<Paint>;

/** The properties of a paint swatch besides its declared ones: its native attributes. */
export type PaintSwatchAttributes = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> &
    ElementPartProperties;

/** Render a paint, over squares that show through where it is see-through. */
export const PaintSwatch = defineComponent({
    name: "paintSwatch",
    properties: PAINT_SWATCH_PROPERTIES,
    render(
        properties: RenderProperties<PaintSwatchAttributes, typeof PAINT_SWATCH_PROPERTIES>,
    ): JSX.Element {
        // show the paint's layer, nothing without one
        const locale = useLocale();
        const rest = omit(properties, "paint");

        return renderPart("span", "paint-swatch", rest, [checkers.backdrop, styles.swatch], {
            role: "img" as const,
            get "aria-label"() {
                const { paint } = properties;

                return locale.render(
                    paint === undefined ? t.context("label")`Paint` : PAINT_KINDS[paint.kind].title,
                );
            },
            get children() {
                return (
                    <Show when={properties.paint}>
                        {(paint) => (
                            <span {...style.attrs(styles.swatchFill(Paint.layer(paint())))} />
                        )}
                    </Show>
                );
            },
        });
    },
});

/** The properties of a paint swatch, its paint included. */
export type PaintSwatchProperties = ComponentProperties<typeof PaintSwatch>;

/** Render a gradient's shape, its stops on a strip, the selected stop's color, and its blend. */
export const GradientPicker = defineComponent({
    name: "gradientPicker",
    properties: PICKER_PROPERTIES,
    render(
        properties: RenderProperties<GradientPickerAttributes, typeof PICKER_PROPERTIES>,
    ): JSX.Element {
        // follow the controlled gradient or the picker's own
        const [value, setValue] = createControllableSignal<GradientJson>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? Paint.gradient(BLACK),
            defaultValue: untrack(() => properties.defaultValue) ?? Paint.gradient(BLACK),
            onChange: (next) => properties.onValueChange?.(next),
        });
        const rest = omit(properties, "value", "defaultValue", "onValueChange", "disabled");

        return renderPart("div", "gradient-picker", rest, styles.picker, {
            role: "group" as const,
            get "aria-disabled"() {
                return properties.disabled === true ? "true" : undefined;
            },
            get children() {
                return (
                    <GradientParts
                        value={value()}
                        disabled={properties.disabled === true}
                        onValueChange={setValue}
                    />
                );
            },
        });
    },
});

/** The properties of a gradient picker, its gradient included. */
export type GradientPickerProperties = ComponentProperties<typeof GradientPicker>;

/** Render a paint's kind, then its color, gradient or image. */
export const PaintPicker = defineComponent({
    name: "paintPicker",
    properties: PICKER_PROPERTIES,
    render(
        properties: RenderProperties<PaintPickerAttributes, typeof PICKER_PROPERTIES>,
    ): JSX.Element {
        // follow the controlled paint or the picker's own, and the kind shown
        const locale = useLocale();
        const [value, setValue] = createControllableSignal<Paint>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? BLACK,
            defaultValue: untrack(() => properties.defaultValue) ?? BLACK,
            onChange: (next) => properties.onValueChange?.(next),
        });
        const [pending, setPending] = createSignal<PaintKind | undefined>(undefined);
        const shown = (): PaintKind => pending() ?? value().kind;
        const isDisabled = (): boolean => properties.disabled === true;
        const rest = omit(properties, "value", "defaultValue", "onValueChange", "disabled");

        // read the paint by its kind
        const solid = () => {
            const paint = value();

            return paint.kind === "solid" ? paint : undefined;
        };
        const gradient = () => {
            const paint = value();

            return paint.kind === "gradient" ? paint : undefined;
        };

        // turn the paint into another kind, an image waiting for its address
        const choose = (kind: PaintKind): void => {
            setPending(kind === "image" && value().kind !== "image" ? "image" : undefined);
            if (kind === "solid" && solid() === undefined) {
                setValue({ kind: "solid", color: Paint.color(value()) });
            } else if (kind === "gradient" && gradient() === undefined) {
                setValue({ kind: "gradient", gradient: Paint.gradient(value()) });
            }
        };
        const give = (paint: Paint): void => {
            setPending(undefined);
            setValue(paint);
        };

        return renderPart("div", "paint-picker", rest, styles.picker, {
            role: "group" as const,
            get "aria-disabled"() {
                return isDisabled() ? "true" : undefined;
            },
            get children() {
                return (
                    <>
                        {/* Kind */}
                        <ToggleGroup
                            variant="segmented"
                            size="sm"
                            aria-label={locale.render(t.context("label")`Kind`)}
                            disabled={isDisabled()}
                            disallowEmptySelection
                            value={shown()}
                            onValueChange={(key) => {
                                if (key === "solid" || key === "gradient" || key === "image") {
                                    choose(key);
                                }
                            }}
                        >
                            <For each={Object.entries(PAINT_KINDS)}>
                                {([key, kind]) => (
                                    <ToggleGroupItem value={key}>
                                        {locale.render(kind.title)}
                                    </ToggleGroupItem>
                                )}
                            </For>
                        </ToggleGroup>

                        {/* Color */}
                        <Show when={shown() === "solid" ? solid() : undefined}>
                            {(paint) => (
                                <ColorPicker
                                    value={paint().color}
                                    disabled={isDisabled()}
                                    onValueChange={(next) =>
                                        setValue({ kind: "solid", color: next })
                                    }
                                >
                                    <ColorArea />
                                    <ColorSlider channel="hue" />
                                    <ColorSlider channel="alpha" />
                                    <ColorField />
                                </ColorPicker>
                            )}
                        </Show>

                        {/* Gradient */}
                        <Show when={shown() === "gradient" ? gradient() : undefined}>
                            {(paint) => (
                                <GradientParts
                                    value={paint().gradient}
                                    disabled={isDisabled()}
                                    onValueChange={(next) =>
                                        setValue({ kind: "gradient", gradient: next })
                                    }
                                />
                            )}
                        </Show>

                        {/* Image */}
                        <Show when={shown() === "image"}>
                            <ImageParts
                                value={value()}
                                disabled={isDisabled()}
                                onValueChange={give}
                            />
                        </Show>
                    </>
                );
            },
        });
    },
});

/** The properties of a paint picker, its paint included. */
export type PaintPickerProperties = ComponentProperties<typeof PaintPicker>;

/** Render a gradient's shape, its strip of stops, the selected stop and its blend. */
function GradientParts(properties: {
    /** The gradient. */
    readonly value: GradientJson;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle each change of the gradient. */
    readonly onValueChange: (value: GradientJson) => void;
}): JSX.Element {
    // read the gradient, and keep the stop selected
    const locale = useLocale();
    const gradient = (): Gradient => new Gradient(properties.value);
    const [selected, setSelected] = createSignal(0);
    const change = (next: Gradient): void => properties.onValueChange(next.toJson());
    const reshape = (shape: GradientShape): void => change(gradient().reshape(shape));
    const shape = (): GradientShape => properties.value.shape;

    // remove a stop and select its neighbour
    const remove = (index: number): void => {
        change(gradient().remove(index));
        setSelected(Math.max(index - 1, 0));
    };

    return (
        <>
            {/* Shape, with its angle and center */}
            <div {...style.attrs(styles.row)}>
                <ToggleGroup
                    variant="segmented"
                    size="sm"
                    aria-label={locale.render(t.context("label")`Shape`)}
                    disabled={properties.disabled}
                    disallowEmptySelection
                    value={shape().kind}
                    onValueChange={(key) => {
                        if (key === "linear" || key === "radial" || key === "conic") {
                            change(gradient().shaped(key));
                        }
                    }}
                >
                    <For each={Object.entries(GRADIENT_SHAPES)}>
                        {([key, choice]) => (
                            <ToggleGroupItem value={key}>
                                {locale.render(choice.title)}
                            </ToggleGroupItem>
                        )}
                    </For>
                </ToggleGroup>
                <ShapeFields
                    shape={shape()}
                    disabled={properties.disabled}
                    onShapeChange={reshape}
                />
            </div>

            {/* Strip of stops */}
            <GradientStrip
                gradient={gradient()}
                selected={selected()}
                disabled={properties.disabled}
                onSelect={setSelected}
                onChange={change}
                onRemove={remove}
            />

            {/* Selected stop's color and place */}
            <Show when={gradient().stops[selected()]}>
                {(stop) => (
                    <ColorPicker
                        value={stop().color}
                        disabled={properties.disabled}
                        onValueChange={(next) => change(gradient().recolor(selected(), next))}
                    >
                        <ColorArea />
                        <ColorSlider channel="hue" />
                        <ColorSlider channel="alpha" />
                        <div {...style.attrs(styles.row)}>
                            <ColorField />
                            <NumberField
                                aria-label={locale.render(t.context("label")`Position`)}
                                xstyle={styles.number}
                                value={stop().position}
                                min={0}
                                max={1}
                                step={STEP}
                                format={PERCENT}
                                disabled={properties.disabled}
                                onValueChange={(next) =>
                                    next !== undefined && change(gradient().move(selected(), next))
                                }
                            />
                            <Button
                                variant="ghost"
                                disabled={properties.disabled || gradient().stops.length <= 2}
                                onClick={() => remove(selected())}
                            >
                                <Icon icon={minusIcon} />
                                {locale.render(t.context("action")`Remove stop`)}
                            </Button>
                        </div>
                    </ColorPicker>
                )}
            </Show>

            {/* Space it blends in, and the way round the hues in one with hue */}
            <div {...style.attrs(styles.row)}>
                <ToggleGroup
                    variant="segmented"
                    size="sm"
                    aria-label={locale.render(t.context("label")`Blend`)}
                    disabled={properties.disabled}
                    disallowEmptySelection
                    value={gradient().space}
                    onValueChange={(key) => {
                        if (key === "oklab" || key === "oklch" || key === "srgb") {
                            change(gradient().blend(key, gradient().hue));
                        }
                    }}
                >
                    <For each={Object.entries(GRADIENT_SPACES)}>
                        {([key, choice]) => (
                            <ToggleGroupItem value={key}>
                                {locale.render(choice.title)}
                            </ToggleGroupItem>
                        )}
                    </For>
                </ToggleGroup>
                <Show when={gradient().space === "oklch"}>
                    <ToggleGroup
                        variant="segmented"
                        size="sm"
                        aria-label={locale.render(t.context("label")`Hue`)}
                        disabled={properties.disabled}
                        disallowEmptySelection
                        value={gradient().hue}
                        onValueChange={(key) => {
                            if (key === "shorter" || key === "longer") {
                                change(gradient().blend(gradient().space, key));
                            }
                        }}
                    >
                        <For each={Object.entries(GRADIENT_HUES)}>
                            {([key, choice]) => (
                                <ToggleGroupItem value={key}>
                                    {locale.render(choice.title)}
                                </ToggleGroupItem>
                            )}
                        </For>
                    </ToggleGroup>
                </Show>
            </div>
        </>
    );
}

/** Render a shape's angle, and its center where it has one. */
function ShapeFields(properties: {
    /** The shape. */
    readonly shape: GradientShape;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle the shape changing. */
    readonly onShapeChange: (shape: GradientShape) => void;
}): JSX.Element {
    // read the shape's angle and center, where it has them
    const locale = useLocale();
    const angled = () => {
        const { shape } = properties;

        return shape.kind === "radial" ? undefined : shape;
    };
    const centered = () => {
        const { shape } = properties;

        return shape.kind === "linear" ? undefined : shape;
    };

    return (
        <>
            {/* Angle of a linear or conic gradient */}
            <Show when={angled()}>
                {(shape) => (
                    <NumberField
                        aria-label={locale.render(t.context("label")`Angle`)}
                        xstyle={styles.number}
                        value={shape().angle}
                        min={0}
                        max={360}
                        step={1}
                        format={DEGREES}
                        disabled={properties.disabled}
                        onValueChange={(angle) =>
                            angle !== undefined && properties.onShapeChange({ ...shape(), angle })
                        }
                    />
                )}
            </Show>

            {/* Center of a radial or conic gradient */}
            <Show when={centered()}>
                {(shape) => (
                    <PointFields
                        point={shape().center}
                        disabled={properties.disabled}
                        onPointChange={(center) => properties.onShapeChange({ ...shape(), center })}
                    />
                )}
            </Show>
        </>
    );
}

/** Render a place in a box as its two percents across and down. */
function PointFields(properties: {
    /** The place. */
    readonly point: { readonly x: number; readonly y: number };
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle the place changing. */
    readonly onPointChange: (point: { readonly x: number; readonly y: number }) => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <>
            <NumberField
                aria-label={locale.render(t.context("label")`Across`)}
                xstyle={styles.number}
                value={properties.point.x}
                min={0}
                max={1}
                step={STEP}
                format={PERCENT}
                disabled={properties.disabled}
                onValueChange={(x) =>
                    x !== undefined && properties.onPointChange({ ...properties.point, x })
                }
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Down`)}
                xstyle={styles.number}
                value={properties.point.y}
                min={0}
                max={1}
                step={STEP}
                format={PERCENT}
                disabled={properties.disabled}
                onValueChange={(y) =>
                    y !== undefined && properties.onPointChange({ ...properties.point, y })
                }
            />
        </>
    );
}

/** Render a gradient's stops on a strip: pressing it adds one, dragging moves one. */
function GradientStrip(properties: {
    /** The gradient. */
    readonly gradient: Gradient;
    /** The selected stop's index. */
    readonly selected: number;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle a stop being selected. */
    readonly onSelect: (index: number) => void;
    /** Handle the gradient changing. */
    readonly onChange: (gradient: Gradient) => void;
    /** Handle a stop being removed. */
    readonly onRemove: (index: number) => void;
}): JSX.Element {
    // follow the strip's element
    const locale = useLocale();
    let strip: HTMLElement | undefined;

    // read the place along the strip under a pointer
    const placeOf = (event: PointerEvent): number => {
        const bounds = strip?.getBoundingClientRect();

        return bounds === undefined || bounds.width === 0
            ? 0
            : Math.min(Math.max((event.clientX - bounds.left) / bounds.width, 0), 1);
    };

    // drag a stop along the strip until the pointer lets go
    const drag = (
        index: number,
        event: PointerEvent & { readonly currentTarget: HTMLElement },
    ): void => {
        // take the pointer, so the stop follows it past the strip's ends
        properties.onSelect(index);
        const target = event.currentTarget;
        target.setPointerCapture(event.pointerId);

        // move the stop with the pointer, and stop following once it lets go
        const move = (moved: PointerEvent): void =>
            properties.onChange(properties.gradient.move(index, placeOf(moved)));
        const end = (): void => {
            target.removeEventListener("pointermove", move);
            target.removeEventListener("pointerup", end);
        };
        target.addEventListener("pointermove", move);
        target.addEventListener("pointerup", end);
    };

    // move and remove a stop by the keys, as a slider moves
    const press = (index: number, event: KeyboardEvent): void => {
        // read the step an arrow takes, Shift taking a larger one
        const stop = properties.gradient.stops[index];
        const step = event.shiftKey ? LARGE_STEP : STEP;
        const moves: Partial<Record<string, number>> = {
            ArrowLeft: -step,
            ArrowDown: -step,
            ArrowRight: step,
            ArrowUp: step,
        };
        const delta = moves[event.key];

        // move it by an arrow
        if (stop !== undefined && delta !== undefined) {
            event.preventDefault();
            properties.onChange(properties.gradient.move(index, stop.position + delta));
        }
        // move it to an end
        else if (event.key === "Home" || event.key === "End") {
            event.preventDefault();
            properties.onChange(properties.gradient.move(index, event.key === "Home" ? 0 : 1));
        }
        // remove it while two others stay
        else if (
            (event.key === "Delete" || event.key === "Backspace") &&
            properties.gradient.stops.length > 2
        ) {
            event.preventDefault();
            properties.onRemove(index);
        }
    };

    return (
        <div
            ref={(element) => (strip = element)}
            data-slot="gradient-picker-strip"
            dir="ltr"
            {...style.attrs(checkers.backdrop, styles.strip)}
            onPointerDown={(event) => {
                // add a stop on the strip itself, then drag it
                if (event.target === event.currentTarget && !properties.disabled) {
                    properties.onChange(properties.gradient.add(placeOf(event)));
                    drag(properties.gradient.stops.length, event);
                }
            }}
        >
            <div {...style.attrs(styles.fill(properties.gradient.reshape(ALONG_STRIP).css()))} />
            <For each={properties.gradient.stops}>
                {(stop, index) => (
                    <div
                        role="slider"
                        tabindex={properties.disabled ? -1 : 0}
                        data-slot="gradient-picker-stop"
                        data-selected={properties.selected === index() ? "" : undefined}
                        aria-label={locale.render(t.context("label")`Stop ${index() + 1}`)}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-valuenow={Math.round(stop.position * 100)}
                        aria-valuetext={`${locale.number(stop.position, PERCENT)}, ${stop.color}`}
                        aria-disabled={properties.disabled ? "true" : undefined}
                        {...style.attrs(
                            state.focus,
                            styles.stop(`${String(stop.position * 100)}%`, stop.color),
                            properties.selected === index() && styles.selected,
                        )}
                        onPointerDown={(event) => {
                            // select the stop, then drag it
                            event.stopPropagation();
                            if (!properties.disabled) {
                                drag(index(), event);
                            }
                        }}
                        onFocus={() => properties.onSelect(index())}
                        onKeyDown={(event) => !properties.disabled && press(index(), event)}
                    />
                )}
            </For>
        </div>
    );
}

/** Render an image paint's address, how it fills the box, and where it lines up. */
function ImageParts(properties: {
    /** The paint, an image once it has an address. */
    readonly value: Paint;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle the paint changing. */
    readonly onValueChange: (value: Paint) => void;
}): JSX.Element {
    // keep the address typed while it has none yet, and start an image covering the box from its center
    const locale = useLocale();
    const image = (): (Paint & { kind: "image" }) | undefined =>
        properties.value.kind === "image" ? properties.value : undefined;
    const change = (next: Partial<Omit<Paint & { kind: "image" }, "kind">>): void => {
        const current = image() ?? Paint.image("");
        const paint = { ...current, ...next };
        if (paint.source.length > 0) {
            properties.onValueChange(paint);
        }
    };

    return (
        <>
            {/* Address */}
            <Input
                type="url"
                aria-label={locale.render(t.context("label")`Address`)}
                placeholder="https://"
                value={image()?.source ?? ""}
                disabled={properties.disabled}
                onChange={(event) => change({ source: event.currentTarget.value.trim() })}
            />

            {/* Fit and place */}
            <div {...style.attrs(styles.row)}>
                <ToggleGroup
                    variant="segmented"
                    size="sm"
                    aria-label={locale.render(t.context("label")`Fit`)}
                    disabled={properties.disabled || image() === undefined}
                    disallowEmptySelection
                    value={image()?.fit ?? "cover"}
                    onValueChange={(key) => {
                        if (
                            key === "cover" ||
                            key === "contain" ||
                            key === "stretch" ||
                            key === "tile"
                        ) {
                            change({ fit: key });
                        }
                    }}
                >
                    <For each={Object.entries(IMAGE_FITS)}>
                        {([key, fit]) => (
                            <ToggleGroupItem value={key}>
                                {locale.render(fit.title)}
                            </ToggleGroupItem>
                        )}
                    </For>
                </ToggleGroup>
                <PointFields
                    point={image()?.position ?? BoxPoint.center}
                    disabled={properties.disabled || image() === undefined}
                    onPointChange={(position) => change({ position })}
                />
            </div>
        </>
    );
}
