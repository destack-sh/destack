import { Color, COLOR_CHANNELS, type ColorChannel, type ColorSpace } from "@destack/color";
import eyedropperIcon from "@destack/icon/phosphor/eyedropper";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, space, stroke, width } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createEffect,
    createMemo,
    createSignal,
    type JSX,
    merge,
    omit,
    onSettled,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { readText, useFieldValue } from "../field/control.ts";
import { Icon } from "../icon/index.ts";
import { InputGroup } from "../input-group/index.ts";
import { Input, type InputProperties } from "../input/index.ts";
import {
    NumberField,
    NumberFieldInput,
    type NumberFieldProperties,
} from "../number-field/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { Slider, type SliderProperties, SliderThumb, SliderTrack } from "../slider/index.ts";
import {
    CHANNEL_NAMES,
    ColorPickerContext,
    ColorPickerControl,
    type ColorPickerOptions,
    useColorPicker,
} from "./control.ts";
import { nameOf } from "./name.ts";

export { type ColorPickerOptions, useColorPicker } from "./control.ts";

/** The points an area is painted in along each side, smooth once the screen scales them. */
const POINTS = 48;

/** The colors a slider's track runs through, enough for the turns of a hue. */
const STOPS = 24;

/** The steps Page Up, Page Down and Shift with an arrow take in an area. */
const LARGE_STEP = 10;

/** The squares behind a color that lets them through. */
const CHECKERS = `repeating-conic-gradient(color-mix(in oklab, ${color.mutedForeground} 35%, ${color.background}) 0% 25%, ${color.background} 0% 50%) 50% / ${space[2]} ${space[2]}`;

/** The edge of a thumb, seen on any color. */
const THUMB_EDGE = "0 0 0 1px rgb(0 0 0 / 0.5), inset 0 0 0 1px rgb(0 0 0 / 0.5)";

/** The styles of a color picker and its parts. */
const styles = style.create({
    picker: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        width: width.popover,
        maxWidth: "100%",
    },
    area: {
        position: "relative",
        width: "100%",
        aspectRatio: "4 / 3",
        borderRadius: radius[3],
        touchAction: "none",
        userSelect: "none",
        cursor: { default: "crosshair", ":is([data-disabled])": "not-allowed" },
        opacity: { default: 1, ":is([data-disabled])": 0.5 },
        forcedColorAdjust: "none",
    },
    paint: {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        borderRadius: "inherit",
        outlineStyle: "solid",
        outlineWidth: stroke.border,
        outlineOffset: `calc(-1 * ${stroke.border})`,
        outlineColor: color.border,
    },
    thumb: {
        position: "absolute",
        boxSizing: "border-box",
        width: space[4],
        height: space[4],
        translate: "-50% 50%",
        borderWidth: stroke.ring,
        borderStyle: "solid",
        borderColor: "white",
        borderRadius: radius.full,
        boxShadow: THUMB_EDGE,
        forcedColorAdjust: "none",
        outlineStyle: { default: "none", ":focus-within": "solid" },
        outlineWidth: stroke.ring,
        outlineOffset: stroke.ring,
        outlineColor: color.ring,
        // take a finger beside it
        "::before": {
            content: "''",
            position: "absolute",
            inset: `calc(-1 * ${space[2]})`,
        },
    },
    thumbAt: (inline: string, block: string, fill: string) => ({
        insetInlineStart: inline,
        insetBlockEnd: block,
        backgroundColor: fill,
    }),
    input: {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        margin: 0,
        opacity: 0,
        pointerEvents: "none",
    },
    slider: {
        height: space[4],
    },
    track: {
        height: space[4],
        background: CHECKERS,
        forcedColorAdjust: "none",
    },
    vertical: {
        width: space[4],
        height: "auto",
    },
    run: (image: string) => ({
        backgroundImage: image,
    }),
    fill: {
        position: "absolute",
        inset: 0,
        borderRadius: "inherit",
    },
    sliderThumb: {
        "::-webkit-slider-thumb": {
            borderWidth: stroke.ring,
            borderColor: "white",
            backgroundColor: "currentColor",
            boxShadow: {
                default: THUMB_EDGE,
                ":focus-visible": `0 0 0 ${stroke.ring} ${color.ring}`,
            },
            forcedColorAdjust: "none",
        },
        "::-moz-range-thumb": {
            borderWidth: stroke.ring,
            borderColor: "white",
            backgroundColor: "currentColor",
            boxShadow: {
                default: THUMB_EDGE,
                ":focus-visible": `0 0 0 ${stroke.ring} ${color.ring}`,
            },
            forcedColorAdjust: "none",
        },
    },
    sliderThumbFill: (fill: string) => ({
        color: fill,
    }),
    swatch: {
        display: "inline-block",
        flexShrink: 0,
        width: space[5],
        height: space[5],
        borderRadius: radius[2],
        background: CHECKERS,
        overflow: "hidden",
        boxShadow: `inset 0 0 0 ${stroke.border} ${color.border}`,
        forcedColorAdjust: "none",
    },
    swatchFill: (fill: string) => ({
        backgroundColor: fill,
    }),
    swatchInside: {
        display: "block",
        width: "100%",
        height: "100%",
    },
});

