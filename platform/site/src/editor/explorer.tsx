import { For, Show, createMemo, createSignal } from "solid-js";

import { beginDrag, dragPayload, dropTarget } from "./drag";
import { basename, dirname, type Workspace } from "./workspace";

/// One directory in the explorer tree with its files and subdirectories.
type TreeDirectory = {
    path: string;
    directories: TreeDirectory[];
    files: string[];
};

/// An open context menu, anchored at the pointer.
type Menu = {
    isDirectory: boolean;
    path: string;
    x: number;
    y: number;
};

export type ExplorerProps = {
    activePath: string;
    closedDirectories: readonly string[];
    onCreate: (directory: string) => void;
    onDelete: (path: string) => void;
    onMove: (path: string, directory: string) => void;
    onOpen: (path: string) => void;
    onRename: (from: string, to: string) => void;
    onToggleDirectory: (path: string) => void;
    workspace: Workspace;
};

export function Explorer(props: ExplorerProps) {
    const root = createMemo(() => buildTree(Object.keys(props.workspace.files)));
    const [menu, setMenu] = createSignal<Menu | undefined>(undefined);
    const [renamingPath, setRenamingPath] = createSignal<string | undefined>(undefined);

    const isOpen = (path: string) => !props.closedDirectories.includes(path);
    const toggle = props.onToggleDirectory;

    // context menus open per row, any click elsewhere dismisses
    const openMenu = (event: MouseEvent, path: string, isDirectory: boolean) => {
        event.preventDefault();
        setMenu({ isDirectory, path, x: event.clientX, y: event.clientY });
    };
    const commitRename = (path: string, name: string) => {
        setRenamingPath(undefined);

        const trimmed = name.trim();
        if (trimmed !== "" && trimmed !== basename(path)) {
            const directory = dirname(path);
            props.onRename(path, directory === "" ? trimmed : `${directory}/${trimmed}`);
        }
    };

    return (
        <aside
            class="relative flex h-full min-h-0 min-w-0 flex-col overflow-auto bg-editor-shade"
            data-drop-directory=""
            onClick={() => setMenu(undefined)}
        >
            {/* Tree header with the root create affordance */}
            <div class="flex items-center justify-between border-b border-editor-line py-1 pr-7 pl-3 text-xs text-editor-muted/80 lowercase">
                <span>workspace</span>
                <button
                    aria-label="new file"
                    class="px-1 text-sm hover:text-editor-text"
                    onClick={() => props.onCreate("")}
                    type="button"
                >
                    +
                </button>
            </div>

            <nav
                class="grid py-1.5 text-[13px] leading-6 lowercase"
                onKeyDown={navigateTree}
                role="tree"
            >
                <For each={root().directories}>
                    {(directory) => (
                        <Directory
                            activePath={props.activePath}
                            depth={0}
                            directory={directory}
                            isOpen={isOpen}
                            onMove={props.onMove}
                            onOpen={props.onOpen}
                            onRename={commitRename}
                            onToggle={toggle}
                            renamingPath={renamingPath()}
                            rowMenu={openMenu}
                        />
                    )}
                </For>

                <For each={root().files}>
                    {(path) => (
                        <File
                            depth={0}
                            isActive={props.activePath === path}
                            isRenaming={renamingPath() === path}
                            onMove={props.onMove}
                            onOpen={() => props.onOpen(path)}
                            onRename={(name) => commitRename(path, name)}
                            path={path}
                            rowMenu={openMenu}
                        />
                    )}
                </For>
            </nav>

            {/* Context menu */}
            <Show when={menu()}>
                {(open) => (
                    <div
                        class="fixed z-50 grid min-w-36 border border-destack-frame bg-editor-window py-1 text-xs text-editor-text shadow-xl"
                        style={`left: ${open().x}px; top: ${open().y}px`}
                    >
                        <Show when={open().isDirectory}>
                            <MenuItem
                                label="new file"
                                onClick={() => {
                                    setMenu(undefined);
                                    props.onCreate(open().path);
                                }}
                            />
                        </Show>
                        <Show when={!open().isDirectory}>
                            <MenuItem
                                label="rename"
                                onClick={() => {
                                    setMenu(undefined);
                                    setRenamingPath(open().path);
                                }}
                            />
                            <MenuItem
                                label="delete"
                                onClick={() => {
                                    setMenu(undefined);
                                    props.onDelete(open().path);
                                }}
                            />
                        </Show>
                    </div>
                )}
            </Show>
        </aside>
    );
}

