import { createStore, type SetStoreFunction } from "solid-js/store";

import { workspaceFiles } from "../generated/workspace";

/// One compiled artifact of a workspace file shown in the output pane.
export type FileOutput = {
    /// Tab label, like `primitives.js`.
    label: string;
    /// Build-time highlighted html.
    html: string;
};

/// One file in the workspace tree.
export type WorkspaceFile = {
    /// Workspace-relative path, like `language/types/primitives.ds`.
    path: string;
    kind: "source" | "markdown";
    /// Current contents, mutated by the editor.
    source: string;
    /// Build-time contents, the reset target.
    pristine: string;
    /// Build-time highlighted html used until the editor hydrates.
    html: string;
    /// Compiled artifacts (`.js`, `.asm`), empty for markdown.
    outputs: readonly FileOutput[];
    /// Captured `destack run` terminal output, highlighted.
    terminal: string | undefined;
};

/// The workspace: a virtual file system seeded at build time.
export type Workspace = {
    files: Record<string, WorkspaceFile>;
};

export type WorkspaceStore = [Workspace, SetStoreFunction<Workspace>];

const storageKey = "destack-workspace";

/// Seeded paths, the baseline that edits, additions, and deletions diff against.
const seededPaths = new Set<string>();

/// User changes persisted across visits.
type PersistedWorkspace = {
    /// Diverged sources of seeded files.
    edits: Record<string, string>;
    /// Sources of user-created files.
    created: Record<string, string>;
    /// Seeded files the user deleted.
    deleted: readonly string[];
};

/// Build the pristine seed files from the generated workspace tree.
export function seedFiles(): Record<string, WorkspaceFile> {
    const files: Record<string, WorkspaceFile> = {};

    for (const seed of workspaceFiles) {
        files[seed.path] = {
            path: seed.path,
            kind: seed.kind === "markdown" ? "markdown" : "source",
            source: seed.source,
            pristine: seed.source,
            html: seed.html,
            outputs: [],
            terminal: undefined,
        };
    }
    for (const path of Object.keys(files)) {
        seededPaths.add(path);
    }

    return files;
}

/// Create the demo workspace store from the pristine seed.
export function createWorkspace(): WorkspaceStore {
    return createStore({ files: seedFiles() });
}

/// Reapply persisted user changes, called after hydration.
export function restoreWorkspace(store: WorkspaceStore) {
    const raw = localStorage.getItem(storageKey);
    if (raw == undefined) {
        return;
    }

    const [, setWorkspace] = store;
    const persisted = JSON.parse(raw) as PersistedWorkspace;

    // replay deletions, divergent edits, and user files over the seed
    for (const path of persisted.deleted) {
        deleteFile(store, path);
    }
    for (const [path, source] of Object.entries(persisted.edits)) {
        setWorkspace("files", path, "source", source);
    }
    for (const [path, source] of Object.entries(persisted.created)) {
        createFile(store, path);
        setWorkspace("files", path, "source", source);
    }
}

/// Persist the user's divergence from the seed.
export function persistWorkspace(workspace: Workspace) {
    const persisted: PersistedWorkspace = { edits: {}, created: {}, deleted: [] };

    // record diverged seeded files and user-created files
    for (const file of Object.values(workspace.files)) {
        if (!seededPaths.has(file.path)) {
            persisted.created[file.path] = file.source;
        } else if (isEdited(file)) {
            persisted.edits[file.path] = file.source;
        }
    }

    // record deleted seeded files
    persisted.deleted = [...seededPaths].filter((path) => workspace.files[path] == undefined);

    localStorage.setItem(storageKey, JSON.stringify(persisted));
}

/// Create an empty source file, returns false when the path is taken.
export function createFile(store: WorkspaceStore, path: string) {
    const [workspace, setWorkspace] = store;
    if (workspace.files[path] != undefined) {
        return false;
    }

    setWorkspace("files", path, {
        path,
        kind: path.endsWith(".md") ? "markdown" : "source",
        source: "",
        pristine: "",
        html: "",
        outputs: [],
        terminal: undefined,
    });

    return true;
}

/// Move a file to a new path, returns false when the target is taken.
export function renameFile(store: WorkspaceStore, from: string, to: string) {
    const [workspace, setWorkspace] = store;
    const file = workspace.files[from];
    if (file == undefined || from === to || workspace.files[to] != undefined) {
        return false;
    }

    setWorkspace("files", to, { ...file, path: to });
    setWorkspace("files", from, undefined!);

    return true;
}

/// Remove a file from the workspace.
export function deleteFile(store: WorkspaceStore, path: string) {
    const [, setWorkspace] = store;
    setWorkspace("files", path, undefined!);
}

/// File name of a workspace path.
export function basename(path: string) {
    return path.split("/").at(-1) ?? path;
}

/// Directory prefix of a workspace path, empty at the root.
export function dirname(path: string) {
    return path.split("/").slice(0, -1).join("/");
}

/// Whether a file has unsaved divergence from its build-time contents.
export function isEdited(file: WorkspaceFile) {
    return file.source !== file.pristine;
}
