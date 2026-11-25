import type { NumberFormat } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    omit,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The maximum of a progress bar that sets none. */
const MAXIMUM = 100;

/** The sweep an indeterminate indicator repeats while the work has no known length. */
const sweep = style.keyframes({
    "0%": { transform: "translateX(-100%)" },
    "100%": { transform: "translateX(250%)" },
});

/** The styles of a progress bar and its indicator. */
const styles = style.create({
    progress: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr) auto",
        alignItems: "baseline",
        rowGap: space[2],
        columnGap: space[2],
        width: "100%",
    },
    label: {
        gridColumn: "1",
        minWidth: 0,
        fontWeight: weight.medium,
    },
    track: {
        gridColumn: "1 / -1",
        outlineStyle: { default: "none", [media.forcedColors]: "solid" },
        outlineWidth: stroke.border,
        outlineOffset: `calc(-1 * ${stroke.border})`,
        position: "relative",
        display: "block",
        width: "100%",
        height: space[2],
        overflow: "hidden",
        borderRadius: radius.full,
        backgroundColor: `color-mix(in oklab, ${swatch.accentMark} 20%, transparent)`,
    },
    indicator: {
        display: "block",
        width: "100%",
        height: "100%",
        backgroundColor: { default: swatch.accentMark, [media.forcedColors]: "Highlight" },
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    value: {
        gridColumn: "2",
        justifySelf: "end",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    indeterminate: {
        width: "40%",
        animationName: sweep,
        animationDuration: `calc(3 * ${motion.durationLong})`,
        animationTimingFunction: motion.easingStandard,
        animationIterationCount: "infinite",
    },
});

/** The share of a progress bar's indicator that is filled. */
const fills = style.create({
    fill: (share: number) => ({ transform: `translateX(-${String(100 - share)}%)` }),
});

/** The state of a progress bar: running without a known value, running, or done. */
export type ProgressState = "indeterminate" | "loading" | "complete";

/** The value and maximum of the nearest progress bar, which its indicator reads. */
interface ProgressControl {
    /** The value, undefined while indeterminate. */
    readonly value: Accessor<number | undefined>;
    /** The maximum. */
    readonly max: Accessor<number>;
    /** The share done, between 0 and 1, undefined while indeterminate. */
    readonly share: Accessor<number | undefined>;
    /** The value as it is written, undefined while indeterminate. */
    readonly written: Accessor<string | undefined>;
    /** Label the bar by an element until it unmounts. */
    readonly addLabel: (id: string) => void;
    /** The state. */
    readonly state: Accessor<ProgressState>;
}

/** The nearest progress bar, null outside one. */
const ProgressContext = createContext<ProgressControl | null>(null);

/** The properties of a progress bar, the native element's attributes included. */
export interface ProgressProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The work done, between 0 and the maximum, indeterminate when absent. */
    readonly value?: number | undefined;
    /** The value no work is done at, 0 by default. */
    readonly min?: number;
    /** The work there is, 100 by default. */
    readonly max?: number;
    /** How the value is written, the share done in percent by default. */
    readonly format?: NumberFormat;
    /** Describe the value for assistive technology, its percentage by default. */
    readonly getValueLabel?: (value: number, max: number) => string;
    /** The StyleX styles applied after the progress bar's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a progress bar's indicator, the native element's attributes included. */
