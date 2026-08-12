import * as stylex from "@stylexjs/stylex";
import type { JSX } from "solid-js";

import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";
import { tokens } from "../style/tokens.stylex";

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
        <div {...stylex.attrs(styles.root, !props.isHome && styles.paper)}>
            <KeyboardShortcuts />
            {!props.isHome && <div aria-hidden="true" {...stylex.attrs(styles.paperGrain)} />}
            {!props.isHome && (
                <div {...stylex.attrs(styles.paperLayer)}>
                    <TopBar />
                </div>
            )}
            <main {...stylex.attrs(styles.main, !props.isHome && styles.paperLayer)}>
                {props.children}
            </main>
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
        position: "relative",
        zIndex: 1,
    },
    root: {
        backgroundColor: tokens.page,
        color: tokens.text,
        display: "grid",
        fontFamily: tokens.textFont,
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: "100svh",
        minWidth: 0,
        overflowX: "clip",
    },
});
