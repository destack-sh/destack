import { field } from "@destack/object";
import { ORIENTATION } from "../position/index.ts";
import { type Localization, type NumberFormat, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    swatch,
} from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createSignal,
    flush,
    createControllableSignal,
    For,
    type JSX,
    omit,
    Repeat,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { type FieldControl, useFieldControl, useFieldValue } from "../field/control.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { CollectionBuilder } from "../collection/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The range a slider covers without its own minimum and maximum, the native default. */
const RANGE = { min: 0, max: 100 };

/** The steps Page Up, Page Down and Shift with an arrow take, as Base UI's. */
const LARGE_STEP = 10;

/** The room around a thumb that takes presses, as tall as a finger needs. */
const TOUCH_SIZE = size[1];

/** The ring a focused thumb shows. */
const FOCUS_RING = `0 0 0 ${stroke.ring} ${color.ring}`;

/** The width and height of a thumb. */
const THUMB_SIZE = space[4];

/** The look of a thumb in every engine. */
const THUMB = {
    boxSizing: "border-box",
    width: space[4],
    height: space[4],
    borderStyle: "solid",
    borderWidth: stroke.border,
    borderColor: { default: swatch.accentMark, [media.forcedColors]: "Highlight" },
    borderRadius: radius.full,
    backgroundColor: {
        default: "white",
        [media.forcedColors]: { default: "Highlight", ":focus-visible": "CanvasText" },
    },
    forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
    cursor: "inherit",
    transitionProperty: "box-shadow",
    transitionDuration: motion.durationShort,
    transitionTimingFunction: motion.easingStandard,
} as const;

/** The properties people change in a slider's controls and inspector. */
const SLIDER_PROPERTIES = {
    orientation: ORIENTATION.described(t`Whether the track runs across or up.`).default(
        "horizontal",
    ),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(
            t`Refuses the thumbs where the value cannot change now, as its field does when absent.`,
        )
        .optional(),
    readOnly: field
        .boolean()
        .labeled(t.context("label")`Read only`)
        .described(t`Shows the values, which a person cannot change.`)
        .default(false),
};

