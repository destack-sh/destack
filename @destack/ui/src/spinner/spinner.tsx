import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { motion } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";

/** The full turn a spinner repeats. */
const spin = style.keyframes({
    from: { transform: "rotate(0turn)" },
    to: { transform: "rotate(1turn)" },
});

/** The styles of a spinner. */
const styles = style.create({
    spinner: {
        display: "inline-flex",
        flexShrink: 0,
        animationName: spin,
        animationDuration: `calc(2 * ${motion.durationLong})`,
        animationTimingFunction: "linear",
        animationIterationCount: "infinite",
    },
});

/** The properties of a spinner, the native element's attributes included. */
export interface SpinnerProperties extends Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class" | "style"
> {
    /** The StyleX styles applied after the spinner's styles. */
    readonly style?: style.Styles;
}

/** Render a turning indicator that announces a loading state. */
export function Spinner(properties: SpinnerProperties): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <span
            data-slot="spinner"
            role="status"
            aria-label={locale.render(t`Loading`)}
            {...rest}
            {...style.attrs(styles.spinner, properties.style)}
        >
            <Icon name="circle-notch" />
        </span>
    );
}
