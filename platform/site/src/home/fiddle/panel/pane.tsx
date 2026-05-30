import type { JSX } from "solid-js";

type PaneProps = {
    bodyClass?: string;
    children: JSX.Element;
    class?: string;
    classList?: Record<string, boolean>;
    tabs: JSX.Element;
};

export function Pane(props: PaneProps) {
    return (
        <section
            class={`grid h-full min-h-0 min-w-0 grid-rows-[2rem_minmax(0,1fr)] bg-destack-panel ${props.class ?? ""}`}
            classList={props.classList}
        >
            <header class="flex min-h-8 min-w-0 items-stretch border-b-2 border-neutral-950 bg-neutral-100 text-sm font-extrabold lowercase">
                {props.tabs}
            </header>
            <div class={`min-h-0 min-w-0 ${props.bodyClass ?? ""}`}>{props.children}</div>
        </section>
    );
}
