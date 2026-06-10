import { For, Show, createEffect, createSignal, onCleanup, onMount } from "solid-js";

import { beginDrag, dragPayload, dropTarget } from "./drag";
import { MarkdownView } from "./markdown";
import { basename, isEdited, type WorkspaceFile } from "./workspace";

/// Cursor position reported to the status bar.
export type Cursor = {
    column: number;
    line: number;
};

/// Editor state survives pane moves and tab switches: undo history, selection, scroll.
const editorStates = new Map<
    string,
    { state: import("@codemirror/state").EditorState; scrollTop: number }
>();

type EditorGroupProps = {
    activePath: string;
    file: WorkspaceFile;
    onClose: (path: string) => void;
    onCursorChange: (cursor: Cursor) => void;
    onEdit: (path: string, source: string) => void;
    onMarkdownLink: (href: string) => boolean;
    onReorder: (from: string, to: string) => void;
    onSelect: (path: string) => void;
    openFiles: readonly WorkspaceFile[];
};

export function EditorGroup(props: EditorGroupProps) {
    return (
        <section class="grid h-full min-h-0 min-w-0 grid-rows-[2rem_minmax(0,1fr)] bg-editor-window">
            {/* Open file tabs */}
            <header class="flex min-h-8 min-w-0 items-stretch overflow-x-auto border-b border-editor-line bg-editor-shade text-xs font-medium">
                <For each={props.openFiles}>
                    {(file) => (
                        <EditorTab
                            file={file}
                            isActive={props.activePath === file.path}
                            onClose={() => props.onClose(file.path)}
                            onReorder={props.onReorder}
                            onSelect={() => props.onSelect(file.path)}
                        />
                    )}
                </For>
            </header>

            {/* Active document, keyed by path so each file keeps its own editor state */}
            <div class="min-h-0 min-w-0 overflow-hidden">
                <Show
                    fallback={<MarkdownView html={props.file.html} onLink={props.onMarkdownLink} />}
                    keyed
                    when={props.file.kind === "source" ? props.file.path : undefined}
                >
                    {(path) => (
                        <CodeEditor
                            file={props.file}
                            onCursorChange={props.onCursorChange}
                            onEdit={(source) => props.onEdit(path, source)}
                        />
                    )}
                </Show>
            </div>
        </section>
    );
}

type EditorTabProps = {
    file: WorkspaceFile;
    isActive: boolean;
    onClose: () => void;
    onReorder: (from: string, to: string) => void;
    onSelect: () => void;
};

function EditorTab(props: EditorTabProps) {
    // another tab dragged over this one inserts before it
    const isInsertTarget = () => {
        const payload = dragPayload();
        const target = dropTarget();
        return (
            payload?.kind === "tab" &&
            payload.path !== props.file.path &&
            target?.kind === "tab" &&
            target.path === props.file.path
        );
    };
    const grab = (event: PointerEvent) => {
        const path = props.file.path;
        beginDrag(event, { kind: "tab", path, label: basename(path) }, (target) => {
            if (target?.kind === "tab" && target.path !== path) {
                props.onReorder(path, target.path);
            }
        });
    };

    return (
        <div
            class="group flex min-w-0 shrink-0 snap-start items-center border-r border-editor-line"
            classList={{
                "bg-editor-window text-editor-text shadow-[inset_0_2px_0_var(--color-destack-accent)]":
                    props.isActive,
                "text-editor-muted hover:text-editor-text": !props.isActive,
                "shadow-[inset_2px_0_0_var(--color-destack-accent)]": isInsertTarget(),
            }}
            data-drop-tab={props.file.path}
            onPointerDown={grab}
        >
            <button class="min-w-0 truncate py-1.5 pr-1 pl-3" onClick={props.onSelect} type="button">
                {basename(props.file.path)}
            </button>

            {/* Dirty marker doubles as the close affordance */}
            <button
                aria-label={`close ${basename(props.file.path)}`}
                class="grid size-5 shrink-0 place-items-center text-editor-muted/60 hover:text-editor-text"
                onClick={(event) => {
                    event.stopPropagation();
                    props.onClose();
                }}
                type="button"
            >
                <Show fallback={<span>×</span>} when={isEdited(props.file)}>
                    <span class="size-1.5 rounded-full bg-destack-accent group-hover:hidden" />
                    <span class="hidden group-hover:inline">×</span>
                </Show>
            </button>
        </div>
    );
}

type CodeEditorProps = {
    file: WorkspaceFile;
    onCursorChange: (cursor: Cursor) => void;
    onEdit: (source: string) => void;
};

