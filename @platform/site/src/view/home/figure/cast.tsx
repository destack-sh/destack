import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";

import { chrome } from "./chrome.stylex";

/** Set a figure's window on the page with a dotted shadow, cast away from the top-left light. */
export function Cast(properties: { children: JSX.Element; xstyle?: style.Styles }) {
    return <div {...style.attrs(cast.window, properties.xstyle)}>{properties.children}</div>;
}

/** The cast shadow of a figure's window, and the clear halo of page around it. */
export const cast = style.create({
    window: {
        boxShadow: `0 0 0 4px ${color.background}`,
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr)",
        height: "100%",
        minHeight: 0,
        minWidth: 0,
        position: "relative",
        "::before": {
            backgroundImage: `radial-gradient(circle at 1px 1px, color-mix(in srgb, ${color.foreground} 38%, transparent) 0.9px, transparent 1.25px)`,
            backgroundSize: "5px 5px",
            borderRadius: chrome.radius,
            bottom: "-12px",
            content: '""',
            left: "12px",
            pointerEvents: "none",
            position: "absolute",
            right: "-12px",
            top: "12px",
            zIndex: -1,
        },
    },
});
