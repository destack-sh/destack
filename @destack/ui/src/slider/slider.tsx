import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { createSignal, omit } from "solid-js";

/** The range a slider covers without its own minimum and maximum, the native default. */
const RANGE = { min: 0, max: 100 };

/** The filled share of a slider's track, which the slider sets inline from its value. */
const FILL = "var(--destack-slider-fill)";

/** The track of a slider, filled with the primary color up to its value. */
const TRACK = `linear-gradient(to right, ${color.primary} ${FILL}, ${color.muted} ${FILL})`;

/** The styles of a slider, its track and its thumb in each engine. */
const styles = style.create({
    slider: {
        appearance: "none",
        width: "100%",
        height: size[1],
        margin: 0,
        backgroundColor: "transparent",
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        outlineStyle: "none",
        "::-webkit-slider-runnable-track": {
            height: space[1],
            borderRadius: radius.full,
            backgroundImage: TRACK,
        },
        "::-moz-range-track": {
            height: space[1],
            borderRadius: radius.full,
            backgroundImage: TRACK,
        },
        "::-webkit-slider-thumb": {
            appearance: "none",
            width: space[4],
            height: space[4],
            marginTop: `calc((${space[1]} - ${space[4]}) / 2)`,
            borderStyle: "solid",
            borderWidth: stroke.border,
            borderColor: color.primary,
            borderRadius: radius.full,
            backgroundColor: color.background,
            boxShadow: {
                default: shadow.raised,
                ":focus-visible": `0 0 0 ${stroke.ring} color-mix(in oklab, ${color.ring} 50%, transparent)`,
            },
            transitionProperty: "box-shadow",
            transitionDuration: motion.durationShort,
        },
        "::-moz-range-thumb": {
            boxSizing: "border-box",
            width: space[4],
            height: space[4],
            borderStyle: "solid",
            borderWidth: stroke.border,
            borderColor: color.primary,
            borderRadius: radius.full,
            backgroundColor: color.background,
            boxShadow: {
                default: shadow.raised,
                ":focus-visible": `0 0 0 ${stroke.ring} color-mix(in oklab, ${color.ring} 50%, transparent)`,
            },
        },
    },
});

/** The properties of a slider, the native range input's attributes included. */
export interface SliderProperties extends Omit<
    JSX.InputHTMLAttributes<HTMLInputElement>,
    "class" | "style" | "type" | "onInput"
> {
    /** Handle each change of the value while the thumb moves. */
    readonly onInput?: (event: InputEvent & { readonly currentTarget: HTMLInputElement }) => void;
    /** The StyleX styles applied after the slider's styles. */
    readonly style?: style.Styles;
}

/** Render a native range input, filled up to its value, that arrow keys, Page Up, Page Down, Home and End move. */
export function Slider(properties: SliderProperties): JSX.Element {
    // follow the passed value, then each value the person moves to
    const rest = omit(properties, "style", "onInput");
    const minimum = (): number => Number(properties.min ?? RANGE.min);
    const maximum = (): number => Number(properties.max ?? RANGE.max);
    const [value, setValue] = createSignal(() =>
        Number(properties.value ?? (minimum() + maximum()) / 2),
    );

    // fill the track up to the value's share of the range
    const fill = (): string => `${((value() - minimum()) / (maximum() - minimum())) * 100}%`;

    return (
        <input
            type="range"
            data-slot="slider"
            {...rest}
            onInput={(event) => {
                setValue(Number(event.currentTarget.value));
                properties.onInput?.(event);
            }}
            {...style.attrs(styles.slider, properties.style)}
            style={{ "--destack-slider-fill": fill() }}
        />
    );
}
