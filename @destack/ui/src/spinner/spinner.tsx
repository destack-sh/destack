import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { motion } from "@destack/theme/tokens.stylex";
import { type JSX, useLocale } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

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
export type SpinnerProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> &
    ElementPartProperties;

/** Render a turning indicator that announces a loading state. */
export function Spinner(properties: SpinnerProperties): JSX.Element {
    const locale = useLocale();

    return renderPart("span", "spinner", properties, styles.spinner, {
        role: "status",
        get "aria-label"() {
            return locale.render(t`Loading`);
        },
        get children() {
            return <Icon name="circle-notch" />;
        },
    });
}
