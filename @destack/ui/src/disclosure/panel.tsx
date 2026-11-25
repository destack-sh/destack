import * as style from "@destack/style";
import { motion } from "@destack/theme/tokens.stylex";
import type { Accessor, JSX } from "@destack/view";

/** The clip of a panel while its disclosure opens, which keeps focus rings whole after. */
const clip = style.keyframes({
    from: { overflow: "clip" },
    to: { overflow: "clip" },
});

/** The styles of a native disclosure whose panel grows and shrinks. */
export const disclosure = style.create({
    root: {
        "::details-content": {
            display: "grid",
            gridTemplateRows: { default: "0fr", ":is([open])": "1fr" },
            transitionProperty: "grid-template-rows, content-visibility",
            transitionDuration: motion.durationMedium,
            transitionTimingFunction: motion.easingStandard,
            transitionBehavior: "allow-discrete",
        },
    },
    panel: {
        minWidth: 0,
        minHeight: 0,
        overflow: "clip",
    },
    open: {
        overflow: "visible",
        animationName: clip,
        animationDuration: motion.durationMedium,
        animationTimingFunction: motion.easingStandard,
    },
});

/** Render what a native disclosure shows, in a row that grows and shrinks. */
export function DisclosurePanel(properties: {
    /** Whether the disclosure is open. */
    readonly isOpen: Accessor<boolean>;
    /** The content, whose spacing the row includes. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <div {...style.attrs(disclosure.panel, properties.isOpen() && disclosure.open)}>
            {properties.children}
        </div>
    );
}