/** The styles of a slider, its track, range, thumbs and marks. */
const styles = style.create({
    slider: {
        position: "relative",
        display: "grid",
        alignItems: "center",
        justifyItems: "stretch",
        width: "100%",
        height: space[4],
        opacity: { default: 1, ":is([data-disabled])": 0.5 },
        cursor: {
            default: "pointer",
            ":is([data-dragging])": "grabbing",
            ":is([data-disabled])": "not-allowed",
            ":is([data-readonly])": "default",
        },
        touchAction: "pan-y",
        userSelect: "none",
        // take presses as tall as a finger
        "::before": {
            content: "''",
            position: "absolute",
            insetInline: 0,
            insetBlock: `min(0px, (100% - ${TOUCH_SIZE}) / 2)`,
        },
    },
    vertical: {
        justifyItems: "center",
        alignItems: "stretch",
        width: space[4],
        height: `calc(4 * ${size[3]})`,
        touchAction: "pan-x",
        "::before": {
            content: "''",
            position: "absolute",
            insetInline: `min(0px, (100% - ${TOUCH_SIZE}) / 2)`,
            insetBlock: 0,
        },
    },
    track: {
        position: "relative",
        gridArea: "1 / 1",
        height: space[1],
        overflow: "hidden",
        borderRadius: radius.full,
        backgroundColor: { default: color.border, [media.forcedColors]: "CanvasText" },
    },
    verticalTrack: {
        width: space[1],
        height: "auto",
    },
    range: {
        position: "absolute",
        backgroundColor: { default: swatch.accentMark, [media.forcedColors]: "Highlight" },
    },
    span: (from: string, to: string) => ({
        insetBlock: 0,
        insetInlineStart: from,
        insetInlineEnd: to,
    }),
    verticalSpan: (from: string, to: string) => ({
        insetInline: 0,
        insetBlockEnd: from,
        insetBlockStart: to,
    }),
    thumb: {
        position: "relative",
        gridArea: "1 / 1",
        appearance: "none",
        width: "100%",
        height: space[4],
        pointerEvents: "none",
        outlineStyle: "none",
        margin: 0,
        "::-webkit-slider-runnable-track": { backgroundColor: "transparent" },
        "::-moz-range-track": { backgroundColor: "transparent" },
        "::-webkit-slider-thumb": {
            ...THUMB,
            appearance: "none",
            boxShadow: { default: shadow.raised, ":focus-visible": FOCUS_RING },
        },
        "::-moz-range-thumb": {
            ...THUMB,
            boxShadow: { default: shadow.raised, ":focus-visible": FOCUS_RING },
        },
    },
    verticalThumb: {
        width: space[4],
        height: "100%",
        writingMode: "vertical-lr",
        direction: "rtl",
    },
    marks: {
        gridArea: "1 / 1",
        position: "relative",
        alignSelf: "stretch",
        pointerEvents: "none",
    },
    mark: {
        position: "absolute",
        width: space[1],
        height: space[1],
        borderRadius: radius.full,
        backgroundColor: { default: color.mutedForeground, [media.forcedColors]: "CanvasText" },
        marginInlineStart: `calc(${space[1]} / -2)`,
        marginBlockStart: `calc(${space[1]} / -2)`,
    },
    label: {
        position: "absolute",
        insetBlockStart: space[3],
        insetInlineStart: "50%",
        translate: "-50% 0",
        color: color.mutedForeground,
        whiteSpace: "nowrap",
        fontVariantNumeric: "tabular-nums",
    },
    verticalLabel: {
        insetBlockStart: "50%",
        insetInlineStart: space[3],
        translate: "0 -50%",
    },
    value: {
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    markAt: (inline: string, block: string) => ({
        insetInlineStart: inline,
        insetBlockStart: block,
    }),
});

/** The values, range and thumbs of a slider, which its track, range and thumbs share. */
export class SliderControl {
    /** The slider's properties, read for its range, step, form and handlers. */
    readonly properties: RenderProperties<SliderAttributes, typeof SLIDER_PROPERTIES>;
    /** The field the slider's first thumb belongs to, null outside one. */
    readonly field: FieldControl | null;
    /** The values of the thumbs. */
    readonly values: Accessor<readonly number[]>;
    /** The thumb a pointer drags, undefined without one. */
    readonly dragged: Accessor<number | undefined>;
    /** The thumbs in document order, by their elements. */
    readonly #thumbs: CollectionBuilder<Accessor<HTMLInputElement | undefined>>;
    /** Set the thumb a pointer drags. */
    readonly #setDragged: (index: number | undefined) => void;
    /** The reader's locale. */
    readonly #locale: Localization;
    /** Replace the values and tell the change handler. */
    readonly #setValues: (values: readonly number[]) => void;

    /** Create the state of a slider from its root's properties. */
    constructor(
        properties: RenderProperties<SliderAttributes, typeof SLIDER_PROPERTIES>,
        fieldControl: FieldControl | null,
        locale: Localization,
    ) {
        // follow the controlled values or the slider's own, the middle of the range by default
        const middle = untrack(
            () => ((properties.min ?? RANGE.min) + (properties.max ?? RANGE.max)) / 2,
        );
        const [values, setValues] = createControllableSignal<readonly number[]>({
            isControlled: () => properties.value !== undefined,
            value: () => properties.value ?? [],
            defaultValue: properties.defaultValue ?? [middle],
            onChange: (next) => properties.onValueChange?.(next),
        });
        this.properties = properties;
        this.field = fieldControl;
        this.values = values;
        this.#thumbs = new CollectionBuilder((element) => element());
        this.#setValues = setValues;
        this.#locale = locale;
        const [dragged, setDragged] = createSignal<number | undefined>(undefined, {
            ownedWrite: true,
        });
        this.dragged = dragged;
        this.#setDragged = (index) => void setDragged(index);
    }

    /** The lowest value. */
    min(): number {
        return this.properties.min ?? RANGE.min;
    }

    /** The highest value. */
    max(): number {
        return this.properties.max ?? RANGE.max;
    }

    /** Read a thumb's value, refusing a thumb the values have none for. */
    valueAt(index: number): number {
        const value = this.values()[index];
        if (value === undefined) {
            throw new RangeError(`a slider has no value for thumb '${String(index)}'`);
        }

        return value;
    }

    /** Report whether the track runs upright. */
    isVertical(): boolean {
        return this.properties.orientation === "vertical";
    }

    /** Read a value's share of the range as a percentage. */
    share(value: number): number {
        return ((value - this.min()) / (this.max() - this.min())) * 100;
    }

    /** Read how far from a side of the track a thumb's center sits for a value. */
    offset(value: number, side: "start" | "end"): string {
        // travel between the track's ends by the thumb's own width less
        const share = side === "start" ? this.share(value) : 100 - this.share(value);

        return `calc(${THUMB_SIZE} / 2 + (100% - ${THUMB_SIZE}) * ${share / 100})`;
    }

    /** Add a thumb until it unmounts, returning its place among the thumbs. */
    addThumb(element: Accessor<HTMLInputElement | undefined>): Accessor<number> {
        return this.#thumbs.add(element);
    }

    /** The step between values. */
    step(): number {
        return this.properties.step ?? 1;
    }

    /** Report whether the slider refuses the person, by its own value or its field's. */
    isDisabled(): boolean {
        return this.properties.disabled === true || this.field?.isDisabled() === true;
    }

    /** Report whether the person moves the thumbs. */
    isEditable(): boolean {
        return !this.isDisabled() && !this.properties.readOnly;
    }

    /** Write a value as its format says. */
    write(value: number): string {
        return this.#locale.number(value, this.properties.format);
    }

    /** Set a thumb's value on its step, between its neighbours. */
    set(index: number, value: number): void {
        // keep the least steps to each neighbour
        const current = this.values();
        const gap = (this.properties.minStepsBetweenValues ?? 0) * this.step();
        const before = current[index - 1];
        const after = current[index + 1];
        const lowest = before === undefined ? this.min() : before + gap;
        const highest = after === undefined ? this.max() : after - gap;
        const next = Math.min(Math.max(this.#snap(value), lowest), highest);
        if (next !== current[index]) {
            this.#setValues(current.map((entry, place) => (place === index ? next : entry)));
        }
    }

    /** Move a thumb to its input's value, and show the owner's until the owner takes it. */
    move(index: number, input: HTMLInputElement): void {
        if (this.isEditable()) {
            this.set(index, Number(input.value));
        }
        flush();
        input.value = String(this.valueAt(index));
    }

    /** Take a thumb's large steps from its keys, as a native input's differ among browsers. */
    press(index: number, event: KeyboardEvent): void {
        // step towards the end by Page Up and Shift with up or right
        const large = this.properties.largeStep ?? LARGE_STEP;
        const isForward = this.#locale.direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const isBackward = this.#locale.direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        const isUp = event.key === "PageUp" || (event.shiftKey && event.key === "ArrowUp");
        const isDown = event.key === "PageDown" || (event.shiftKey && event.key === "ArrowDown");
        const steps =
            isUp || (event.shiftKey && event.key === isForward)
                ? large
                : isDown || (event.shiftKey && event.key === isBackward)
                  ? -large
                  : 0;
        if (steps === 0 || !this.isEditable()) {
            return;
        }
        event.preventDefault();
        this.set(index, this.valueAt(index) + steps * this.step());
        this.commit();
    }

    /** Tell the owner the person settled on the values. */
    commit(): void {
        // read them as just set
        flush();
        this.properties.onValueCommitted?.(this.values());
    }

    /** Drag the thumb nearest a press, which moves there at once. */
    drag(event: PointerEvent & { readonly currentTarget: HTMLElement }): void {
        // leave other buttons and a slider a person cannot move
        if (event.button !== 0 || !this.isEditable()) {
            return;
        }
        const slider = event.currentTarget;
        const value = this.#valueAtPoint(slider, event);
        const index = this.#nearest(value);
        event.preventDefault();
        slider.setPointerCapture(event.pointerId);
        this.#thumbs.items()[index]?.()?.focus({ preventScroll: true, focusVisible: false });
        const before = this.values();
        this.#setDragged(index);
        this.set(index, value);

        // follow the pointer until it lets go
        const listening = new AbortController();
        const { signal } = listening;
        const follow = (step: PointerEvent) => this.set(index, this.#valueAtPoint(slider, step));
        const end = () => {
            listening.abort();
            this.#setDragged(undefined);
            this.commit();
        };
        slider.addEventListener("pointermove", follow, { signal });
        slider.addEventListener("pointerup", end, { signal });
        slider.addEventListener(
            "pointercancel",
            () => {
                listening.abort();
                this.#setDragged(undefined);
                this.#setValues(before);
            },
            { signal },
        );
    }

    /** Round a value to the step from the lowest one. */
    #snap(value: number): number {
        // round away what floating point adds to a decimal step
        const step = this.step();
        const steps = Math.round((value - this.min()) / step);
        const decimals = Math.max(decimalsOf(step), decimalsOf(this.min()));

        return Number((this.min() + steps * step).toFixed(decimals));
    }

    /** Read the value under a pointer along the track. */
    #valueAtPoint(slider: HTMLElement, { clientX, clientY }: PointerEvent): number {
        // measure between the thumbs' places at both ends
        const box = slider.getBoundingClientRect();
        const thumb = Math.min(box.width, box.height);
        const isReversed = this.#locale.direction === "rtl";
        const across = isReversed ? box.right - clientX : clientX - box.left;
        const along = this.isVertical() ? box.bottom - clientY : across;
        const length = (this.isVertical() ? box.height : box.width) - thumb;
        const share = length <= 0 ? 0 : (along - thumb / 2) / length;

        return this.min() + Math.min(Math.max(share, 0), 1) * (this.max() - this.min());
    }

    /** Find the thumb nearest a value, the one that can move there among those on one place. */
    #nearest(value: number): number {
        // compare how far each is, the later one where the value lies past both
        const distances = this.values().map((entry) => Math.abs(entry - value));
        const least = Math.min(...distances);
        const first = distances.indexOf(least);
        const last = distances.lastIndexOf(least);

        return value > (this.values()[first] ?? value) ? last : first;
    }
}

/** Count the decimals a number is written with. */
function decimalsOf(value: number): number {
    const [, fraction = ""] = String(value).split(".");

    return fraction.length;
}

/** The nearest slider, null outside one. */
const SliderContext = createContext<SliderControl | null>(null);

/** The properties of a slider besides its declared ones: its values, range and form. */
export interface SliderAttributes extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onInput" | "onChange"
> {
    /** The values of the thumbs, one for a value and two for a range. */
    readonly value?: readonly number[];
    /** The values of the thumbs at first while uncontrolled, the middle of the range by default. */
    readonly defaultValue?: readonly number[];
    /** Handle each change of the values while a thumb moves. */
    readonly onValueChange?: (value: readonly number[]) => void;
    /** Handle the values the person settles on when a thumb is released. */
    readonly onValueCommitted?: (value: readonly number[]) => void;
    /** The lowest value, 0 by default. */
    readonly min?: number;
    /** The highest value, 100 by default. */
    readonly max?: number;
    /** The step between values, 1 by default. */
    readonly step?: number;
    /** The steps Page Up, Page Down and Shift with an arrow take, 10 by default. */
    readonly largeStep?: number;
    /** The least steps between two thumbs, none by default. */
    readonly minStepsBetweenValues?: number;
    /** How values are written and read aloud, as plain numbers by default. */
    readonly format?: NumberFormat;
    /** The values marked along the track. */
    readonly marks?: readonly number[];
    /** The form field name each thumb submits its value under. */
    readonly name?: string;
    /** The id of the form the thumbs belong to, their ancestor form by default. */
    readonly form?: string;
    /** The StyleX styles applied after the slider's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a slider, the native element's attributes included. */
export type SliderElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** The properties of a slider's thumb, the native range input's attributes included. */
export interface SliderThumbProperties extends SliderElementProperties<
    Omit<
        JSX.InputHTMLAttributes<HTMLInputElement>,
        "type" | "value" | "min" | "max" | "step" | "onInput" | "onChange"
    >
> {
    /** The value of the slider the thumb sets, its place among the thumbs by default. */
    readonly index?: number;
}

/** Render a slider around its track and thumbs, with a native range input per thumb. */
export const Slider = defineComponent({
    name: "slider",
    properties: SLIDER_PROPERTIES,
    render(given: RenderProperties<SliderAttributes, typeof SLIDER_PROPERTIES>): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "value",
            change: "onValueChange",
            read: (value) => (typeof value === "number" ? [value] : value),
            take: (change, current) => {
                const first: unknown = Array.isArray(change) ? change[0] : change;

                return typeof current === "number" ? first : change;
            },
        });

        // share the values with the parts
        const control = new SliderControl(properties, useFieldControl(), useLocale());
        const rest = omit(
            properties,
            "value",
            "defaultValue",
            "onValueChange",
            "onValueCommitted",
            "min",
            "max",
            "step",
            "largeStep",
            "minStepsBetweenValues",
            "format",
            "marks",
            "orientation",
            "disabled",
            "readOnly",
            "name",
            "form",
            "children",
        );

        return (
            <SliderContext value={control}>
                {renderPart(
                    "div",
                    "slider",
                    rest,
                    () => [styles.slider, control.isVertical() && styles.vertical],
                    mergeProperties(
                        {
                            role: "group" as const,
                            get "data-orientation"() {
                                return properties.orientation;
                            },
                            get "data-disabled"() {
                                return control.isDisabled() ? "" : undefined;
                            },
                            get "data-readonly"() {
                                return properties.readOnly ? "" : undefined;
                            },
                            get "data-dragging"() {
                                return control.dragged() === undefined ? undefined : "";
                            },
                            get children() {
                                return contentOr(properties, () => (
                                    <>
                                        <SliderTrack>
                                            <SliderIndicator />
                                        </SliderTrack>
                                        <SliderMarks />
                                        <Repeat count={control.values().length}>
                                            {(index) => <SliderThumb index={index} />}
                                        </Repeat>
                                    </>
                                ));
                            },
                        },
                        {
                            onPointerDown: (
                                event: PointerEvent & { readonly currentTarget: HTMLElement },
                            ) => control.drag(event),
                        },
                    ),
                )}
            </SliderContext>
        );
    },
});

