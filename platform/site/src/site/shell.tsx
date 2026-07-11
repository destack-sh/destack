import type { JSX } from "solid-js";

import { Footer } from "../navigation/footer";
import { KeyboardShortcuts } from "../navigation/shortcut";
import { TopBar } from "../navigation/topbar";

type ShellProps = {
    /// The page body.
    children: JSX.Element;

    /// Additional shell classes.
    class?: string;

    /// Whether the shell remains exactly one viewport high on wide screens.
    isFixed?: boolean;
};

/// Render the persistent site frame around one page.
export function Shell(props: ShellProps) {
    return (
        <div
            class={
                "grid min-h-svh min-w-0 grid-rows-[6rem_auto_6rem] overflow-x-clip " +
                "md:grid-rows-[3rem_auto_3rem] " +
                `bg-destack-page font-mono text-destack-text ${props.class ?? ""}`
            }
            classList={{
                "lg:h-svh lg:grid-rows-[3rem_minmax(0,1fr)_3rem] lg:overflow-hidden":
                    props.isFixed === true,
            }}
        >
            <KeyboardShortcuts />
            <TopBar />
            <main class="h-full min-h-0">{props.children}</main>
            <Footer />
        </div>
    );
}
