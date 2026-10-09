import * as style from "@destack/style";
import { color, motion, radius } from "@destack/theme/tokens.stylex";
import { type JSX } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

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
export type SkeletonProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** Render a pulsing placeholder in the shape of content that is still loading. */
export function Skeleton(properties: SkeletonProperties): JSX.Element {
    return renderPart("div", "skeleton", properties, styles.skeleton);
}