/** The properties of a slider, its values, range and look included. */
export type SliderProperties = ComponentProperties<typeof Slider>;

/** Render the values of the nearest slider as they are written, joined as a range. */
export function SliderValue(
    properties: Omit<SliderElementProperties<JSX.HTMLAttributes<HTMLOutputElement>>, "children"> & {
        /** Render the values, as a range by default. */
        readonly children?: (written: readonly string[], values: readonly number[]) => JSX.Element;
    },
): JSX.Element {
    // write each value
    const control = useSlider();
    const written = (): string[] => control.values().map((value) => control.write(value));
    const rest = omit(properties, "children");

    return renderPart("output", "slider-value", rest, [text.sm, styles.value], {
        "aria-live": "off" as const,
        get children() {
            return properties.children?.(written(), control.values()) ?? written().join(" – ");
        },
    });
}

/** Render the marks of the nearest slider, between its track and its thumbs. */
export function SliderMarks(
    properties: Omit<
        SliderElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
        "children" | "render"
    > & {
        /** Render a mark's label below it, none by default. */
        readonly children?: (mark: number) => JSX.Element;
    },
): JSX.Element {
    const control = useSlider();
    const rest = omit(properties, "children");

    return (
        <Show when={control.properties.marks}>
            {(marks) =>
                renderPart("div", "slider-marks", rest, styles.marks, {
                    get children() {
                        return (
                            <For each={marks()}>
                                {(mark) => (
                                    <span
                                        data-slot="slider-mark"
                                        {...style.attrs(
                                            styles.mark,
                                            control.isVertical()
                                                ? styles.markAt("50%", control.offset(mark, "end"))
                                                : styles.markAt(
                                                      control.offset(mark, "start"),
                                                      "50%",
                                                  ),
                                        )}
                                    >
                                        <Show when={properties.children?.(mark)}>
                                            {(label) => (
                                                <span
                                                    {...style.attrs(
                                                        text.xs,
                                                        styles.label,
                                                        control.isVertical() &&
                                                            styles.verticalLabel,
                                                    )}
                                                >
                                                    {label()}
                                                </span>
                                            )}
                                        </Show>
                                    </span>
                                )}
                            </For>
                        );
                    },
                })
            }
        </Show>
    );
}

