import { Show, createEffect, createMemo, onCleanup, onMount } from "solid-js";
import { createStore } from "solid-js/store";

import { toggleTheme } from "../component/theme";
import { CommandBar, type Command } from "./command";
import {
    DockView,
    appendView,
    dockViews,
    moveView,
    removeView,
    resizeSplit,
    type DockNode,
    type ViewChrome,
} from "./dock";
import { DragGhost } from "./drag";
import { EditorGroup, type Cursor } from "./editor";
import { Explorer } from "./explorer";
import { OutputsPane, TerminalPane } from "./output";
import {
    basename,
    createFile,
    deleteFile,
    dirname,
    isEdited,
    persistWorkspace,
    renameFile,
    restoreWorkspace,
    seedFiles,
    type WorkspaceStore,
} from "./workspace";

const layoutStorageKey = "destack-editor-layout";

/// One view instance the dock can place; the kind is fixed for the instance's lifetime.
type View = {
    kind: "explorer" | "editor" | "output" | "terminal";
    /// Selected output tab, meaningful for output views only.
    target: number;
};

/// One arrangeable layout: a dock tree plus the view instances at its leaves.
type Layout = {
    dock: DockNode;
    views: Record<string, View>;
};

/// Documents read centered without side panels; source files get the full bench.
type LayoutKind = "document" | "code";

/// The default layouts: documents are explorer plus editor, code adds outputs
/// and terminal.
function defaultLayouts(): Record<LayoutKind, Layout> {
    return {
        document: {
            dock: {
                kind: "split",
                direction: "row",
                children: [
                    { kind: "leaf", view: "explorer" },
                    { kind: "leaf", view: "editor" },
                ],
                sizes: [0.18, 0.82],
            },
            views: {
                explorer: { kind: "explorer", target: 0 },
                editor: { kind: "editor", target: 0 },
            },
        },
        code: {
            dock: {
                kind: "split",
                direction: "row",
                children: [
                    { kind: "leaf", view: "explorer" },
                    { kind: "leaf", view: "editor" },
                    {
                        kind: "split",
                        direction: "column",
                        children: [
                            { kind: "leaf", view: "outputs" },
                            { kind: "leaf", view: "terminal" },
                        ],
                        sizes: [0.6, 0.4],
                    },
                ],
                sizes: [0.16, 0.5, 0.34],
            },
            views: {
                explorer: { kind: "explorer", target: 0 },
                editor: { kind: "editor", target: 0 },
                outputs: { kind: "output", target: 0 },
                terminal: { kind: "terminal", target: 0 },
            },
        },
    };
}

const defaultPath = "README.md";
const expandDurationMs = 280;

/// Transient IDE state: open editors, layouts, and cursor.
type ShellState = {
    activePath: string;
    closedDirectories: readonly string[];
    cursor: Cursor;
    isCommandOpen: boolean;
    isExpanded: boolean;
    layouts: Record<LayoutKind, Layout>;
    openPaths: readonly string[];
};

type EditorShellProps = {
    workspace: WorkspaceStore;
};

