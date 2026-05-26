import type { JSX } from "solid-js";

type DottedFrameDepth = "small" | "medium" | "large";

type DottedFrameProps = {
    children: JSX.Element;
    class?: string;
    depth: DottedFrameDepth;
};

export function DottedFrame(props: DottedFrameProps) {
    return (
        <div
            class={`relative isolate min-w-0 ${props.class ?? ""}`}
            classList={{
                "pb-1.5 pr-1.5": props.depth === "small",
                "pb-2 pr-2": props.depth === "medium",
                "pb-3 pr-3": props.depth === "large",
            }}
        >
            <div
                aria-hidden="true"
                class="pointer-events-none absolute z-0 bg-[radial-gradient(circle,#e08b48_0_1.15px,transparent_1.3px)] bg-[length:3px_3px]"
                classList={{
                    "top-1.5 right-0 bottom-0 left-1.5": props.depth === "small",
                    "top-2 right-0 bottom-0 left-2": props.depth === "medium",
                    "top-3 right-0 bottom-0 left-3": props.depth === "large",
                }}
            />
            <div class="relative z-10 grid h-full min-h-0 min-w-0">{props.children}</div>
        </div>
    );
}
