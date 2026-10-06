import * as style from "@destack/style";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";

/** The track a progress bar fills, the primary color at a fifth of its strength. */
const TRACK = `color-mix(in oklab, ${color.primary} 20%, transparent)`;

/** The styles of a progress bar. */
const styles = style.create({
    progress: {
        appearance: "none",
        display: "block",
        width: "100%",
        height: space[2],
        overflow: "hidden",
        borderWidth: 0,
        borderRadius: radius.full,
        backgroundColor: TRACK,
        color: color.primary,
        accentColor: color.primary,
        "::-webkit-progress-bar": { backgroundColor: TRACK },
        "::-webkit-progress-value": { backgroundColor: color.primary },
        "::-moz-progress-bar": { backgroundColor: color.primary },
    },
});

/** The properties of a progress bar, the native progress element's attributes included. */
export interface ProgressProperties extends Omit<
    JSX.ProgressHTMLAttributes<HTMLProgressElement>,
    "class"
> {
    /** The StyleX styles applied after the progress bar's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a native progress bar, indeterminate without a value. */
export function Progress(properties: ProgressProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <progress
            data-slot="progress"
            {...rest}
            {...style.attributes([styles.progress, properties.xstyle], properties.style)}
        />
    );
}