type MenuItemProps = {
    label: string;
    onClick: () => void;
};

function MenuItem(props: MenuItemProps) {
    return (
        <button
            class="px-3 py-1 text-left lowercase hover:bg-destack-accent/15"
            onClick={props.onClick}
            type="button"
        >
            {props.label}
        </button>
    );
}

type DirectoryProps = {
    activePath: string;
    depth: number;
    directory: TreeDirectory;
    isOpen: (path: string) => boolean;
    onMove: (path: string, directory: string) => void;
    onOpen: (path: string) => void;
    onRename: (path: string, name: string) => void;
    onToggle: (path: string) => void;
    renamingPath: string | undefined;
    rowMenu: (event: MouseEvent, path: string, isDirectory: boolean) => void;
};

function Directory(props: DirectoryProps) {
    const isExpanded = () => props.isOpen(props.directory.path);
    const isDropTarget = () => {
        const target = dropTarget();
        return (
            dragPayload()?.kind === "file" &&
            target?.kind === "directory" &&
            target.path === props.directory.path
        );
    };

    return (
        <div class="grid" role="treeitem">
            <button
                aria-expanded={isExpanded()}
                class="flex h-6 min-w-0 items-center gap-1 pr-4 text-left text-editor-muted outline-none hover:text-editor-text focus-visible:ring-1 focus-visible:ring-destack-accent focus-visible:ring-inset"
                classList={{ "bg-destack-accent/15": isDropTarget() }}
                data-drop-directory={props.directory.path}
                data-tree-node
                onClick={() => props.onToggle(props.directory.path)}
                onContextMenu={(event) => props.rowMenu(event, props.directory.path, true)}
                style={`padding-left: ${0.75 + props.depth * 0.875}rem`}
                type="button"
            >
                <span aria-hidden="true" class="w-3 shrink-0 text-[10px] text-editor-muted/60">
                    {isExpanded() ? "▾" : "▸"}
                </span>
                <span class="min-w-0 truncate">{basename(props.directory.path)}/</span>
            </button>

            <Show when={isExpanded()}>
                <div class="grid" role="group">
                    <For each={props.directory.directories}>
                        {(child) => (
                            <Directory
                                activePath={props.activePath}
                                depth={props.depth + 1}
                                directory={child}
                                isOpen={props.isOpen}
                                onMove={props.onMove}
                                onOpen={props.onOpen}
                                onRename={props.onRename}
                                onToggle={props.onToggle}
                                renamingPath={props.renamingPath}
                                rowMenu={props.rowMenu}
                            />
                        )}
                    </For>

                    <For each={props.directory.files}>
                        {(path) => (
                            <File
                                depth={props.depth + 1}
                                isActive={props.activePath === path}
                                isRenaming={props.renamingPath === path}
                                onMove={props.onMove}
                                onOpen={() => props.onOpen(path)}
                                onRename={(name) => props.onRename(path, name)}
                                path={path}
                                rowMenu={props.rowMenu}
                            />
                        )}
                    </For>
                </div>
            </Show>
        </div>
    );
}

type FileProps = {
    depth: number;
    isActive: boolean;
    isRenaming: boolean;
    onMove: (path: string, directory: string) => void;
    onOpen: () => void;
    onRename: (name: string) => void;
    path: string;
    rowMenu: (event: MouseEvent, path: string, isDirectory: boolean) => void;
};

