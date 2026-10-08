import * as style from "@destack/style";
import { color, motion, radius, space } from "@destack/theme/tokens.stylex";
import { type Accessor, createContext, type JSX, omit, useContext } from "@destack/view";

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
        position: "relative",
        display: "block",
        width: "100%",
        height: space[2],
        overflow: "hidden",
        borderRadius: radius.full,
        backgroundColor: `color-mix(in oklab, ${color.primary} 20%, transparent)`,
    },
    indicator: {
        display: "block",
        width: "100%",
        height: "100%",
        backgroundColor: color.primary,
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
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
    /** The state. */
    readonly state: Accessor<ProgressState>;
}

/** The nearest progress bar, null outside one. */
const ProgressContext = createContext<ProgressControl | null>(null);

/** The properties of a progress bar, the native element's attributes included. */
export interface ProgressProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The work done, between 0 and the maximum, indeterminate when absent. */
    readonly value?: number | undefined;
    /** The work there is, 100 by default. */
    readonly max?: number;
    /** Describe the value for assistive technology, its percentage by default. */
    readonly getValueLabel?: (value: number, max: number) => string;
    /** The StyleX styles applied after the progress bar's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a progress bar's indicator, the native element's attributes included. */
export type ProgressIndicatorProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> & {
    /** The StyleX styles applied after the indicator's styles. */
    readonly xstyle?: style.Styles;
};

/** Render a progress bar around its indicator, indeterminate without a value. */
export function Progress(properties: ProgressProperties): JSX.Element {
    // keep the value within the range and read the state it puts the bar in
    const rest = omit(properties, "value", "max", "getValueLabel", "xstyle", "style", "children");
    const max = (): number => maximumOf(properties.max);
    const value = (): number | undefined =>
        properties.value === undefined ? undefined : Math.min(Math.max(properties.value, 0), max());
    const state = (): ProgressState => {
        const current = value();

        return current === undefined ? "indeterminate" : current === max() ? "complete" : "loading";
    };
    const label = (): string | undefined => {
        const current = value();
        if (current === undefined) {
            return undefined;
        }

        return (
            properties.getValueLabel?.(current, max()) ??
            `${String(Math.round((current / max()) * 100))}%`
        );
    };

    return (
        <ProgressContext value={{ value, max, state }}>
            <div
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={max()}
                aria-valuenow={value()}
                aria-valuetext={label()}
                data-slot="progress"
                data-state={state()}
                data-value={value()}
                data-max={max()}
                {...rest}
                {...style.attributes([styles.progress, properties.xstyle], properties.style)}
            >
                {properties.children ?? <ProgressIndicator />}
            </div>
        </ProgressContext>
    );
}

/** Render the filled part of the nearest progress bar, sweeping across it while indeterminate. */
export function ProgressIndicator(properties: ProgressIndicatorProperties): JSX.Element {
    // read the bar, refusing an indicator outside one
    const progress = useContext(ProgressContext);
    if (progress === null) {
        throw new TypeError("a progress indicator needs a progress bar around it");
    }
    const rest = omit(properties, "xstyle", "style");
    const share = (): number | undefined => {
        const value = progress.value();

        return value === undefined ? undefined : (value / progress.max()) * 100;
    };

    return (
        <div
            data-slot="progress-indicator"
            data-state={progress.state()}
            data-value={progress.value()}
            data-max={progress.max()}
            {...rest}
            {...style.attributes(
                [
                    styles.indicator,
                    share() === undefined ? styles.indeterminate : fills.fill(share() ?? 0),
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Read a progress bar's maximum, 100 when it sets none, refusing a maximum of zero or less. */
function maximumOf(max: number | undefined): number {
    if (max !== undefined && max <= 0) {
        throw new RangeError(`a progress bar's maximum must be positive, not ${String(max)}`);
    }

    return max ?? MAXIMUM;
}
