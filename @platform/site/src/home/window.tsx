import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { children, type JSX } from "@destack/view";

import { tokens } from "../style/tokens.stylex";

/** Draw an app window: a card in the theme's card colour with a title bar of three lights and a title. */
export function Window(properties: {
    title: JSX.Element;
    style?: stylex.Styles;
    children: JSX.Element;
}) {
    // resolve the title once, so it renders the same on the server and in the browser
    const title = children(() => properties.title);

    return (
        <div {...stylex.attrs(styles.window, properties.style)}>
            <p {...stylex.attrs(styles.chrome)}>
                <span aria-hidden="true" {...stylex.attrs(styles.lights)} />
                {title()}
            </p>
            {properties.children}
        </div>
    );
}

/** The window styles. */
const styles = stylex.create({
    window: {
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.cardForeground,
        display: "grid",
        fontSize: "0.84375rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "2.5rem minmax(0, 1fr)",
        overflow: "hidden",
    },
    chrome: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
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
        backgroundImage:
            "radial-gradient(circle at 5.5px 50%, #ff5f57 5px, transparent 5.5px), radial-gradient(circle at 21.5px 50%, #febc2e 5px, transparent 5.5px), radial-gradient(circle at 37.5px 50%, #28c840 5px, transparent 5.5px)",
        flexShrink: 0,
        height: "11px",
        width: "44px",
    },
});