export function EditorShell(props: EditorShellProps) {
    const [workspace, setWorkspace] = props.workspace;
    const [shell, setShell] = createStore<ShellState>({
        activePath: defaultPath,
        closedDirectories: [],
        cursor: { column: 1, line: 1 },
        isCommandOpen: false,
        isExpanded: false,
        layouts: defaultLayouts(),
        openPaths: [defaultPath],
    });
    let frame: HTMLDivElement | undefined;

    const activeFile = createMemo(() => workspace.files[shell.activePath]);

    // documents read without side panels; source files get the full bench
    const layoutKind = createMemo<LayoutKind>(() =>
        activeFile()?.kind === "markdown" ? "document" : "code",
    );
    const layout = () => shell.layouts[layoutKind()];
    const openFiles = createMemo(() =>
        shell.openPaths.map((path) => workspace.files[path]).filter((file) => file != undefined),
    );

    // file operations
    const open = (path: string) => {
        if (!shell.openPaths.includes(path)) {
            setShell("openPaths", (paths) => [...paths, path]);
        }
        setShell("activePath", path);
    };
    const close = (path: string) => {
        const remaining = shell.openPaths.filter((entry) => entry !== path);
        setShell("openPaths", remaining);

        // closing the active tab activates its neighbor
        if (shell.activePath === path) {
            setShell("activePath", remaining.at(-1) ?? defaultPath);
        }
        if (remaining.length === 0) {
            setShell("openPaths", [defaultPath]);
        }
    };
    const edit = (path: string, source: string) => {
        setWorkspace("files", path, "source", source);
    };
    const reset = () => {
        const file = activeFile();
        if (file != undefined) {
            setWorkspace("files", file.path, "source", file.pristine);
        }
    };
    const create = (directory: string) => {
        // find a free untitled name in the target directory
        const prefix = directory === "" ? "" : `${directory}/`;
        let name = "untitled.ds";
        let counter = 1;
        while (!createFile(props.workspace, `${prefix}${name}`)) {
            counter += 1;
            name = `untitled-${counter}.ds`;
        }

        open(`${prefix}${name}`);
    };
    const rename = (from: string, to: string) => {
        if (!renameFile(props.workspace, from, to)) {
            return;
        }

        // follow the file in open tabs and the active selection
        setShell("openPaths", (paths) => paths.map((path) => (path === from ? to : path)));
        if (shell.activePath === from) {
            setShell("activePath", to);
        }
    };
    const remove = (path: string) => {
        deleteFile(props.workspace, path);
        close(path);
    };
    const move = (path: string, directory: string) => {
        rename(path, directory === "" ? basename(path) : `${directory}/${basename(path)}`);
    };
    const toggleDirectory = (path: string) => {
        setShell("closedDirectories", (closed) =>
            closed.includes(path) ? closed.filter((entry) => entry !== path) : [...closed, path],
        );
    };
    const resetWorkspace = () => {
        // back to the pristine seed: files, layout, and persisted state
        localStorage.removeItem(layoutStorageKey);
        setWorkspace("files", seedFiles());
        setShell({
            activePath: defaultPath,
            closedDirectories: [],
            layouts: defaultLayouts(),
            openPaths: [defaultPath],
        });
    };
    const reorder = (from: string, to: string) => {
        setShell("openPaths", (paths) => {
            const list = paths.filter((path) => path !== from);
            const at = list.indexOf(to);
            list.splice(at < 0 ? list.length : at, 0, from);

            return list;
        });
    };

    // markdown links resolve against the document's directory, then the root
    const openMarkdownLink = (href: string) => {
        const file = activeFile();
        const path = href.split(/[?#]/, 1)[0]?.trim() ?? "";
        if (file == undefined || path === "" || /^[a-z][a-z0-9+.-]*:/i.test(path)) {
            return false;
        }

        const relative = normalizePath(`${dirname(file.path)}/${path}`);
        const absolute = normalizePath(path);
        const target = [relative, absolute].find((entry) => workspace.files[entry] != undefined);
        if (target == undefined) {
            return false;
        }

        open(target);
        return true;
    };

    // expanding plays a FLIP transform between embedded and fullscreen
    const toggleExpand = () => {
        const apply = () => setShell("isExpanded", (expanded) => !expanded);
        if (frame == undefined) {
            apply();
            return;
        }

        const before = frame.getBoundingClientRect();
        apply();

        const after = frame.getBoundingClientRect();
        const transform = `translate(${before.left - after.left}px, ${before.top - after.top}px) scale(${before.width / after.width}, ${before.height / after.height})`;
        animateTransform(frame, transform);
    };

    // restore persisted user state after hydration, then keep persisting changes
    onMount(() => {
        restoreWorkspace(props.workspace);

        const raw = localStorage.getItem(layoutStorageKey);
        if (raw != undefined) {
            const saved = JSON.parse(raw) as Partial<ShellState>;
            const openPaths = (saved.openPaths ?? []).filter(
                (path) => workspace.files[path] != undefined,
            );
            if (openPaths.length > 0) {
                setShell("openPaths", openPaths);
            }
            if (saved.activePath != undefined && workspace.files[saved.activePath] != undefined) {
                setShell("activePath", saved.activePath);
            }
            if (saved.closedDirectories != undefined) {
                setShell("closedDirectories", saved.closedDirectories);
            }

            // only adopt persisted layouts whose leaves match their views and keep an editor
            for (const kind of ["document", "code"] as const) {
                const candidate = saved.layouts?.[kind];
                if (candidate == undefined) {
                    continue;
                }

                const leaves = dockViews(candidate.dock);
                const ids = Object.keys(candidate.views);
                const hasEditor = leaves.some((id) => candidate.views[id]?.kind === "editor");
                if (hasEditor && leaves.sort().join(",") === ids.sort().join(",")) {
                    setShell("layouts", kind, candidate);
                }
            }
        }

        // a #path deep link opens that file, winning over the restored selection,
        // and hash navigation keeps working after load
        const openHash = () => {
            const hash = decodeURIComponent(window.location.hash.slice(1));
            if (hash !== "" && workspace.files[hash] != undefined) {
                open(hash);
            }
        };
        openHash();
        window.addEventListener("hashchange", openHash);
        onCleanup(() => window.removeEventListener("hashchange", openHash));

        // reflect the active file in the address bar for shareable links
        createEffect(() => {
            const hash = shell.activePath === defaultPath ? "" : `#${encodeURI(shell.activePath)}`;
            const url = `${window.location.pathname}${window.location.search}${hash}`;
            window.history.replaceState(null, "", url);
        });

        createEffect(() => persistWorkspace(workspace));
        createEffect(() => {
            const persisted = {
                activePath: shell.activePath,
                closedDirectories: shell.closedDirectories,
                layouts: shell.layouts,
                openPaths: shell.openPaths,
            };
            localStorage.setItem(layoutStorageKey, JSON.stringify(persisted));
        });
    });

    // global keys: command bar on mod+k / mod+p, escape closes overlays
    onMount(() => {
        const handleKeys = (event: KeyboardEvent) => {
            const isModifier = event.metaKey || event.ctrlKey;
            if (isModifier && (event.key === "k" || event.key === "p")) {
                event.preventDefault();
                setShell("isCommandOpen", (open) => !open);
            } else if (event.key === "Escape" && shell.isCommandOpen) {
                setShell("isCommandOpen", false);
            } else if (event.key === "Escape" && shell.isExpanded) {
                toggleExpand();
            }
        };
        window.addEventListener("keydown", handleKeys);
        onCleanup(() => window.removeEventListener("keydown", handleKeys));
    });


    // view instances: add anywhere, close anything but the last editor
    const addView = (kind: View["kind"]) => {
        let counter = 1;
        let id = `${kind}-${counter}`;
        while (layout().views[id] != undefined) {
            counter += 1;
            id = `${kind}-${counter}`;
        }

        setShell("layouts", layoutKind(), "views", id, { kind, target: 0 });
        setShell("layouts", layoutKind(), "dock", (dock) => appendView(dock, id));
    };
    const closeView = (id: string) => {
        const view = layout().views[id];
        const editors = dockViews(layout().dock).filter((entry) => layout().views[entry]?.kind === "editor");
        if (view == undefined || (view.kind === "editor" && editors.length <= 1)) {
            return;
        }

        const dock = removeView(layout().dock, id);
        if (dock != undefined) {
            setShell("layouts", layoutKind(), "dock", dock);
            setShell("layouts", layoutKind(), "views", id, undefined!);
        }
    };

    // chrome rendered into whatever layout the user arranges; accessors stay lazy so
    // layout changes do not remount pane contents
    const chrome = (id: string): ViewChrome => ({
        isClosable: () => {
            const view = layout().views[id];
            const isLastEditor =
                view?.kind === "editor" &&
                dockViews(layout().dock).filter((entry) => layout().views[entry]?.kind === "editor")
                    .length <= 1;

            return view != undefined && !isLastEditor;
        },
        title: () => layout().views[id]?.kind ?? "view",
        render: () => {
            // the kind is fixed per instance, so branching once is safe
            const kind = layout().views[id]?.kind;

            // explorer is workspace-wide, the rest follow the active file
            if (kind === "explorer") {

    return (
                    <Explorer
                        activePath={shell.activePath}
                        closedDirectories={shell.closedDirectories}
                        onCreate={create}
                        onDelete={remove}
                        onMove={move}
                        onOpen={open}
                        onRename={rename}
                        onToggleDirectory={toggleDirectory}
                        workspace={workspace}
                    />
                );
            }
            if (kind === "editor") {
                return (
                    <Show when={activeFile()}>
                        {(file) => (
                            <EditorGroup
                                activePath={shell.activePath}
                                file={file()}
                                onClose={close}
                                onCursorChange={(cursor) => setShell("cursor", cursor)}
                                onEdit={edit}
                                onMarkdownLink={openMarkdownLink}
                                onReorder={reorder}
                                onSelect={open}
                                openFiles={openFiles()}
                            />
                        )}
                    </Show>
                );
            }
            if (kind === "output") {
                return (
                    <Show when={activeFile()}>
                        {(file) => (
                            <OutputsPane
                                file={file()}
                                onTargetChange={(index) => setShell("layouts", layoutKind(), "views", id, "target", index)}
                                targetIndex={layout().views[id]?.target ?? 0}
                            />
                        )}
                    </Show>
                );
            }

            return <Show when={activeFile()}>{(file) => <TerminalPane file={file()} />}</Show>;
        },
    });

    // command bar entries beyond plain file opening
    const commands = createMemo<readonly Command[]>(() => {
        const file = activeFile();
        const entries: Command[] = [
            {
                id: "new-file",
                label: "new file",
                run: () => create(""),
            },
            {
                id: "theme",
                label: "toggle light/dark theme",
                run: toggleTheme,
            },
            {
                id: "copy-link",
                label: `copy link to ${basename(shell.activePath)}`,
                hint: "share",
                run: () => {
                    const link = `${window.location.origin}/#${encodeURI(shell.activePath)}`;
                    void navigator.clipboard.writeText(link);
                },
            },
            {
                id: "expand",
                label: shell.isExpanded ? "shrink editor" : "expand editor",
                hint: "esc",
                run: toggleExpand,
            },
        ];

        if (file != undefined && isEdited(file)) {
            entries.push({
                id: "reset",
                label: `reset ${basename(file.path)} to original`,
                run: reset,
            });
        }
        for (const [index, output] of (file?.outputs ?? []).entries()) {
            entries.push({
                id: `target-${index}`,
                label: `show ${output.label}`,
                hint: "output",
                run: () => {
                    for (const [id, view] of Object.entries(layout().views)) {
                        if (view.kind === "output") {
                            setShell("layouts", layoutKind(), "views", id, "target", index);
                        }
                    }
                },
            });
        }

        // view management
        entries.push(
            { id: "add-editor", label: "new editor view", hint: "view", run: () => addView("editor") },
            { id: "add-output", label: "new output view", hint: "view", run: () => addView("output") },
            { id: "add-terminal", label: "new terminal view", hint: "view", run: () => addView("terminal") },
            { id: "add-explorer", label: "new explorer view", hint: "view", run: () => addView("explorer") },
        );

        return entries;
    });

    return (
        <div class="relative h-full min-h-0 min-w-0">
            <div
                class="grid min-h-0 min-w-0 grid-rows-[2.25rem_minmax(0,1fr)_1.75rem] overflow-hidden border border-destack-frame bg-editor-window text-editor-text"
                classList={{
                    "absolute inset-0": !shell.isExpanded,
                    "fixed inset-0 z-50": shell.isExpanded,
                }}
                ref={frame}
            >
                <TitleBar isExpanded={shell.isExpanded} onExpand={toggleExpand} />

                {/* Dock: user-arrangeable splits of view instances */}
                <div class="min-h-0 min-w-0">
                    <DockView
                        chrome={chrome}
                        node={layout().dock}
                        onClose={closeView}
                        onMove={(view, target, edge) =>
                            setShell("layouts", layoutKind(), "dock", (dock) => moveView(dock, view, target, edge))
                        }
                        onResize={(path, index, delta) =>
                            setShell("layouts", layoutKind(), "dock", (dock) => resizeSplit(dock, path, index, delta))
                        }
                    />
                </div>

                <DragGhost />

                <StatusBar
                    cursor={shell.cursor}
                    file={activeFile()?.path ?? defaultPath}
                    isEdited={activeFile() != undefined && isEdited(activeFile()!)}
                    lines={activeFile()?.source.split("\n").length ?? 0}
                    onReset={resetWorkspace}
                />

                {/* Command bar overlay */}
                <Show when={shell.isCommandOpen}>
                    <CommandBar
                        commands={commands()}
                        files={Object.keys(workspace.files)}
                        onClose={() => setShell("isCommandOpen", false)}
                        onOpenFile={open}
                    />
                </Show>
            </div>
        </div>
    );
}

type TitleBarProps = {
    isExpanded: boolean;
    onExpand: () => void;
};

function TitleBar(props: TitleBarProps) {
    return (
        <header class="grid min-w-0 grid-cols-[1fr_auto_1fr] items-center gap-3 border-b border-editor-line bg-destack-ink px-3 text-xs font-medium text-destack-cream">
            {/* Window controls, the green one toggles fullscreen */}
            <div class="flex min-w-0 items-center gap-4">
                <div class="flex shrink-0 items-center gap-1.5">
                    <span aria-hidden="true" class="size-3 rounded-full bg-[#ff5f57]" />
                    <span aria-hidden="true" class="size-3 rounded-full bg-[#febc2e]" />
                    <button
                        aria-label={props.isExpanded ? "shrink the workspace" : "expand the workspace"}
                        class="size-3 rounded-full bg-[#28c840] hover:brightness-110"
                        onClick={props.onExpand}
                        title={props.isExpanded ? "shrink" : "expand"}
                        type="button"
                    />
                </div>
                <span class="truncate text-destack-cream/70">workspace</span>
            </div>

            <span class="hidden min-w-0 truncate text-destack-cream/85 lowercase md:block">
                engineer impeccable software with confidence
            </span>
        </header>
    );
}

type StatusBarProps = {
    cursor: Cursor;
    file: string;
    isEdited: boolean;
    lines: number;
    onReset: () => void;
};

function StatusBar(props: StatusBarProps) {
    return (
        <footer class="flex min-w-0 items-stretch overflow-hidden border-t border-editor-line bg-destack-ink text-xs font-medium lowercase text-destack-cream/80">
            <StatusCell class="bg-destack-accent font-semibold text-destack-ink">destack</StatusCell>
            <StatusCell class="min-w-0 flex-1 shrink truncate normal-case">
                {props.file}
                {props.isEdited ? " ●" : ""}
            </StatusCell>
            <StatusCell>{props.lines} lines</StatusCell>
            <StatusCell>
                ln {props.cursor.line}, col {props.cursor.column}
            </StatusCell>

            {/* Workspace reset, tucked into the corner */}
            <button
                class="flex min-h-7 shrink-0 items-center px-3 text-destack-cream/60 hover:text-destack-accent"
                onClick={props.onReset}
                type="button"
            >
                reset
            </button>
        </footer>
    );
}

type StatusCellProps = {
    children: string | number | (string | number)[];
    class?: string;
};

function StatusCell(props: StatusCellProps) {
    return (
        <span
            class={`flex min-h-7 shrink-0 items-center border-r border-editor-line px-3 ${props.class ?? ""}`}
        >
            {props.children}
        </span>
    );
}

function animateTransform(element: HTMLElement, transform: string) {
    // jump to the inverse transform, then transition back to identity
    element.style.transformOrigin = "top left";
    element.style.transition = "none";
    element.style.transform = transform;
    void element.offsetWidth;

    element.style.transition = `transform ${expandDurationMs}ms cubic-bezier(0.2, 0, 0, 1)`;
    element.style.transform = "";
    setTimeout(() => {
        element.style.transition = "";
    }, expandDurationMs + 20);
}

function normalizePath(path: string) {
    const parts: string[] = [];
    for (const part of path.split("/")) {
        if (part === "" || part === ".") {
            continue;
        }

        if (part === "..") {
            parts.pop();
            continue;
        }

        parts.push(part);
    }

    return parts.join("/");
}