function File(props: FileProps) {
    // drag the row into another directory via pointer capture
    const grab = (event: PointerEvent) => {
        if (props.isRenaming) {
            return;
        }

        beginDrag(event, { kind: "file", path: props.path, label: basename(props.path) }, (target) => {
            if (target?.kind === "directory" && target.path !== dirname(props.path)) {
                props.onMove(props.path, target.path);
            }
        });
    };

    return (
        <button
            aria-selected={props.isActive}
            class="flex h-6 min-w-0 items-center pr-4 text-left outline-none focus-visible:ring-1 focus-visible:ring-destack-accent focus-visible:ring-inset"
            classList={{
                "bg-destack-accent/12 text-editor-text shadow-[inset_2px_0_0_var(--color-destack-accent)]":
                    props.isActive,
                "text-editor-muted hover:bg-editor-line/40 hover:text-editor-text": !props.isActive,
            }}
            data-drop-directory={dirname(props.path)}
            data-tree-node
            onClick={props.onOpen}
            onContextMenu={(event) => props.rowMenu(event, props.path, false)}
            onPointerDown={grab}
            style={`padding-left: ${1.75 + props.depth * 0.875}rem`}
            type="button"
        >
            {/* Inline rename swaps the label for an input */}
            <Show
                fallback={<span class="min-w-0 truncate">{basename(props.path)}</span>}
                when={props.isRenaming}
            >
                <input
                    class="min-w-0 border border-destack-accent bg-editor-window px-1 text-editor-text outline-none"
                    onBlur={(event) => props.onRename(event.currentTarget.value)}
                    onClick={(event) => event.stopPropagation()}
                    onKeyDown={(event) => {
                        if (event.key === "Enter") {
                            props.onRename(event.currentTarget.value);
                        } else if (event.key === "Escape") {
                            props.onRename(basename(props.path));
                        }
                        event.stopPropagation();
                    }}
                    onPointerDown={(event) => event.stopPropagation()}
                    ref={(element) => requestAnimationFrame(() => element.select())}
                    spellcheck={false}
                    value={basename(props.path)}
                />
            </Show>
        </button>
    );
}

function buildTree(paths: readonly string[]) {
    const root: TreeDirectory = { path: "", directories: [], files: [] };
    const directories = new Map<string, TreeDirectory>([["", root]]);

    // materialize a directory node for every path prefix
    const directoryFor = (path: string): TreeDirectory => {
        const existing = directories.get(path);
        if (existing != undefined) {
            return existing;
        }

        const directory: TreeDirectory = { path, directories: [], files: [] };
        directories.set(path, directory);
        directoryFor(dirname(path)).directories.push(directory);

        return directory;
    };

    // readmes pin first within each directory, the rest keeps seed order
    for (const path of paths) {
        const directory = directoryFor(dirname(path));
        if (basename(path) === "README.md") {
            directory.files.unshift(path);
        } else {
            directory.files.push(path);
        }
    }

    return root;
}

function navigateTree(event: KeyboardEvent) {
    const tree = event.currentTarget as HTMLElement;
    const nodes = [...tree.querySelectorAll<HTMLButtonElement>("[data-tree-node]")];
    const index = nodes.findIndex((node) => node === document.activeElement);
    if (index < 0) {
        return;
    }

    if (event.key === "ArrowDown") {
        event.preventDefault();
        nodes[Math.min(index + 1, nodes.length - 1)]?.focus();
    } else if (event.key === "ArrowUp") {
        event.preventDefault();
        nodes[Math.max(index - 1, 0)]?.focus();
    } else if (event.key === "ArrowRight") {
        const node = nodes[index];
        if (node?.getAttribute("aria-expanded") === "false") {
            event.preventDefault();
            node.click();
        }
    } else if (event.key === "ArrowLeft") {
        const node = nodes[index];
        if (node?.getAttribute("aria-expanded") === "true") {
            event.preventDefault();
            node.click();
        }
    }
}
