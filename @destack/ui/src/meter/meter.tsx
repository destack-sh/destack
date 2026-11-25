import type { NumberFormat } from "@destack/locale";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
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
import { contentOr, type ElementPartProperties, renderPart } from "../part/index.ts";

/** The range a meter covers without its own minimum and maximum. */
const RANGE = { min: 0, max: 100 };

/** The styles of a meter and its parts. */
const styles = style.create({
    meter: {
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
    value: {
        gridColumn: "2",
        justifySelf: "end",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    track: {
        gridColumn: "1 / -1",
        outlineStyle: { default: "none", [media.forcedColors]: "solid" },
        outlineWidth: stroke.border,
        outlineOffset: `calc(-1 * ${stroke.border})`,
        width: "100%",
        height: space[2],
        overflow: "hidden",
        borderRadius: radius.full,
        backgroundColor: color.muted,
    },
    indicator: {
        height: "100%",
        borderRadius: radius.full,
        transitionProperty: "width, background-color",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    fill: (share: number) => ({ width: `${String(share * 100)}%` }),
});

/** The color of each level. */
const levels = style.create({
    optimum: { backgroundColor: { default: swatch.accentMark, [media.forcedColors]: "Highlight" } },
    suboptimum: {
        backgroundColor: { default: swatch.warningMark, [media.forcedColors]: "CanvasText" },
    },
    critical: {
        backgroundColor: { default: swatch.destructiveMark, [media.forcedColors]: "CanvasText" },
    },
});

/** How good a value is: in the best part of its range, beside it, or furthest from it. */
export type MeterLevel = "optimum" | "suboptimum" | "critical";

/** The value of the nearest meter, which its parts read. */
interface MeterControl {
    /** The value. */
    readonly value: Accessor<number>;
    /** The share of the range, between 0 and 1. */
    readonly share: Accessor<number>;
    /** How good the value is. */
    readonly level: Accessor<MeterLevel>;
    /** The value as it is written. */
    readonly written: Accessor<string>;
    /** Label the meter by an element until it unmounts. */
    readonly addLabel: (id: string) => void;
}

/** The nearest meter, null outside one. */
const MeterContext = createContext<MeterControl | null>(null);

/** The properties of a meter, the native element's attributes included. */
export type MeterProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties & {
        /** The value. */
        readonly value: number;
        /** The lowest value, 0 by default. */
        readonly min?: number;
        /** The highest value, 100 by default. */
        readonly max?: number;
        /** The value the low part of the range ends at, as HTML's meter, none by default. */
        readonly low?: number;
        /** The value the high part of the range starts at, none by default. */
        readonly high?: number;
        /** The best value, which tells whether low or high ones are good, the middle by default. */
        readonly optimum?: number;
        /** How the value is written, its share of the range in percent by default. */
        readonly format?: NumberFormat;
        /** Describe the value for assistive technology, as it is written by default. */
        readonly getValueLabel?: (written: string, value: number) => string;
    };

/** Render a measure within a known range, such as storage used, with its parts composed in it. */
export function Meter(properties: MeterProperties): JSX.Element {
    // clamp the value to the range
    const locale = useLocale();
    const labels = new CollectionBuilder<string>();
    const min = (): number => properties.min ?? RANGE.min;
    const max = (): number => Math.max(properties.max ?? RANGE.max, min());
    const value = (): number => Math.min(Math.max(properties.value, min()), max());
    const share = (): number => (max() === min() ? 0 : (value() - min()) / (max() - min()));
    const written = (): string =>
        properties.format === undefined
            ? locale.number(share(), { style: "percent" })
            : locale.number(properties.value, properties.format);
    const level = (): MeterLevel =>
        levelOf(value(), {
            low: properties.low ?? min(),
            high: properties.high ?? max(),
            optimum: properties.optimum ?? (min() + max()) / 2,
        });
    const rest = omit(
        properties,
        "value",
        "min",
        "max",
        "low",
        "high",
        "optimum",
        "format",
        "getValueLabel",
        "children",
    );

    return (
        <MeterContext
            value={{ value, share, level, written, addLabel: (id) => void labels.add(id) }}
        >
            {renderPart("div", "meter", rest, styles.meter, {
                role: "meter" as const,
                get "aria-valuemin"() {
                    return min();
                },
                get "aria-valuemax"() {
                    return max();
                },
                get "aria-valuenow"() {
                    return value();
                },
                get "aria-valuetext"() {
                    return properties.getValueLabel?.(written(), properties.value) ?? written();
                },
                get "aria-labelledby"() {
                    return labels.items().at(-1);
                },
                get "data-level"() {
                    return level();
                },
                get children() {
                    return contentOr(properties, () => <MeterTrack />);
                },
            })}
        </MeterContext>
    );
}

/** The properties of a part of a meter, the native element's attributes included. */
export type MeterElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** Render the label of the nearest meter, which names it. */
export function MeterLabel(properties: MeterElementProperties<HTMLSpanElement>): JSX.Element {
    // label the meter until it unmounts
    const meter = useMeter();
    const given = untrack(() => properties.id);
    const id = typeof given === "string" ? given : createUniqueId();
    meter.addLabel(id);

    return renderPart("span", "meter-label", properties, [text.sm, styles.label], { id });
}

/** The properties of a meter's value. */
export type MeterValueProperties = Omit<MeterElementProperties<HTMLSpanElement>, "children"> & {
    /** Render the value as written. */
    readonly children?: (written: string, value: number) => JSX.Element;
};

/** Render the value of the nearest meter as it is written, which its label reads aloud. */
export function MeterValue(properties: MeterValueProperties): JSX.Element {
    const meter = useMeter();
    const rest = omit(properties, "children");

    return renderPart("span", "meter-value", rest, [text.sm, styles.value], {
        "aria-hidden": "true" as const,
        get children() {
            return properties.children === undefined
                ? meter.written()
                : properties.children(meter.written(), meter.value());
        },
    });
}

/** Render the track of the nearest meter around its indicator. */
export function MeterTrack(properties: MeterElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "children");

    return renderPart("div", "meter-track", rest, styles.track, {
        get children() {
            return contentOr(properties, () => <MeterIndicator />);
        },
    });
}

/** Render the filled part of the nearest meter, colored by how good its value is. */
export function MeterIndicator(properties: MeterElementProperties<HTMLDivElement>): JSX.Element {
    const meter = useMeter();

    return renderPart("div", "meter-indicator", properties, () => [
        styles.indicator,
        levels[meter.level()],
        styles.fill(meter.share()),
    ]);
}

/** Read the nearest meter, refusing a part outside one. */
function useMeter(): MeterControl {
    const meter = useContext(MeterContext);
    if (meter === null) {
        throw new TypeError("a meter part needs a meter around it");
    }

    return meter;
}

/** Read how good a value is, by the part of the range it and the best value are in, as HTML. */
function levelOf(
    value: number,
    range: { readonly low: number; readonly high: number; readonly optimum: number },
): MeterLevel {
    // count the parts between the value's and the best value's
    const partOf = (at: number): number => (at < range.low ? 0 : at > range.high ? 2 : 1);
    const distance = Math.abs(partOf(value) - partOf(range.optimum));

    return distance === 0 ? "optimum" : distance === 1 ? "suboptimum" : "critical";
}