/** The properties of an element of a color picker, the native element's attributes included. */
export type ColorElementProperties<Target extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of a color picker: its color, its form and its parts. */
export type ColorPickerProperties = ColorElementProperties &
    ColorPickerOptions & {
        /** The form field name it submits its color under. */
        readonly name?: string;
        /** The id of the form it belongs to, its ancestor form by default. */
        readonly form?: string;
    };

/** Render a color's area, hue and text, or parts of one's own around the same color. */
export function ColorPicker(given: ColorPickerProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onValueChange",
        read: (value) => readText(value) || undefined,
    });
    const control = new ColorPickerControl(properties);
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "onValueCommitted",
        "format",
        "disabled",
        "name",
        "form",
        "children",
    );

    return (
        <ColorPickerContext value={control}>
            {renderPart("div", "color-picker", rest, styles.picker, {
                role: "group" as const,
                get "data-disabled"() {
                    return control.isDisabled() ? "" : undefined;
                },
                get children() {
                    return (
                        <>
                            {contentOr(properties, () => (
                                <>
                                    <ColorArea />
                                    <ColorSlider channel="hue" />
                                    <ColorField />
                                </>
                            ))}
                            <Show when={properties.name}>
                                <input
                                    type="hidden"
                                    name={properties.name}
                                    form={properties.form}
                                    value={control.value()}
                                />
                            </Show>
                        </>
                    );
                },
            })}
        </ColorPickerContext>
    );
}

/** What a color area tells its thumb. */
interface ColorAreaThumb {
    /** Read its place along the width and the height. */
    readonly place: () => readonly [inline: string, block: string];
    /** Read the color that fills it. */
    readonly fill: () => string;
    /** Read the attributes of its input. */
    readonly attributes: () => JSX.InputHTMLAttributes<HTMLInputElement>;
    /** Take its input. */
    readonly take: (input: HTMLInputElement) => void;
    /** Set the channel along the width. */
    readonly set: (value: number) => void;
    /** Move it by a key. */
    readonly press: (event: KeyboardEvent) => void;
}

/** The nearest color area, null outside one. */
const ColorAreaContext = createContext<ColorAreaThumb | null>(null);

/** The properties of a color area. */
export type ColorAreaProperties = ColorElementProperties & {
    /** The space whose channels it shows, hue, saturation and brightness by default. */
    readonly colorSpace?: ColorSpace;
    /** The channel along the width, the second of the space by default. */
    readonly xChannel?: ColorChannel;
    /** The channel along the height, the third of the space by default, a lightness first. */
    readonly yChannel?: ColorChannel;
};

