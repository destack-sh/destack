import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { Footer } from "../navigation/footer";
import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";

/// Properties for the persistent site frame.
type ShellProps = {
    /// The page body.
    children: JSX.Element;

    /// Whether the page supplies its own visual field.
    isHome?: boolean;
};

/// Render the persistent site frame around one page.
export function Shell(props: ShellProps) {
    return (
        <div {...stylex.attrs(styles.root)} data-home={props.isHome ? "" : undefined} data-site>
            <KeyboardShortcuts />
            <div {...stylex.attrs(styles.layer)}>
                <TopBar />
            </div>
            <main
                {...stylex.attrs(styles.main, styles.layer)}
                data-reading={props.isHome ? undefined : ""}
            >
                {props.children}
            </main>
            <div {...stylex.attrs(styles.layer)}>
                <Footer />
            </div>
        </div>
    );
}

const styles = stylex.create({
    main: {
        display: "grid",
        minHeight: 0,
        minWidth: 0,
        width: "100%",
    },
    layer: {
        minWidth: 0,
        position: "relative",
        zIndex: 1,
    },
    root: {
        isolation: "isolate",
        position: "relative",
        backgroundColor: color.background,
        color: color.foreground,
        display: "grid",
        fontFamily: fontFamily.default,
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minHeight: "100svh",
        minWidth: 0,
        overflowX: "clip",
        width: "100%",
    },
});
