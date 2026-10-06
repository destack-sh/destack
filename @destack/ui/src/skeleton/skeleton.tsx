import * as style from "@destack/style";
import { color, motion, radius } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";

/** The fade a skeleton repeats while its content loads. */
const pulse = style.keyframes({
    "0%, 100%": { opacity: 1 },
    "50%": { opacity: 0.5 },
});

/** The styles of a skeleton. */
const styles = style.create({
    skeleton: {
        borderRadius: radius[3],
        backgroundColor: color.accent,
        animationName: pulse,
        animationDuration: `calc(4 * ${motion.durationLong})`,
        animationTimingFunction: motion.easingStandard,
        animationIterationCount: "infinite",
    },
});

/** The properties of a skeleton, the native element's attributes included. */
export interface SkeletonProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The StyleX styles applied after the skeleton's styles, usually its size. */
    readonly xstyle?: style.Styles;
}

/** Render a pulsing placeholder in the shape of content that is still loading. */
export function Skeleton(properties: SkeletonProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="skeleton"
            {...rest}
            {...style.attributes([styles.skeleton, properties.xstyle], properties.style)}
        />
    );
}
