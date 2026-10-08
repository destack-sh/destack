import { color, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { children, type JSX } from "@destack/view";

import { palette } from "../../palette.stylex";

/** Draw an app window: a card in the theme's card colour with a title bar of three lights and a title. */
export function Window(properties: {
    title: JSX.Element;
    xstyle?: style.Styles;
    children: JSX.Element;
}) {
    // resolve the title once, so it renders the same on the server and in the browser
    const title = children(() => properties.title);

    return (
        <div data-component="Window" {...style.attrs(styles.window, properties.xstyle)}>
            <p {...style.attrs(styles.chrome)}>
                <span aria-hidden="true" {...style.attrs(styles.lights)} />
                {title()}
            </p>
            {properties.children}
        </div>
    );
}

/** The window styles. */
const styles = style.create({
    window: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.cardForeground,
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "2.5rem minmax(0, 1fr)",
        overflow: "hidden",
    },
    chrome: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.mutedForeground,
        display: "flex",
        fontSize: "0.78rem",
        gap: "0.75rem",
        margin: 0,
        minWidth: 0,
        overflow: "hidden",
        paddingInline: "0.875rem",
    },
    lights: {
        backgroundImage: `radial-gradient(circle at 5.5px 50%, ${palette.windowClose} 5px, transparent 5.5px), radial-gradient(circle at 21.5px 50%, ${palette.windowMinimize} 5px, transparent 5.5px), radial-gradient(circle at 37.5px 50%, ${palette.windowZoom} 5px, transparent 5.5px)`,
        flexShrink: 0,
        height: "11px",
        width: "44px",
    },
});
