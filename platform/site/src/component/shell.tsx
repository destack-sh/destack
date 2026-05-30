import type { JSX } from "solid-js";

import { Footer } from "./footer";
import { TopBar } from "./topbar";

type ShellProps = {
    children: JSX.Element;
    class?: string;
    isFixed?: boolean;
};

export function Shell(props: ShellProps) {
    return (
        <div
            class={`grid min-h-svh min-w-0 grid-rows-[3rem_auto_3rem] overflow-x-clip bg-destack-page font-mono text-neutral-950 ${props.class ?? ""}`}
            classList={{
                "lg:h-svh lg:grid-rows-[3rem_minmax(0,1fr)_3rem] lg:overflow-hidden":
                    props.isFixed === true,
            }}
        >
            <TopBar />
            <main class="h-full min-h-0">{props.children}</main>
            <Footer />
        </div>
    );
}
