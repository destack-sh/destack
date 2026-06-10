import type { JSX } from "solid-js";

type PanelDepth = "shallow" | "deep";

type PanelProps = {
    children: JSX.Element;
    class?: string;
    depth: PanelDepth;
    title: string;
};

export function Panel(props: PanelProps) {
    return (
        <div
            class={`relative isolate min-w-0 ${props.class ?? ""}`}
            classList={{
                "pr-1 pb-1": props.depth === "shallow",
                "pr-2 pb-2": props.depth === "deep",
            }}
        >
            <div
                aria-hidden="true"
                class="pointer-events-none absolute right-0 bottom-0 z-0 bg-size-[3px_3px] bg-[radial-gradient(circle,var(--color-destack-accent)_0_1.15px,transparent_1.3px)]"
                classList={{
                    "top-1.5 left-1.5": props.depth === "shallow",
                    "top-3 left-3": props.depth === "deep",
                }}
            />

            <section class="relative z-10 h-full min-h-0 min-w-0 border-[2.5px] border-destack-frame bg-destack-panel">
                <span class="absolute -top-3 left-3 z-20 bg-destack-page px-1 text-sm font-extrabold lowercase">
                    {props.title}
                </span>
                {props.children}
            </section>
        </div>
    );
}
