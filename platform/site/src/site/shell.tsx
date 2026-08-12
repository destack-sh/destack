import * as stylex from "@stylexjs/stylex";
import type { JSX } from "solid-js";

import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";
import { styles } from "./shell.stylex";

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
