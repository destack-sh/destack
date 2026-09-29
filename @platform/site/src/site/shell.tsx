import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { Universe } from "../effect/goo";
import { Footer } from "../navigation/footer";
import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";

/**
 * Render the persistent site frame around one page.
 *
 * The flow runs water through the footer's black hole: 1 draining into it, -1 welling out of it, 0 still.
 * The site's one goo field draws every goo cell, and a page with a universe grows it across the whole site frame while open.
 */
export function Shell(properties: { children: JSX.Element; flow?: number; universe?: boolean }) {
    // keep the universe closed and the water still on pages without them
    const isOpen = () => properties.universe ?? false;
    const flow = () => properties.flow ?? 0;

    return (
        <div {...stylex.attrs(styles.root)}>
            <Universe isOpen={isOpen()} flow={flow()} />
            <KeyboardShortcuts />
            <TopBar />
            <main {...stylex.attrs(styles.main)}>{properties.children}</main>
            <Footer />
        </div>
    );
}

/** The site frame styles. */
const styles = stylex.create({
    root: {
        color: color.foreground,
        display: "grid",
        fontFamily: fontFamily.default,
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        isolation: "isolate",
        minHeight: "100svh",
        overflow: "clip",
        position: "relative",
    },
    main: {
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
    },
});