/** Render two channels of the nearest picker's color as an area, with a thumb to drag. */
export function ColorArea(properties: ColorAreaProperties): JSX.Element {
    // show the channels that are left once the hue is elsewhere
    const picker = useColorPicker();
    const locale = useLocale();
    const colorSpace = (): ColorSpace => properties.colorSpace ?? "hsb";
    const xChannel = (): ColorChannel =>
        properties.xChannel ??
        (colorSpace() === "oklch" ? "chroma" : COLOR_CHANNELS[colorSpace()][1]);
    const yChannel = (): ColorChannel =>
        properties.yChannel ??
        (colorSpace() === "oklch" ? "lightness" : COLOR_CHANNELS[colorSpace()][2]);
    const current = (): Color => picker.color(colorSpace());
    const share = (channel: ColorChannel): number => {
        const { min, max } = Color.range(channel);

        return (current().get(channel) - min) / (max - min);
    };
    const rest = omit(properties, "colorSpace", "xChannel", "yChannel", "children");

    // paint the colors of every point, anew as the channel left out changes
    const [canvas, setCanvas] = createSignal<HTMLCanvasElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const base = createMemo(
        () => current().with(xChannel(), 0).with(yChannel(), 0).with("alpha", 1),
        { equals: (one, other) => one.equals(other) },
    );
    createEffect(
        () => [canvas(), base(), xChannel(), yChannel()] as const,
        ([element, start, horizontal, vertical]) => paint(element, start, horizontal, vertical),
    );

    // move the thumb to a point, by a press, a drag or a key
    let area: HTMLElement | undefined;
    let input: HTMLInputElement | undefined;
    const moveTo = (x: number, y: number): void => {
        picker.set(
            current()
                .with(xChannel(), stepAt(xChannel(), x))
                .with(yChannel(), stepAt(yChannel(), y)),
        );
    };
    const follow = ({ clientX, clientY }: PointerEvent): void => {
        // measure from the side the text starts at, and up
        const box = area?.getBoundingClientRect();
        if (box !== undefined && box.width > 0 && box.height > 0) {
            const across = (clientX - box.left) / box.width;
            moveTo(
                locale.direction === "rtl" ? 1 - across : across,
                1 - (clientY - box.top) / box.height,
            );
        }
    };
    const drag = (event: PointerEvent & { readonly currentTarget: HTMLElement }): void => {
        // leave other buttons, and follow until the pointer lets go
        if (event.button !== 0 || picker.isDisabled()) {
            return;
        }
        event.preventDefault();
        const { currentTarget } = event;
        currentTarget.setPointerCapture(event.pointerId);
        input?.focus({ preventScroll: true });
        const before = current();
        follow(event);
        const listening = new AbortController();
        const { signal } = listening;
        currentTarget.addEventListener("pointermove", follow, { signal });
        currentTarget.addEventListener(
            "pointerup",
            () => {
                listening.abort();
                picker.commit();
            },
            { signal },
        );
        currentTarget.addEventListener(
            "pointercancel",
            () => {
                listening.abort();
                picker.set(before);
            },
            { signal },
        );
    };
    const press = (event: KeyboardEvent): void => {
        // step along the width or the height
        const isForward = locale.direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const isBackward = locale.direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        const steps = {
            [isForward]: [xChannel(), 1],
            [isBackward]: [xChannel(), -1],
            ArrowUp: [yChannel(), 1],
            ArrowDown: [yChannel(), -1],
            PageUp: [yChannel(), LARGE_STEP],
            PageDown: [yChannel(), -LARGE_STEP],
            Home: [xChannel(), -LARGE_STEP],
            End: [xChannel(), LARGE_STEP],
        } as const satisfies Record<string, readonly [ColorChannel, number]>;
        const found = Object.entries(steps).find(([key]) => key === event.key);
        if (found === undefined || picker.isDisabled()) {
            return;
        }
        event.preventDefault();
        const [channel, count] = found[1];
        const large = event.shiftKey && Math.abs(count) === 1 ? LARGE_STEP : 1;
        const { step } = Color.range(channel);
        picker.set(current().with(channel, current().get(channel) + count * large * step));
        picker.commit();
    };
    const words = (): string =>
        [xChannel(), yChannel()]
            .map(
                (channel) =>
                    `${locale.render(CHANNEL_NAMES[channel])} ${written(channel, current().get(channel), locale)}`,
            )
            .join(", ");
    const valueText = (): string => `${words()}, ${nameOf(current(), locale)}`;

    // tell the thumb its place, its color and its keys
    const thumb: ColorAreaThumb = {
        place: () => [`${String(share(xChannel()) * 100)}%`, `${String(share(yChannel()) * 100)}%`],
        fill: () => current().with("alpha", 1).toString(),
        attributes: () => ({
            "aria-valuetext": valueText(),
            ...Color.range(xChannel()),
            disabled: picker.isDisabled(),
            value: current().get(xChannel()),
        }),
        take: (element) => (input = element),
        set: (value) => picker.set(current().with(xChannel(), value)),
        press,
    };

    return (
        <ColorAreaContext value={thumb}>
            {renderPart(
                "div",
                "color-area",
                rest,
                styles.area,
                mergeProperties(
                    {
                        get "data-disabled"() {
                            return picker.isDisabled() ? "" : undefined;
                        },
                        get children() {
                            return (
                                <>
                                    <canvas
                                        ref={(element) => setCanvas(element)}
                                        width={POINTS}
                                        height={POINTS}
                                        aria-hidden="true"
                                        data-slot="color-area-paint"
                                        {...style.attrs(styles.paint)}
                                        style={{
                                            transform:
                                                locale.direction === "rtl"
                                                    ? "scaleX(-1)"
                                                    : undefined,
                                        }}
                                    />
                                    {contentOr(properties, () => (
                                        <ColorThumb />
                                    ))}
                                </>
                            );
                        },
                    },
                    {
                        role: "group" as const,
                        ref: (element: HTMLElement) => (area = element),
                        onPointerDown: drag,
                    },
                ),
            )}
        </ColorAreaContext>
    );
}

