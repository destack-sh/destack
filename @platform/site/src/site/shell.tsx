import { color, text } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { GooField } from "../effect/goo";
import { Footer } from "../navigation/footer";
import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";

/** Render the persistent site frame around one page, with the site's one goo field behind its goo cells. */
export function Shell(properties: { children: JSX.Element }) {
    return (
        <div {...stylex.attrs(styles.root)}>
            <GooField />
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
        fontFamily: text.family,
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
