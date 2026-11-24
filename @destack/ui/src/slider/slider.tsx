import type { Axis } from "../position/index.ts";
import { t } from "@destack/locale";
import * as style from "@destack/style";
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
    createControllableSignal,
    For,
    type JSX,
    omit,
    Repeat,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { type FieldControl, useFieldControl } from "../field/control.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import { CollectionBuilder } from "../collection/index.ts";

/** The range a slider covers without its own minimum and maximum, the native default. */
const RANGE = { min: 0, max: 100 };

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
    borderColor: swatch.accentMark,
    borderRadius: radius.full,
    backgroundColor: "white",
    pointerEvents: "auto",
    cursor: "pointer",
    transitionProperty: "box-shadow",
    transitionDuration: motion.durationShort,
} as const;

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
    },
    vertical: {
        justifyItems: "center",
        alignItems: "stretch",
        width: space[4],
        height: `calc(4 * ${size[3]})`,
    },
    track: {
        position: "relative",
        gridArea: "1 / 1",
        height: space[1],
        overflow: "hidden",
        borderRadius: radius.full,
        backgroundColor: color.border,
    },
    verticalTrack: {
        width: space[1],
        height: "auto",
    },
    range: {
        position: "absolute",
        backgroundColor: swatch.accentMark,
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
        cursor: { default: "pointer", ":disabled": "not-allowed" },
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
        backgroundColor: color.mutedForeground,
        marginInlineStart: `calc(${space[1]} / -2)`,
        marginBlockStart: `calc(${space[1]} / -2)`,
    },
    markAt: (inline: string, block: string) => ({
        insetInlineStart: inline,
        insetBlockStart: block,
    }),
});

/** The values, range and thumbs of a slider, which its track, range and thumbs share. */
export class SliderControl {
    /** The slider's properties, read for its range, step, form and handlers. */
    readonly properties: SliderProperties;
    /** The field the slider's first thumb belongs to, null outside one. */
    readonly field: FieldControl | null;
    /** The values of the thumbs. */
    readonly values: Accessor<readonly number[]>;
    /** The thumbs in document order, by their elements. */
    readonly #thumbs: CollectionBuilder<Accessor<Element | undefined>>;
    /** Replace the values and tell the change handler. */
    readonly #setValues: (values: readonly number[]) => void;

    /** Create the state of a slider from its root's properties. */
    constructor(properties: SliderProperties, field: FieldControl | null) {
        // follow the controlled values or the slider's own, the middle of the range by default
        const middle = ((properties.min ?? RANGE.min) + (properties.max ?? RANGE.max)) / 2;
        const [values, setValues] = createControllableSignal<readonly number[]>({
            isControlled: () => properties.value !== undefined,
            value: () => properties.value ?? [],
            defaultValue: properties.defaultValue ?? [middle],
            onChange: (next) => properties.onValueChange?.(next),
        });
        this.properties = properties;
        this.field = field;
        this.values = values;
        this.#thumbs = new CollectionBuilder((element) => element());
        this.#setValues = setValues;
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
    addThumb(element: Accessor<Element | undefined>): Accessor<number> {
        return this.#thumbs.add(element);
    }

    /** Move a thumb between its neighbours and show the owner's value until the owner takes it. */
    move(index: number, input: HTMLInputElement): void {
        // clamp the value between the neighbouring thumbs
        const current = this.values();
        const lowest = current[index - 1] ?? this.min();
        const highest = current[index + 1] ?? this.max();
        const value = Math.min(Math.max(Number(input.value), lowest), highest);
        this.#setValues(current.map((entry, position) => (position === index ? value : entry)));
        input.value = String(this.properties.value?.[index] ?? value);
    }
}

/** The nearest slider, null outside one. */
const SliderContext = createContext<SliderControl | null>(null);

/** The properties of a slider: its values, range and look. */
export interface SliderProperties extends Omit<
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
    /** The values marked along the track. */
    readonly marks?: readonly number[];
    /** The direction the track runs along, horizontal by default. */
    readonly orientation?: Axis;
    /** Whether the thumbs ignore the person. */
    readonly disabled?: boolean;
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
export function Slider(properties: SliderProperties): JSX.Element {
    // share the values with the parts
    const control = new SliderControl(properties, useFieldControl());
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "onValueCommitted",
        "min",
        "max",
        "step",
        "marks",
        "orientation",
        "disabled",
        "name",
        "form",
        "xstyle",
        "style",
        "children",
    );

    return (
        <SliderContext value={control}>
            <div
                role="group"
                data-slot="slider"
                data-orientation={properties.orientation ?? "horizontal"}
                data-disabled={properties.disabled === true ? "" : undefined}
                {...rest}
                {...style.attributes(
                    [styles.slider, control.isVertical() && styles.vertical, properties.xstyle],
                    properties.style,
                )}
            >
                {properties.children ?? (
                    <>
                        <SliderTrack>
                            <SliderIndicator />
                        </SliderTrack>
                        <SliderMarks />
                        <Repeat count={control.values().length}>
                            {(index) => <SliderThumb index={index} />}
                        </Repeat>
                    </>
                )}
            </div>
        </SliderContext>
    );
}

/** Render the marks of the nearest slider, between its track and its thumbs. */
export function SliderMarks(): JSX.Element {
    const control = useSlider();

    return (
        <Show when={control.properties.marks}>
            {(marks) => (
                <div data-slot="slider-marks" {...style.attrs(styles.marks)}>
                    <For each={marks()}>
                        {(mark) => (
                            <span
                                data-slot="slider-mark"
                                {...style.attrs(
                                    styles.mark,
                                    control.isVertical()
                                        ? styles.markAt("50%", control.offset(mark, "end"))
                                        : styles.markAt(control.offset(mark, "start"), "50%"),
                                )}
                            />
                        )}
                    </For>
                </div>
            )}
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
                return control.properties.orientation ?? "horizontal";
            },
            get "data-disabled"() {
                return control.properties.disabled === true ? "" : undefined;
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
                return control.properties.orientation ?? "horizontal";
            },
            get "data-disabled"() {
                return control.properties.disabled === true ? "" : undefined;
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
            data-orientation={control.properties.orientation ?? "horizontal"}
            data-disabled={control.properties.disabled === true ? "" : undefined}
            min={control.min()}
            max={control.max()}
            step={control.properties.step}
            name={control.properties.name}
            form={control.properties.form}
            disabled={control.properties.disabled}
            aria-label={
                isRange()
                    ? locale.render(index() === 0 ? t`Minimum` : t`Maximum`)
                    : control.properties["aria-label"]
            }
            aria-labelledby={isRange() ? undefined : control.properties["aria-labelledby"]}
            aria-orientation={control.properties.orientation}
            {...(index() === 0 ? control.field?.attributes() : undefined)}
            {...mergeProperties({ ref: (created: HTMLInputElement) => (element = created) }, rest)}
            value={control.valueAt(index())}
            onInput={(event) => control.move(index(), event.currentTarget)}
            onChange={() => control.properties.onValueCommitted?.(control.values())}
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