/** Render the thumb of the nearest color area, which the keys move both ways. */
export function ColorThumb(properties: ColorElementProperties): JSX.Element {
    // stand at the color, filled by it
    const thumb = useContext(ColorAreaContext);
    const locale = useLocale();
    if (thumb === null) {
        throw new TypeError("a color thumb needs a color area around it");
    }
    const rest = omit(properties, "children");

    return renderPart(
        "div",
        "color-thumb",
        rest,
        () => {
            const [inline, block] = thumb.place();

            return [styles.thumb, styles.thumbAt(inline, block, thumb.fill())];
        },
        {
            get children() {
                return (
                    <>
                        {properties.children}
                        <input
                            type="range"
                            ref={thumb.take}
                            aria-label={locale.render(t.context("label")`Color`)}
                            aria-roledescription={locale.render(t.context("title")`2D slider`)}
                            {...thumb.attributes()}
                            onInput={(event) => thumb.set(Number(event.currentTarget.value))}
                            onKeyDown={thumb.press}
                            {...style.attrs(styles.input)}
                        />
                    </>
                );
            },
        },
    );
}

/** The properties of a color slider, its slider's besides its values. */
export type ColorSliderProperties = Omit<
    SliderProperties,
    | "value"
    | "defaultValue"
    | "onValueChange"
    | "onValueCommitted"
    | "min"
    | "max"
    | "step"
    | "children"
> & {
    /** The channel it sets. */
    readonly channel: ColorChannel;
    /** The space of the channel, hue, saturation and brightness by default. */
    readonly colorSpace?: ColorSpace;
};

/** Render one channel of the nearest picker's color as a slider over the colors it sets. */
export function ColorSlider(properties: ColorSliderProperties): JSX.Element {
    // run through the channel's colors, the rest of the color kept
    const picker = useColorPicker();
    const locale = useLocale();
    const colorSpace = (): ColorSpace => properties.colorSpace ?? "hsb";
    const current = (): Color => picker.color(colorSpace());
    const isVertical = (): boolean => properties.orientation === "vertical";
    const base = (): Color => {
        // show a hue at its most vivid, and each channel without the alpha but its own
        const shown = properties.channel === "alpha" ? current() : current().with("alpha", 1);
        const isVivid = properties.channel === "hue" && colorSpace() !== "oklch";

        return isVivid ? new Color("hsb", [shown.get("hue"), 100, 100]) : shown;
    };
    const image = (): string => {
        // list the colors from one end of the channel to the other
        const { min, max } = Color.range(properties.channel);
        const start = base();
        const colors = Array.from({ length: STOPS + 1 }, (_, index) =>
            start.with(properties.channel, min + (index / STOPS) * (max - min)).toString(),
        );
        const isReversed = locale.direction === "rtl";
        const side = isVertical() ? "to top" : isReversed ? "to left" : "to right";

        return `linear-gradient(${side}, ${colors.join(", ")})`;
    };
    const rest = omit(properties, "channel", "colorSpace", "xstyle");

    return (
        <Slider
            data-slot="color-slider"
            aria-label={locale.render(CHANNEL_NAMES[properties.channel])}
            {...rest}
            min={Color.range(properties.channel).min}
            max={Color.range(properties.channel).max}
            step={Color.range(properties.channel).step}
            disabled={properties.disabled === true || picker.isDisabled()}
            value={[current().get(properties.channel)]}
            onValueChange={([value]) => {
                if (value !== undefined) {
                    picker.set(current().with(properties.channel, value));
                }
            }}
            onValueCommitted={() => picker.commit()}
            xstyle={[styles.slider, properties.xstyle]}
        >
            <SliderTrack xstyle={[styles.track, isVertical() && styles.vertical]}>
                <div {...style.attrs(styles.fill, styles.run(image()))} />
            </SliderTrack>
            <SliderThumb
                aria-valuetext={written(
                    properties.channel,
                    current().get(properties.channel),
                    locale,
                )}
                xstyle={[
                    styles.sliderThumb,
                    styles.sliderThumbFill(
                        base()
                            .with(properties.channel, current().get(properties.channel))
                            .toString(),
                    ),
                ]}
            />
        </Slider>
    );
}

