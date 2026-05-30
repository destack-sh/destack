import { Show, createEffect, createSignal, onCleanup, onMount } from "solid-js";

import type { Cursor } from "./types";

type SourcePanelProps = {
    html: string;
    onCursorChange: ((cursor: Cursor) => void) | undefined;
    source: string;
};

export function SourcePanel(props: SourcePanelProps) {
    let host: HTMLDivElement | undefined;
    let isMounted = false;
    let view: import("codemirror").EditorView | undefined;
    const [isReady, setIsReady] = createSignal(false);

    onMount(async () => {
        isMounted = true;
        if (host == undefined) {
            return;
        }

        const [{ basicSetup, EditorView }, { javascript }] = await Promise.all([
            import("codemirror"),
            import("@codemirror/lang-javascript"),
        ]);

        if (!isMounted || host == undefined) {
            return;
        }

        view = new EditorView({
            doc: props.source,
            extensions: [
                basicSetup,
                javascript({ jsx: true, typescript: true }),
                EditorView.lineWrapping,
                editorThemeFor(EditorView),
                EditorView.updateListener.of((update) => {
                    if (update.docChanged || update.selectionSet) {
                        props.onCursorChange?.(cursorFor(update.view));
                    }
                }),
            ],
            parent: host,
        });

        setIsReady(true);
        props.onCursorChange?.(cursorFor(view));
    });

    createEffect(() => {
        const source = props.source;
        if (view == undefined || view.state.doc.toString() === source) {
            return;
        }

        view.dispatch({
            changes: {
                from: 0,
                insert: source,
                to: view.state.doc.length,
            },
        });
        props.onCursorChange?.(cursorFor(view));
    });

    onCleanup(() => {
        isMounted = false;
        view?.destroy();
    });

    return (
        <div class="editor-block relative">
            <div class="absolute inset-0" ref={host} />
            <Show when={!isReady()}>
                <pre class="code-block absolute inset-0">
                    <code innerHTML={props.html} />
                </pre>
            </Show>
        </div>
    );
}

function editorThemeFor(EditorView: typeof import("codemirror").EditorView) {
    return EditorView.theme({
        "&": {
            backgroundColor: "var(--color-destack-panel)",
            color: "#0a0a0a",
            fontSize: "14px",
            fontWeight: "700",
            height: "100%",
        },
        ".cm-activeLine": {
            backgroundColor: "rgb(224 139 72 / 0.1)",
        },
        ".cm-activeLineGutter": {
            backgroundColor: "rgb(224 139 72 / 0.15)",
            color: "#0a0a0a",
        },
        ".cm-content": {
            caretColor: "var(--color-destack-accent)",
            padding: "0",
        },
        ".cm-focused": {
            outline: "none",
        },
        ".cm-gutters": {
            backgroundColor: "var(--color-destack-panel)",
            borderRight: "1px solid rgb(10 10 10 / 0.18)",
            color: "#737373",
        },
        ".cm-foldGutter": {
            display: "none",
        },
        ".cm-lineNumbers": {
            minWidth: "3rem",
        },
        ".cm-lineNumbers .cm-gutterElement": {
            minWidth: "3rem",
            padding: "0 0.75rem 0 0.5rem",
            textAlign: "right",
        },
        ".cm-line": {
            padding: "0 1.25rem",
        },
        ".cm-scroller": {
            fontFamily:
                '"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, "SFMono-Regular", Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
            lineHeight: "1.25rem",
        },
        ".cm-selectionBackground": {
            backgroundColor: "rgb(224 139 72 / 0.24) !important",
        },
    });
}

function cursorFor(view: import("codemirror").EditorView): Cursor {
    const head = view.state.selection.main.head;
    const line = view.state.doc.lineAt(head);
    const column = head - line.from + 1;

    return { column, line: line.number };
}
