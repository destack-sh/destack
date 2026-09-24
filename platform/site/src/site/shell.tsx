import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { Footer } from "../navigation/footer";
import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";

/**
 * Render the persistent site frame around one page.
 *
 * The flow runs water through the footer's black hole: 1 draining into it, -1 welling out of it, 0 still.
 */
export function Shell(properties: { children: JSX.Element; flow?: number }) {
    return (
        <div {...stylex.attrs(styles.root)}>
            <KeyboardShortcuts />
            <TopBar />
            <main {...stylex.attrs(styles.main)}>{properties.children}</main>
            <Footer flow={properties.flow} />
        </div>
    );
}

/** The site frame styles. */
const styles = stylex.create({
    root: {
        backgroundColor: color.background,
        color: color.foreground,
        display: "grid",
        fontFamily: fontFamily.default,
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minHeight: "100svh",
        overflow: "clip",
    },
    main: {
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
    },
});
