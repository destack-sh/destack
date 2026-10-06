import { t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { createControllableSignal, For, type JSX, omit, Show } from "@destack/view";
import { useFieldControl } from "../field/control.ts";

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

/** The styles of a slider, its track, thumbs and marks. */
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
        gridArea: "1 / 1",
        height: space[1],
        borderRadius: radius.full,
    },
    verticalTrack: {
        width: space[1],
        height: "auto",
    },
    fill: (image: string) => ({ backgroundImage: image }),
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
    /** The StyleX styles applied after the slider's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a track with a native range input per thumb, filled between its values, that arrow keys, Page Up, Page Down, Home and End move. */
export function Slider(properties: SliderProperties): JSX.Element {
    // follow the controlled values or the slider's own within the range
    const field = useFieldControl();
    const locale = useLocale();
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
        "xstyle",
        "style",
    );
    const minimum = (): number => properties.min ?? RANGE.min;
    const maximum = (): number => properties.max ?? RANGE.max;
    const [values, setValues] = createControllableSignal<readonly number[]>({
        isControlled: () => properties.value !== undefined,
        value: () => properties.value ?? [],
        defaultValue: properties.defaultValue ?? [(minimum() + maximum()) / 2],
        onChange: (next) => properties.onValueChange?.(next),
    });
    const isVertical = (): boolean => properties.orientation === "vertical";

    // read a value's share of the range and the filled track
    const share = (value: number): number => ((value - minimum()) / (maximum() - minimum())) * 100;
    const fill = (): string => {
        // fill from the lower thumb or the start up to the highest thumb
        const current = values();
        const from = current.length > 1 ? share(current[0] ?? minimum()) : 0;
        const to = share(current.at(-1) ?? minimum());
        const toward = isVertical() ? "top" : locale.direction === "rtl" ? "left" : "right";

        return `linear-gradient(to ${toward}, ${color.muted} ${from}%, ${color.primary} ${from}%, ${color.primary} ${to}%, ${color.muted} ${to}%)`;
    };

    // keep a moved thumb between its neighbours and show the owner's values until the owner takes them
    const move = (index: number, input: HTMLInputElement): void => {
        // clamp the value between the neighbouring thumbs
        const current = values();
        const lowest = current[index - 1] ?? minimum();
        const highest = current[index + 1] ?? maximum();
        const value = Math.min(Math.max(Number(input.value), lowest), highest);
        setValues(current.map((entry, position) => (position === index ? value : entry)));
        input.value = String(properties.value?.[index] ?? value);
    };

    // name the thumbs of a range from the lower one
    const label = (index: number): string | undefined =>
        values().length === 2 ? locale.render(index === 0 ? t`Minimum` : t`Maximum`) : undefined;

    return (
        <div
            role="group"
            data-slot="slider"
            data-orientation={properties.orientation ?? "horizontal"}
            data-disabled={properties.disabled === true ? "" : undefined}
            {...rest}
            {...style.attributes(
                [styles.slider, isVertical() && styles.vertical, properties.xstyle],
                properties.style,
            )}
        >
            <div
                data-slot="slider-track"
                {...style.attrs(
                    styles.track,
                    isVertical() && styles.verticalTrack,
                    styles.fill(fill()),
                )}
            />
            <Show when={properties.marks}>
                {(marks) => (
                    <div data-slot="slider-marks" {...style.attrs(styles.marks)}>
                        <For each={marks()}>
                            {(mark) => (
                                <span
                                    data-slot="slider-mark"
                                    {...style.attrs(
                                        styles.mark,
                                        isVertical()
                                            ? styles.markAt("50%", `${100 - share(mark)}%`)
                                            : styles.markAt(`${share(mark)}%`, "50%"),
                                    )}
                                />
                            )}
                        </For>
                    </div>
                )}
            </Show>
            <For each={values()}>
                {(value, index) => (
                    <input
                        type="range"
                        data-slot="slider-thumb"
                        min={minimum()}
                        max={maximum()}
                        step={properties.step}
                        name={properties.name}
                        disabled={properties.disabled}
                        aria-label={label(index())}
                        aria-orientation={properties.orientation}
                        {...(index() === 0 ? field?.attributes() : undefined)}
                        value={value}
                        onInput={(event) => move(index(), event.currentTarget)}
                        onChange={() => properties.onValueCommit?.(values())}
                        {...style.attrs(styles.thumb, isVertical() && styles.verticalThumb)}
                    />
                )}
            </For>
        </div>
    );
}
