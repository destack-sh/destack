import * as style from "@destack/style";
import { createContext, useContext } from "@destack/view";

/** The orientation of the nearest group joining its controls, undefined outside one. */
export const JoinContext = createContext<() => JoinOrientation | undefined>(() => undefined);

/** The direction a group joins its controls in. */
export type JoinOrientation = "horizontal" | "vertical";

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
        position: "relative",
        zIndex: { default: "auto", ":focus-visible": 1 },
        borderStartStartRadius: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
        borderStartEndRadius: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
        borderEndStartRadius: { default: null, ":not(:nth-last-child(1 of :not([popover])))": 0 },
        borderEndEndRadius: { default: null, ":not(:nth-last-child(1 of :not([popover])))": 0 },
        borderTopWidth: { default: null, ":not(:nth-child(1 of :not([popover])))": 0 },
    },
});

/** Return the StyleX styles that join a control to its neighbours in the nearest joining group. */
export function useJoin(): () => style.Styles {
    const orientation = useContext(JoinContext);

    return () => {
        const current = orientation();

        return current === undefined ? null : joined[current];
    };
}
