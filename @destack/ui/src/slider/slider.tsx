import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createSignal,
    createUniqueId,
    For,
    type JSX,
    omit,
    onCleanup,
    Repeat,
    type Setter,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { type FieldControl, useFieldControl } from "../field/control.ts";

/** The range a slider covers without its own minimum and maximum, the native default. */
const RANGE = { min: 0, max: 100 };

/** The ring a focused thumb shows. */
const FOCUS_RING = `0 0 0 ${stroke.ring} color-mix(in oklab, ${color.ring} 50%, transparent)`;

/** The look of a thumb in every engine. */
const THUMB = {
    boxSizing: "border-box",
    width: space[4],
    height: space[4],
    borderStyle: "solid",
    borderWidth: stroke.border,
    borderColor: color.primary,
    borderRadius: radius.full,
    backgroundColor: color.background,
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
        backgroundColor: color.muted,
    },
    verticalTrack: {
        width: space[1],
        height: "auto",
    },
    range: {
        position: "absolute",
        backgroundColor: color.primary,
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
        gridArea: "1 / 1",
        appearance: "none",
        width: "100%",
        height: space[4],
        margin: 0,
        backgroundColor: "transparent",
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
        translate: "-50% -50%",
    },
    markAt: (inline: string, block: string) => ({
        insetInlineStart: inline,
        insetBlockStart: block,
    }),
});

/** The values, range and thumbs of a slider, which its track, range and thumbs share. */
class SliderControl {
    /** The slider's properties, read for its range, step, form and handlers. */
    readonly properties: SliderProperties;
    /** The field the slider's first thumb belongs to, null outside one. */
    readonly field: FieldControl | null;
    /** The values of the thumbs. */
    readonly values: Accessor<readonly number[]>;
    /** The ids of the thumbs in the order they joined. */
    readonly #thumbs: Accessor<readonly string[]>;
    /** Replace the values and tell the change handler. */
    readonly #setValues: (values: readonly number[]) => void;
    /** Replace the ids of the thumbs. */
    readonly #setThumbs: Setter<readonly string[]>;

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
        const [thumbs, setThumbs] = createSignal<readonly string[]>([], { ownedWrite: true });
        this.properties = properties;
        this.field = field;
        this.values = values;
        this.#thumbs = thumbs;
        this.#setValues = setValues;
        this.#setThumbs = setThumbs;
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
            throw new RangeError(`a slider has no value for thumb ${String(index)}`);
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

    /** Add a thumb until it unmounts, returning its place among the thumbs. */
    join(): Accessor<number> {
        // list the thumb until it unmounts and read its place among the others
        const id = createUniqueId();
        this.#setThumbs((thumbs) => [...thumbs, id]);
        onCleanup(() => this.#setThumbs((thumbs) => thumbs.filter((entry) => entry !== id)));

        return () => this.#thumbs().indexOf(id);
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

/** The properties of a slider: its values, range and look, the native element's attributes included. */
export interface SliderProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onInput" | "onChange"
> {
    /** The values of the thumbs, one for a value and two for a range, which make the state controlled. */
    readonly value?: readonly number[];
    /** The values of the thumbs at first while uncontrolled, the middle of the range by default. */
    readonly defaultValue?: readonly number[];
    /** Handle each change of the values while a thumb moves. */
    readonly onValueChange?: (value: readonly number[]) => void;
    /** Handle the values the person settles on when a thumb is released. */
    readonly onValueCommit?: (value: readonly number[]) => void;
    /** The lowest value, 0 by default. */
    readonly min?: number;
    /** The highest value, 100 by default. */
    readonly max?: number;
    /** The step between values, 1 by default. */
    readonly step?: number;
    /** The values marked along the track. */
    readonly marks?: readonly number[];
    /** The direction the track runs along, horizontal by default. */
    readonly orientation?: "horizontal" | "vertical";
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
export type SliderElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

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

/** Render a slider around its track and thumbs, a native range input per thumb that arrow keys, Page Up, Page Down, Home and End move. */
export function Slider(properties: SliderProperties): JSX.Element {
    // share the values with the parts
    const control = new SliderControl(properties, useFieldControl());
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "onValueCommit",
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
                            <SliderRange />
                        </SliderTrack>
                        <Repeat count={control.values().length}>
                            {(index) => <SliderThumb index={index} />}
                        </Repeat>
                    </>
                )}
                <Show when={properties.marks}>
                    {(marks) => (
                        <div data-slot="slider-marks" {...style.attrs(styles.marks)}>
                            <For each={marks()}>
                                {(mark) => (
                                    <span
                                        data-slot="slider-mark"
                                        {...style.attrs(
                                            styles.mark,
                                            control.isVertical()
                                                ? styles.markAt(
                                                      "50%",
                                                      `${100 - control.share(mark)}%`,
                                                  )
                                                : styles.markAt(`${control.share(mark)}%`, "50%"),
                                        )}
                                    />
                                )}
                            </For>
                        </div>
                    )}
                </Show>
            </div>
        </SliderContext>
    );
}

/** Render the track of the nearest slider, which holds its range. */
export function SliderTrack(
    properties: SliderElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useSlider();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="slider-track"
            data-orientation={control.properties.orientation ?? "horizontal"}
            data-disabled={control.properties.disabled === true ? "" : undefined}
            {...rest}
            {...style.attributes(
                [styles.track, control.isVertical() && styles.verticalTrack, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render the filled span of the nearest slider's track: up to its value, or between a range's two values. */
export function SliderRange(
    properties: SliderElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // span from the lower thumb or the start up to the highest thumb
    const control = useSlider();
    const rest = omit(properties, "xstyle", "style");
    const from = (): string => {
        const values = control.values();

        return `${values.length > 1 ? control.share(values[0] ?? control.min()) : 0}%`;
    };
    const to = (): string => `${100 - control.share(control.values().at(-1) ?? control.min())}%`;

    return (
        <div
            data-slot="slider-range"
            data-orientation={control.properties.orientation ?? "horizontal"}
            data-disabled={control.properties.disabled === true ? "" : undefined}
            {...rest}
            {...style.attributes(
                [
                    styles.range,
                    control.isVertical()
                        ? styles.verticalSpan(from(), to())
                        : styles.span(from(), to()),
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Render a thumb of the nearest slider as a native range input, kept between its neighbours. */
export function SliderThumb(properties: SliderThumbProperties): JSX.Element {
    // take the thumb's place and label a range's thumbs by the end they set
    const control = useSlider();
    const locale = useLocale();
    const place = control.join();
    const index = (): number => properties.index ?? place();
    const rest = omit(properties, "index", "xstyle", "style");
    const label = (): string | undefined =>
        control.values().length === 2
            ? locale.render(index() === 0 ? t`Minimum` : t`Maximum`)
            : undefined;

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
            aria-label={label()}
            aria-orientation={control.properties.orientation}
            {...(index() === 0 ? control.field?.attributes() : undefined)}
            {...rest}
            value={control.valueAt(index())}
            onInput={(event) => control.move(index(), event.currentTarget)}
            onChange={() => control.properties.onValueCommit?.(control.values())}
            {...style.attributes(
                [styles.thumb, control.isVertical() && styles.verticalThumb, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Read the nearest slider, refusing a part outside one. */
function useSlider(): SliderControl {
    const control = useContext(SliderContext);
    if (control === null) {
        throw new TypeError("a slider part needs a slider around it");
    }

    return control;
}
