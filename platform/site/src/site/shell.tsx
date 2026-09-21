import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { Footer } from "../navigation/footer";
import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";

const mobile = "@media (max-width: 767px)";

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
        <div {...stylex.attrs(styles.root, props.isHome ? styles.home : styles.paper)}>
            <KeyboardShortcuts />
            {!props.isHome && <div aria-hidden="true" {...stylex.attrs(styles.paperGrain)} />}
            <div {...stylex.attrs(styles.paperLayer)}>
                <TopBar />
            </div>
            <main {...stylex.attrs(styles.main, !props.isHome && styles.paperLayer)}>
                {props.children}
            </main>
            <div {...stylex.attrs(styles.paperLayer)}>
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
    home: {
        height: "100svh",
        minHeight: 0,
        [mobile]: {
            height: "auto",
            minHeight: "100svh",
        },
    },
    paper: {
        isolation: "isolate",
        position: "relative",
    },
    paperGrain: {
        backgroundImage: 'url("/grain.svg")',
        backgroundRepeat: "repeat",
        backgroundSize: "8rem 8rem",
        inset: 0,
        mixBlendMode: "multiply",
        opacity: 0.24,
        pointerEvents: "none",
        position: "fixed",
        zIndex: 0,
    },
    paperLayer: {
        minWidth: 0,
        position: "relative",
        zIndex: 1,
    },
    root: {
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
