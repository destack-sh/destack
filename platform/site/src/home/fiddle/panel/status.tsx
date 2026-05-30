import type { JSX } from "solid-js";

import type { Status, ViewerFile } from "./types";

type StatusBarProps = {
    status: Status;
};

export function StatusBar(props: StatusBarProps) {
    return (
        <footer class="flex min-w-0 items-stretch overflow-hidden border-t-2 border-neutral-950 bg-destack-panel bg-size-[3px_3px] bg-[radial-gradient(circle,rgb(224_139_72_/_0.42)_0_1px,transparent_1.25px)] text-sm font-extrabold lowercase text-neutral-950">
            <StatusCell class="bg-destack-accent">destack</StatusCell>
            <StatusCell>ready</StatusCell>
            <StatusCell class="min-w-0 flex-1 shrink truncate">{props.status.file}</StatusCell>
            <StatusCell>{modeLabel(props.status.mode)}</StatusCell>
            <StatusCell>{props.status.lines} lines</StatusCell>
            <StatusCell>
                ln {props.status.cursor.line}, col {props.status.cursor.column}
            </StatusCell>
            <StatusCell>{props.status.files} files</StatusCell>
        </footer>
    );
}

type StatusCellProps = {
    children: JSX.Element;
    class?: string;
};

function StatusCell(props: StatusCellProps) {
    return (
        <span
            class={`flex min-h-7 shrink-0 items-center border-r-2 border-neutral-950 bg-destack-panel/90 px-3 ${props.class ?? ""}`}
        >
            {props.children}
        </span>
    );
}

function modeLabel(mode: ViewerFile["kind"]) {
    if (mode === "source") {
        return "destack";
    }

    if (mode === "output") {
        return "readonly";
    }

    if (mode === "terminal") {
        return "terminal";
    }

    return "markdown";
}
