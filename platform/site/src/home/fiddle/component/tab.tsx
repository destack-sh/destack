import type { JSX } from "solid-js";

type FileTabProps = {
    isActive: boolean;
    label: string;
    onClick: () => void;
};

export function FileTab(props: FileTabProps) {
    return (
        <button
            class="group flex min-h-8 min-w-0 shrink-0 snap-start items-center gap-1.5 border-r border-neutral-950 px-3 text-left"
            classList={{
                "bg-destack-panel text-neutral-950": props.isActive,
                "bg-neutral-100 text-neutral-500 hover:bg-destack-panel hover:text-neutral-950":
                    !props.isActive,
            }}
            onClick={props.onClick}
            type="button"
        >
            <span
                class="size-2 shrink-0 rounded-full"
                classList={{
                    "bg-destack-accent": props.isActive,
                    "bg-neutral-300 group-hover:bg-neutral-500": !props.isActive,
                }}
            />
            <span class="block min-w-0 truncate whitespace-nowrap">{props.label}</span>
        </button>
    );
}

type FileTabsProps = {
    children: JSX.Element;
};

export function FileTabs(props: FileTabsProps) {
    return <div class="flex min-w-0 flex-1 overflow-x-auto">{props.children}</div>;
}
