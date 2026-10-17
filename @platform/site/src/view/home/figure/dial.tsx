import { color, font } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";

/** The team sizes a bill can be read for. */
export const teams = [1, 5, 25, 100] as const;

/** The monthly active users a bill can be read for. */
export const audiences = [100, 1_000, 10_000, 100_000] as const;

/** Return a count of users as the dial shows it, in thousands from a thousand up. */
export function audienceOf(users: number): string {
    return users < 1_000 ? String(users) : `${users / 1_000}k`;
}

/** Draw a small dial of set values, one chosen, sized to sit in a figure's label row. */
export function Dial(properties: {
    label: string;
    options: readonly number[];
    value: number;
    format: (value: number) => string;
    onChange: (value: number) => void;
}) {
    return (
        <span
            role="radiogroup"
            aria-label={properties.label}
            data-component="Dial"
            {...style.attrs(styles.dial)}
        >
            <span aria-hidden="true">{properties.label}</span>
            <span {...style.attrs(styles.track)}>
                {properties.options.map((option) => (
                    <button
                        type="button"
                        role="radio"
                        aria-checked={option === properties.value ? "true" : "false"}
                        onClick={() => properties.onChange(option)}
                        {...style.attrs(
                            styles.option,
                            option === properties.value && styles.chosen,
                        )}
                    >
                        {properties.format(option)}
                    </button>
                ))}
            </span>
        </span>
    );
}

/** The dial styles. */
const styles = style.create({
    dial: {
        alignItems: "baseline",
        color: color.mutedForeground,
        display: "inline-flex",
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        gap: "0.5rem",
        letterSpacing: "0.1em",
        marginBlock: "-0.25rem",
        textTransform: "uppercase",
    },
    track: {
        borderColor: color.border,
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        fontSize: "0.625rem",
        padding: "1px",
    },
    option: {
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
        borderRadius: "3px",
        borderWidth: 0,
        color: "inherit",
        cursor: "pointer",
        fontFamily: "inherit",
        fontSize: "inherit",
        fontVariantNumeric: "tabular-nums",
        letterSpacing: "0.04em",
        lineHeight: "1rem",
        minWidth: "1.75rem",
        paddingInline: "0.375rem",
    },
    chosen: {
        backgroundColor: color.foreground,
        color: color.background,
    },
});