/** The properties of a color field, its input's or its number field's included. */
export type ColorFieldProperties = Omit<InputProperties, "value" | "defaultValue" | "type"> & {
    /** The channel it sets as a number, the whole color as text by default. */
    readonly channel?: ColorChannel;
    /** The space of the channel, hue, saturation and brightness by default. */
    readonly colorSpace?: ColorSpace;
};

/** Render the nearest picker's color as text to edit, or one of its channels as a number. */
export function ColorField(properties: ColorFieldProperties): JSX.Element {
    const rest = omit(properties, "channel", "colorSpace");

    return (
        <Show when={properties.channel} fallback={<ColorTextField {...rest} />}>
            {(channel) => (
                <ColorNumberField
                    channel={channel()}
                    colorSpace={properties.colorSpace ?? "hsb"}
                    {...rest}
                />
            )}
        </Show>
    );
}

/** Render a color as text, read when a person leaves it or presses Enter. */
function ColorTextField(properties: Omit<InputProperties, "value">): JSX.Element {
    // take the text where it is a color, and write the color again else
    const picker = useColorPicker();
    const locale = useLocale();
    const take = (element: HTMLInputElement): void => {
        // read it with or without its mark
        const typed = element.value.trim();
        const read = Color.read(typed) ?? Color.read(`#${typed}`);
        if (read !== undefined && read.toString(picker.format()) !== picker.value()) {
            picker.set(read);
            picker.commit();
        }
        element.value = picker.value();
    };

    return (
        <Input
            data-slot="color-field"
            aria-label={locale.render(t.context("label")`Color`)}
            spellcheck={false}
            dir="ltr"
            autocomplete="off"
            disabled={picker.isDisabled()}
            {...mergeProperties(
                {
                    onBlur: (event: FocusEvent & { readonly currentTarget: HTMLInputElement }) =>
                        take(event.currentTarget),
                    onKeyDown: (
                        event: KeyboardEvent & { readonly currentTarget: HTMLInputElement },
                    ) => {
                        // take the text without sending the form
                        if (event.key === "Enter") {
                            event.preventDefault();
                            take(event.currentTarget);
                        }
                    },
                },
                properties,
            )}
            value={picker.value()}
        />
    );
}

/** Render one channel of a color as a number. */
function ColorNumberField(
    properties: Omit<InputProperties, "value"> & {
        /** The channel. */
        readonly channel: ColorChannel;
        /** The space of the channel. */
        readonly colorSpace: ColorSpace;
    },
): JSX.Element {
    // write the opacity as a share
    const picker = useColorPicker();
    const locale = useLocale();
    const current = (): Color => picker.color(properties.colorSpace);
    const range = () => Color.range(properties.channel);
    const rest = omit(properties, "channel", "colorSpace", "xstyle");

    return (
        <NumberField
            data-slot="color-field"
            min={range().min}
            max={range().max}
            step={range().step}
            format={formatOf(properties.channel)}
            disabled={picker.isDisabled()}
            value={Number(current().get(properties.channel).toFixed(3))}
            onValueChange={(value) => {
                if (value !== undefined && value !== null) {
                    picker.set(current().with(properties.channel, value));
                    picker.commit();
                }
            }}
        >
            <InputGroup xstyle={properties.xstyle}>
                <NumberFieldInput
                    aria-label={locale.render(CHANNEL_NAMES[properties.channel])}
                    {...rest}
                />
            </InputGroup>
        </NumberField>
    );
}

/** The properties of a swatch. */
export type ColorSwatchProperties = ColorElementProperties<HTMLSpanElement> & {
    /** The color it shows, the nearest picker's by default. */
    readonly color?: string;
};

