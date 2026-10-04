import * as style from "@destack/style";
import { createContext, useContext } from "solid-js";

/** The orientation of the nearest group joining its controls, undefined outside one. */
export const JoinContext = createContext<() => JoinOrientation | undefined>(() => undefined);

/** The direction a group joins its controls in. */
export type JoinOrientation = "horizontal" | "vertical";

/** The corners and borders a joined control drops where it meets its neighbours, raised above them while focused. */
const joined = style.create({
    horizontal: {
        position: "relative",
        zIndex: { default: "auto", ":focus-visible": 1 },
        borderStartStartRadius: { default: null, ":not(:first-child)": 0 },
        borderEndStartRadius: { default: null, ":not(:first-child)": 0 },
        borderStartEndRadius: { default: null, ":not(:last-child)": 0 },
        borderEndEndRadius: { default: null, ":not(:last-child)": 0 },
        borderInlineStartWidth: { default: null, ":not(:first-child)": 0 },
    },
    vertical: {
        position: "relative",
        zIndex: { default: "auto", ":focus-visible": 1 },
        borderStartStartRadius: { default: null, ":not(:first-child)": 0 },
        borderStartEndRadius: { default: null, ":not(:first-child)": 0 },
        borderEndStartRadius: { default: null, ":not(:last-child)": 0 },
        borderEndEndRadius: { default: null, ":not(:last-child)": 0 },
        borderTopWidth: { default: null, ":not(:first-child)": 0 },
    },
});

/** Return the StyleX styles that join a control to its neighbours in the nearest joining group, none outside one. */
export function useJoin(): () => style.Styles {
    const orientation = useContext(JoinContext);

    return () => {
        const current = orientation();

        return current === undefined ? null : joined[current];
    };
}