/** Render the track of the nearest slider, around its range. */
export function SliderTrack(
    properties: SliderElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useSlider();

    return renderPart(
        "div",
        "slider-track",
        properties,
        () => [styles.track, control.isVertical() && styles.verticalTrack],
        {
            get "data-orientation"() {
                return control.properties.orientation;
            },
            get "data-disabled"() {
                return control.isDisabled() ? "" : undefined;
            },
        },
    );
}

/** Render the filled span of the nearest slider's track. */
export function SliderIndicator(
    properties: SliderElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // span from the lower thumb or the start up to the highest thumb
    const control = useSlider();
    const from = (): string => {
        const values = control.values();

        return values.length > 1 ? control.offset(values[0] ?? control.min(), "start") : "0%";
    };
    const to = (): string => control.offset(control.values().at(-1) ?? control.min(), "end");

    return renderPart(
        "div",
        "slider-indicator",
        properties,
        () => [
            styles.range,
            control.isVertical() ? styles.verticalSpan(from(), to()) : styles.span(from(), to()),
        ],
        {
            get "data-orientation"() {
                return control.properties.orientation;
            },
            get "data-disabled"() {
                return control.isDisabled() ? "" : undefined;
            },
        },
    );
}

/** Render a thumb of the nearest slider as a native range input, between its neighbours. */
export function SliderThumb(properties: SliderThumbProperties): JSX.Element {
    // take the thumb's place, label a range's thumbs by the end they set and one thumb as its slider
    const control = useSlider();
    const locale = useLocale();
    let element: HTMLInputElement | undefined;
    const place = control.addThumb(() => element);
    const index = (): number => properties.index ?? place();
    const rest = omit(properties, "index", "xstyle", "style");
    const isRange = () => control.values().length === 2;

    return (
        <input
            type="range"
            data-slot="slider-thumb"
            data-orientation={control.properties.orientation}
            data-disabled={control.isDisabled() ? "" : undefined}
            data-dragging={control.dragged() === index() ? "" : undefined}
            min={control.min()}
            max={control.max()}
            step={control.properties.step}
            name={control.properties.name}
            form={control.properties.form}
            disabled={control.isDisabled()}
            aria-label={
                isRange()
                    ? locale.render(
                          index() === 0 ? t.context("label")`Minimum` : t.context("label")`Maximum`,
                      )
                    : control.properties["aria-label"]
            }
            aria-labelledby={isRange() ? undefined : control.properties["aria-labelledby"]}
            aria-orientation={control.properties.orientation}
            aria-valuetext={
                control.properties.format === undefined
                    ? undefined
                    : control.write(control.valueAt(index()))
            }
            aria-readonly={control.properties.readOnly ? "true" : undefined}
            {...(index() === 0 ? control.field?.attributes() : undefined)}
            {...mergeProperties(
                {
                    ref: (created: HTMLInputElement) => (element = created),
                    onKeyDown: (event: KeyboardEvent) => control.press(index(), event),
                },
                rest,
            )}
            value={control.valueAt(index())}
            onInput={(event) => control.move(index(), event.currentTarget)}
            onChange={() => control.commit()}
            {...style.attributes(
                [styles.thumb, control.isVertical() && styles.verticalThumb, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Read the nearest slider, refusing a part outside one. */
export function useSlider(): SliderControl {
    const control = useContext(SliderContext);
    if (control === null) {
        throw new TypeError("a slider part needs a slider around it");
    }

    return control;
}
