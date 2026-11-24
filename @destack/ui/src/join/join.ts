import type { Axis } from "../position/index.ts";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import { createContext, useContext } from "@destack/view";

/** The orientation of the nearest group joining its controls, undefined outside one. */
export const JoinContext = createContext<() => Axis | undefined>(() => undefined);

/** The corners and borders a joined control drops where it meets its neighbours, popovers aside. */
const joined = style.create({
    horizontal: {
        position: "relative",
        zIndex: { default: "auto", ":focus-visible": 1 },
        borderStartStartRadius: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
        borderEndStartRadius: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
        borderStartEndRadius: { default: null, ":not(:nth-last-child(1 of :not([popover])))": 0 },
        borderEndEndRadius: { default: null, ":not(:nth-last-child(1 of :not([popover])))": 0 },
        borderInlineStartWidth: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
    },
    vertical: {
        width: "100%",
        position: "relative",
        zIndex: { default: "auto", ":focus-visible": 1 },
        borderStartStartRadius: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
        borderStartEndRadius: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
        borderEndStartRadius: { default: null, ":not(:nth-last-child(1 of :not([popover])))": 0 },
        borderEndEndRadius: { default: null, ":not(:nth-last-child(1 of :not([popover])))": 0 },
        borderTopWidth: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
    },
});

/** Whether a line sets the filled controls of the nearest joining group apart. */
export const DivideContext = createContext(false);

/** The line after a filled control, a border so that high contrast draws it whole. */
const divided = style.create({
    horizontal: {
        borderInlineEndColor: {
            default: null,
            ":not(:nth-last-child(1 of :not([popover])))": color.background,
        },
    },
    vertical: {
        borderBottomColor: {
            default: null,
            ":not(:nth-last-child(1 of :not([popover])))": color.background,
        },
    },
});

/** The styles that join controls and set them apart. */
const DIVIDED: Readonly<Record<Axis, style.Styles>> = {
    horizontal: [joined.horizontal, divided.horizontal],
    vertical: [joined.vertical, divided.vertical],
};

/** Return the StyleX styles that join a control to its neighbours in the nearest joining group. */
export function useJoin(): () => style.Styles {
    const orientation = useContext(JoinContext);
    const isDivided = useContext(DivideContext);

    return () => {
        // join alone, or with a line after each
        const current = orientation();
        if (current === undefined) {
            return null;
        }

        return isDivided ? DIVIDED[current] : joined[current];
    };
}