function CodeEditor(props: CodeEditorProps) {
    let host: HTMLDivElement | undefined;
    let isMounted = false;
    let view: import("codemirror").EditorView | undefined;
    const [isReady, setIsReady] = createSignal(false);

    onMount(async () => {
        isMounted = true;
        if (host == undefined) {
            return;
        }

        // load the editor lazily so first paint stays static html
        const [{ basicSetup, EditorView }, { javascript }, language, lezer, cmView] =
            await Promise.all([
                import("codemirror"),
                import("@codemirror/lang-javascript"),
                import("@codemirror/language"),
                import("@lezer/highlight"),
                import("@codemirror/view"),
            ]);

        if (!isMounted || host == undefined) {
            return;
        }

        const extensions = [
            basicSetup,
            javascript({ jsx: true, typescript: true }),
            language.syntaxHighlighting(highlightStyleFor(language, lezer)),
            destackKeywordsFor(cmView, language),
            EditorView.lineWrapping,
            editorThemeFor(EditorView),
            EditorView.updateListener.of((update) => {
                if (update.docChanged) {
                    props.onEdit(update.state.doc.toString());
                }
                if (update.docChanged || update.selectionSet) {
                    props.onCursorChange(cursorFor(update.view));
                }
            }),
        ];

        // resume the cached state when the document still matches, else start fresh
        const cached = editorStates.get(props.file.path);
        const isResumable = cached?.state.doc.toString() === props.file.source;
        view = new EditorView({
            ...(isResumable ? { state: cached!.state } : { doc: props.file.source, extensions }),
            parent: host,
        });
        if (isResumable) {
            requestAnimationFrame(() => {
                view?.scrollDOM.scrollTo({ top: cached!.scrollTop });
            });
        }

        // drop the placeholder one frame later so the editor never flashes half-laid-out
        requestAnimationFrame(() => {
            if (isMounted) {
                setIsReady(true);
            }
        });
        props.onCursorChange(cursorFor(view));
    });

    // sync external source swaps (file switch, reset) into the live document
    createEffect(() => {
        const source = props.file.source;
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
        props.onCursorChange(cursorFor(view));
    });

    onCleanup(() => {
        isMounted = false;

        // remember undo history, selection, and scroll for the next mount
        if (view != undefined) {
            editorStates.set(props.file.path, {
                state: view.state,
                scrollTop: view.scrollDOM.scrollTop,
            });
            view.destroy();
        }
    });

    return (
        <div class="relative h-full min-h-0 overflow-hidden">
            <div class="absolute inset-0" ref={host} />
            <Show when={!isReady()}>
                <pre class="editor-code numbered-code-block absolute inset-0 bg-editor-window">
                    <code>
                        <For each={props.file.html.split("\n")}>
                            {(line, index) => (
                                <span class="numbered-code-line">
                                    <span class="numbered-code-gutter">{index() + 1}</span>
                                    <span
                                        class="numbered-code-text"
                                        innerHTML={line === "" ? " " : line}
                                    />
                                </span>
                            )}
                        </For>
                    </code>
                </pre>
            </Show>
        </div>
    );
}

// TODO #Incomplete: replace with web-tree-sitter driving CodeMirror off language/grammar
function destackKeywordsFor(
    cmView: typeof import("@codemirror/view"),
    language: typeof import("@codemirror/language"),
) {
    const keyword = cmView.Decoration.mark({ class: "cm-destack-keyword" });
    const decorator = new cmView.MatchDecorator({
        decoration: (_match, view, position) => {
            // keywords inside comments, strings, and template text stay plain
            const node = language.syntaxTree(view.state).resolveInner(position + 1);
            return /Comment|String|Template/.test(node.name) ? null : keyword;
        },
        regexp: /\b(?:struct|newtype|extension|comptime|macro|where|unsafe|match|using|of|static|satisfies|implements)\b/g,
    });

    return cmView.ViewPlugin.define(
        (view) => ({
            decorations: decorator.createDeco(view),
            update(update: import("@codemirror/view").ViewUpdate) {
                this.decorations = decorator.updateDeco(update, this.decorations);
            },
        }),
        { decorations: (plugin) => plugin.decorations },
    );
}

function highlightStyleFor(
    language: typeof import("@codemirror/language"),
    lezer: typeof import("@lezer/highlight"),
) {
    const { tags } = lezer;

    // mirror the build-time data-k palette so static and live highlighting agree
    return language.HighlightStyle.define([
        {
            tag: [
                tags.keyword,
                tags.controlKeyword,
                tags.definitionKeyword,
                tags.moduleKeyword,
                tags.operatorKeyword,
                tags.modifier,
                tags.self,
            ],
            color: "var(--syntax-keyword)",
        },
        {
            tag: [tags.typeName, tags.className, tags.standard(tags.tagName)],
            color: "var(--syntax-type)",
        },
        {
            tag: [tags.number, tags.bool, tags.null, tags.atom],
            color: "var(--syntax-literal)",
        },
        {
            tag: [tags.string, tags.special(tags.string), tags.regexp],
            color: "var(--syntax-string)",
        },
        {
            tag: [tags.comment, tags.meta],
            color: "var(--syntax-comment)",
        },
        {
            tag: [
                tags.definition(tags.variableName),
                tags.function(tags.variableName),
                tags.function(tags.propertyName),
                tags.propertyName,
                tags.attributeName,
            ],
            color: "var(--syntax-name)",
        },
        {
            tag: [tags.operator, tags.punctuation, tags.bracket],
            color: "var(--syntax-punct)",
        },
    ]);
}

function editorThemeFor(EditorView: typeof import("codemirror").EditorView) {
    return EditorView.theme(
        {
            "&": {
                backgroundColor: "var(--color-editor-window)",
                color: "var(--color-editor-text)",
                fontSize: "14px",
                fontWeight: "450",
                height: "100%",
            },
            ".cm-activeLine": {
                backgroundColor: "rgb(214 222 235 / 0.04)",
            },
            ".cm-activeLineGutter": {
                backgroundColor: "rgb(214 222 235 / 0.06)",
                color: "var(--color-editor-text)",
            },
            ".cm-content": {
                caretColor: "var(--color-destack-accent)",
                padding: "0",
            },
            ".cm-cursor": {
                borderLeftColor: "var(--color-destack-accent)",
            },
            ".cm-focused": {
                outline: "none",
            },
            ".cm-gutters": {
                backgroundColor: "var(--color-editor-shade)",
                borderRight: "1px solid var(--color-editor-line)",
                color: "var(--syntax-gutter)",
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
                lineHeight: "1.375rem",
            },
            ".cm-selectionBackground": {
                backgroundColor: "rgb(224 139 72 / 0.2) !important",
            },
        },
        { dark: true },
    );
}

function cursorFor(view: import("codemirror").EditorView): Cursor {
    const head = view.state.selection.main.head;
    const line = view.state.doc.lineAt(head);
    const column = head - line.from + 1;

    return { column, line: line.number };
}
