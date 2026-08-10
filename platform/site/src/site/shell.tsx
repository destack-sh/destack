import type { JSX } from "solid-js";

import { Footer } from "../navigation/footer";
import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";

/// Properties for the persistent site frame.
type ShellProps = {
    /// The page body.
    children: JSX.Element;

    /// Additional shell classes.
    class?: string;
};

/// Render the persistent site frame around one page.
export function Shell(props: ShellProps) {
    return (
        <div class={`site-shell ${props.class ?? ""}`}>
            <KeyboardShortcuts />
            <TopBar />
            <main class="site-main grid">{props.children}</main>
            <Footer />
        </div>
    );
}
