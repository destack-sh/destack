import { For, Show, createMemo } from "solid-js";

import { basename, type WorkspaceFile } from "./workspace";

type OutputsPaneProps = {
    file: WorkspaceFile;
    onTargetChange: (index: number) => void;
    targetIndex: number;
};

export function OutputsPane(props: OutputsPaneProps) {
    const output = createMemo(() => {
        const outputs = props.file.outputs;
        return outputs[props.targetIndex] ?? outputs[0];
    });

    return (
        <section class="grid h-full min-h-0 min-w-0 grid-rows-[2rem_minmax(0,1fr)] bg-editor-window">
            {/* Compiled target tabs */}
            <header class="flex min-h-8 min-w-0 items-stretch overflow-x-auto border-b border-editor-line bg-editor-shade pr-6 text-xs font-medium">
                <For each={props.file.outputs}>
                    {(target, index) => (
                        <button
                            class="shrink-0 border-r border-editor-line px-3 py-1.5"
                            classList={{
                                "bg-editor-window text-editor-text shadow-[inset_0_2px_0_var(--color-destack-accent)]":
                                    index() === props.targetIndex,
                                "text-editor-muted hover:text-editor-text": index() !== props.targetIndex,
                            }}
                            onClick={() => props.onTargetChange(index())}
                            type="button"
                        >
                            {target.label}
                        </button>
                    )}
                </For>
            </header>

            <Show
                fallback={<EmptyPane label="no compiled output for this file" />}
                when={output()}
            >
                {(target) => <OutputCode html={target().html} />}
            </Show>
        </section>
    );
}

type TerminalPaneProps = {
    file: WorkspaceFile;
};

export function TerminalPane(props: TerminalPaneProps) {
    return (
        <section class="grid h-full min-h-0 min-w-0 grid-rows-[2rem_minmax(0,1fr)] bg-editor-window">
            <header class="flex min-h-8 items-center border-b border-editor-line bg-editor-shade px-3 pr-6 text-xs font-medium text-editor-muted">
                terminal
            </header>

            <Show
                fallback={<EmptyPane label="no run output for this file" />}
                when={props.file.kind === "source"}
            >
                <pre class="editor-terminal">
                    <code>
                        <Show
                            fallback={
                                <>
                                    <span data-k="terminal-command">
                                        $ destack run {basename(props.file.path)}
                                    </span>
                                    {"\n"}
                                    <span class="text-editor-muted">(run output pending)</span>
                                </>
                            }
                            when={props.file.terminal}
                        >
                            {(terminal) => <span innerHTML={terminal()} />}
                        </Show>
                    </code>
                </pre>
            </Show>
        </section>
    );
}

type OutputCodeProps = {
    html: string;
};

function OutputCode(props: OutputCodeProps) {
    return (
        <pre class="editor-code numbered-code-block min-h-0">
            <code>
                <For each={props.html.split("\n")}>
                    {(line, index) => (
                        <span class="numbered-code-line">
                            <span class="numbered-code-gutter">{index() + 1}</span>
                            <span class="numbered-code-text" innerHTML={line === "" ? " " : line} />
                        </span>
                    )}
                </For>
            </code>
        </pre>
    );
}

type EmptyPaneProps = {
    label: string;
};

function EmptyPane(props: EmptyPaneProps) {
    return (
        <div class="grid h-full min-h-0 place-items-center px-4 text-xs text-editor-muted">
            {props.label}
        </div>
    );
}