export type ProgressIndicatorProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** Render progress as a bar, with the label and the value composed in it. */
export function Progress(properties: ProgressProperties): JSX.Element {
    // clamp the value to the range and read the state it puts the bar in
    const locale = useLocale();
    const rest = omit(
        properties,
        "value",
        "min",
        "max",
        "format",
        "getValueLabel",
        "xstyle",
        "style",
        "children",
    );
    const labels = new CollectionBuilder<string>();
    const min = (): number => properties.min ?? 0;
    const max = (): number => maximumOf(min(), properties.max);
    const value = (): number | undefined =>
        properties.value === undefined
            ? undefined
            : Math.min(Math.max(properties.value, min()), max());
    const share = (): number | undefined => {
        const current = value();

        return current === undefined ? undefined : (current - min()) / (max() - min());
    };
    const state = (): ProgressState => {
        const current = value();

        return current === undefined ? "indeterminate" : current === max() ? "complete" : "loading";
    };

    // write the value, the share done without a format
    const written = (): string | undefined => {
        const current = value();
        if (current === undefined) {
            return undefined;
        }

        return properties.format === undefined
            ? locale.number(share() ?? 0, { style: "percent" })
            : locale.number(current, properties.format);
    };
    const label = (): string | undefined => {
        const current = value();

        return current === undefined
            ? undefined
            : (properties.getValueLabel?.(current, max()) ?? written());
    };

    return (
        <ProgressContext
            value={{ value, max, share, written, state, addLabel: (id) => void labels.add(id) }}
        >
            <div
                role="progressbar"
                aria-valuemin={min()}
                aria-valuemax={max()}
                aria-valuenow={value()}
                aria-valuetext={label()}
                aria-labelledby={labels.items().at(-1)}
                data-slot="progress"
                data-indeterminate={state() === "indeterminate" ? "" : undefined}
                data-progressing={state() === "loading" ? "" : undefined}
                data-complete={state() === "complete" ? "" : undefined}
                data-value={value()}
                data-max={max()}
                {...rest}
                {...style.attributes([styles.progress, properties.xstyle], properties.style)}
            >
                {properties.children ?? <ProgressTrack />}
            </div>
        </ProgressContext>
    );
}

/** Read the nearest progress bar, refusing a part outside one. */
function useProgress(): ProgressControl {
    const progress = useContext(ProgressContext);
    if (progress === null) {
        throw new TypeError("a progress part needs a progress bar around it");
    }

    return progress;
}

/** Render the label of the nearest progress bar, which names it. */
export function ProgressLabel(
    properties: Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> & ElementPartProperties,
): JSX.Element {
    // label the bar until it unmounts
    const progress = useProgress();
    const given = untrack(() => properties.id);
    const id = typeof given === "string" ? given : createUniqueId();
    progress.addLabel(id);

    return renderPart("span", "progress-label", properties, [text.sm, styles.label], { id });
}

/** Render the track of the nearest progress bar around its indicator. */
export function ProgressTrack(
    properties: Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> & ElementPartProperties,
): JSX.Element {
    const rest = omit(properties, "children");

    return renderPart("div", "progress-track", rest, styles.track, {
        get children() {
            return properties.children ?? <ProgressIndicator />;
        },
    });
}

/** Render the filled part of the nearest progress bar, sweeping across it while indeterminate. */
export function ProgressIndicator(properties: ProgressIndicatorProperties): JSX.Element {
    const progress = useProgress();
    return renderPart(
        "div",
        "progress-indicator",
        properties,
        () => [
            styles.indicator,
            progress.share() === undefined
                ? styles.indeterminate
                : fills.fill((progress.share() ?? 0) * 100),
        ],
        {
            get "data-indeterminate"() {
                return progress.state() === "indeterminate" ? "" : undefined;
            },
            get "data-progressing"() {
                return progress.state() === "loading" ? "" : undefined;
            },
            get "data-complete"() {
                return progress.state() === "complete" ? "" : undefined;
            },
            get "data-value"() {
                return progress.value();
            },
            get "data-max"() {
                return progress.max();
            },
        },
    );
}

/** The properties of a progress bar's value, the native element's attributes included. */
export type ProgressValueProperties = Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class" | "children"
> &
    ElementPartProperties & {
        /** Render the value as written, undefined while indeterminate. */
        readonly children?: (written: string | undefined, value: number | undefined) => JSX.Element;
    };

/** Render the value of the nearest progress bar as it is written, which its label reads aloud. */
export function ProgressValue(properties: ProgressValueProperties): JSX.Element {
    // write the bar's value
    const progress = useProgress();
    const rest = omit(properties, "children");

    return renderPart("span", "progress-value", rest, [text.sm, styles.value], {
        "aria-hidden": "true" as const,
        get children() {
            const { children } = properties;

            return children === undefined
                ? progress.written()
                : children(progress.written(), progress.value());
        },
    });
}

/** Read a progress bar's maximum, 100 when it sets none, refusing one at its minimum or less. */
function maximumOf(min: number, max: number | undefined): number {
    if (max !== undefined && max <= min) {
        throw new RangeError(
            `a progress bar's maximum must be above its minimum, not '${String(max)}'`,
        );
    }

    return max ?? MAXIMUM;
}