/** Render a color, over squares that show through where it is see-through. */
export function ColorSwatch(properties: ColorSwatchProperties): JSX.Element {
    // show the nearest picker's color without one of its own
    const picker = useContext(ColorPickerContext);
    const locale = useLocale();
    const rest = omit(properties, "color");
    const shown = (): string | undefined => properties.color ?? picker?.value();

    return renderPart("span", "color-swatch", rest, styles.swatch, {
        role: "img" as const,
        get "aria-label"() {
            const read = Color.read(shown() ?? "");

            return read === undefined
                ? locale.render(t.context("label")`Color`)
                : nameOf(read, locale);
        },
        get children() {
            return (
                <span
                    {...style.attrs(
                        styles.swatchInside,
                        styles.swatchFill(shown() ?? "transparent"),
                    )}
                />
            );
        },
    });
}

/** What a browser offers to take a color from the screen, where it does. */
interface EyeDropperWindow {
    /** The browser's eye dropper. */
    readonly EyeDropper?: new () => { open: () => Promise<{ readonly sRGBHex: string }> };
}

/** Render a button that takes a color from the screen, where the browser offers to. */
export function ColorEyeDropper(properties: ButtonProperties): JSX.Element {
    // ask the browser once it is there
    const picker = useColorPicker();
    const locale = useLocale();
    const [Dropper, setDropper] = createSignal<EyeDropperWindow["EyeDropper"]>(undefined, {
        ownedWrite: true,
    });
    let button: HTMLElement | undefined;
    onSettled(() => {
        const window: (Window & EyeDropperWindow) | null | undefined =
            button?.ownerDocument.defaultView;
        setDropper(() => window?.EyeDropper);
    });
    const rest = merge(
        { variant: "outline" as const, isIconOnly: true },
        omit(properties, "children"),
    );
    const take = async (): Promise<void> => {
        // keep the color a person leaves without taking one
        const create = Dropper();
        const taken =
            create === undefined ? undefined : await new create().open().catch(() => undefined);
        if (taken !== undefined) {
            picker.set(Color.parse(taken.sRGBHex));
            picker.commit();
        }
    };

    return (
        <span ref={(element) => (button = element)} style={{ display: "contents" }}>
            <Show when={Dropper()}>
                <Button
                    data-slot="color-eye-dropper"
                    aria-label={locale.render(t`Take a color from the screen`)}
                    disabled={picker.isDisabled()}
                    {...mergeProperties({ onClick: () => void take() }, rest)}
                >
                    {contentOr(properties, () => (
                        <Icon icon={eyedropperIcon} />
                    ))}
                </Button>
            </Show>
        </span>
    );
}

/** Paint the colors of an area, two channels of a color along its sides. */
function paint(
    canvas: HTMLCanvasElement | undefined,
    base: Color,
    horizontal: ColorChannel,
    vertical: ColorChannel,
): void {
    // leave a screen that cannot paint
    const context = canvas?.getContext("2d");
    if (context === null || context === undefined) {
        return;
    }
    const image = context.createImageData(POINTS, POINTS);
    const along = Color.range(horizontal);
    const up = Color.range(vertical);
    for (let row = 0; row < POINTS; row += 1) {
        const line = base.with(vertical, up.max - (row / (POINTS - 1)) * (up.max - up.min));
        for (let column = 0; column < POINTS; column += 1) {
            const value = along.min + (column / (POINTS - 1)) * (along.max - along.min);
            const rgb = line.with(horizontal, value).to("rgb");
            image.data.set([...rgb.values, 255], (row * POINTS + column) * 4);
        }
    }
    context.putImageData(image, 0, 0);
}

/** Read the step of a channel at a share of its range. */
function stepAt(channel: ColorChannel, share: number): number {
    const { min, max, step } = Color.range(channel);

    return Math.round((min + Math.min(Math.max(share, 0), 1) * (max - min)) / step) * step;
}

/** Read how a channel is written. */
function formatOf(channel: ColorChannel): NonNullable<NumberFieldProperties["format"]> {
    return channel === "alpha"
        ? { style: "percent" }
        : { maximumFractionDigits: channel === "chroma" ? 3 : 0 };
}

/** Write a channel's value. */
function written(
    channel: ColorChannel,
    value: number,
    locale: ReturnType<typeof useLocale>,
): string {
    return locale.number(value, formatOf(channel));
}
