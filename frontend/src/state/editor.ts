import { graphql } from "@/gql";
import {
  StatementModifier,
  SymbolType,
  TypeTag,
  type File,
  type Project,
  type ProjectVersion,
  type Statement,
} from "@/gql/graphql";
import { useAppearanceState, type Theme } from "@/state/appearance";
import { useNotifications } from "@/state/notifications";
import { reverseRecord } from "@/utils/functools";
import { useLazyQuery } from "@vue/apollo-composable";
import { defineStore } from "pinia";
import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";

export type ProjectHeader = Pick<Project, "id" | "name" | "slug" | "canWrite" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<
  File,
  "__typename" | "id" | "name" | "path" | "createdAt" | "updatedAt" | "deletedAt" | "directory" | "generated"
>;
export type StatementHeader = Pick<
  Statement,
  | "__typename"
  | "id"
  | "modifier"
  | "type"
  | "symbolType"
  | "name"
  | "createdAt"
  | "updatedAt"
  | "deletedAt"
  | "orderKey"
  | "generated"
  | "commented"
  | "parent"
>;

// sync with language in backend
export const SYMBOL_TYPE_KEYWORD: Record<SymbolType, string> = {
  [SymbolType.Type]: "type",
  [SymbolType.Code]: "code",
  [SymbolType.Data]: "data",
  [SymbolType.Model]: "model",
  [SymbolType.Expectation]: "expect",
  [SymbolType.Task]: "task",
  [SymbolType.Value]: "value",
  [SymbolType.Capability]: "capability",
  [SymbolType.Requirement]: "require",
  [SymbolType.Runconfig]: "run",
  [SymbolType.Build]: "build",
};
export const SYMBOL_TYPE_BY_KEYWORD: Record<string, SymbolType> = reverseRecord(SYMBOL_TYPE_KEYWORD);
export const MODIFIER_KEYWORD: Record<StatementModifier, string> = {
  [StatementModifier.Like]: "like",
  [StatementModifier.Unlike]: "unlike",
  [StatementModifier.Check]: "check",
  [StatementModifier.With]: "with",
  [StatementModifier.Var]: "vary",
  [StatementModifier.Include]: "include",
  [StatementModifier.Magic]: "magic",
};
export const MODIFIER_BY_KEYWORD: Record<string, StatementModifier> = reverseRecord(MODIFIER_KEYWORD);
export const TYPETAG_KEYWORD: Record<TypeTag, string> = {
  [TypeTag.Any]: "anything",
  [TypeTag.Null]: "nothing",
  [TypeTag.Boolean]: "boolean",
  [TypeTag.String]: "text",
  [TypeTag.Number]: "number",
  [TypeTag.Array]: "list",
  [TypeTag.TypeReference]: "reference",
  [TypeTag.Enum]: "enum",
  [TypeTag.Map]: "map",
};
export const TYPETAG_BY_KEYWORD: Record<string, TypeTag> = reverseRecord(TYPETAG_KEYWORD);

export type ViewId = "explorer" | "history" | "issues";

export type Editor = {
  type: "file" | "run";
  id: string;
  path: string;
  scroll?: { x: number; y: number };
  localState: Record<string, unknown>; // opaque (JSONable) local state for each editor
  groupId: string | null; // id instead of EditorGroup to avoid circular dependency
};

export const EDITOR_INTERFACE_STATE = Symbol();
export type EditorInterfaceState = {
  get<T>(key: string, default_?: T): T;
  set<T>(key: string, value: T): void;
};

export type FileEditor = Editor & {
  type: "file";
  fileId: string;
};

export type RunEditor = Editor & {
  type: "run";
  symbolId: string;
  symbolType: SymbolType;
};

export type EditorGroup = {
  id: string;
  name: string;
  editors: Editor[];
  activeEditorId: string | null;
};

function makeEditorGroup(id: string, name: string): EditorGroup {
  return {
    id,
    name,
    editors: [],
    activeEditorId: null,
  };
}

export function makeFileEditor(file: FileHeader): FileEditor {
  return {
    // append random string to enable multiple editors for the same file
    id: file.id + "-" + Math.random().toString(16).substring(2, 8),
    type: "file",
    fileId: file.id,
    path: file.path,
    localState: {},
    groupId: null,
  } as FileEditor;
}

export function makeRunEditor(symbol: { id: string; name: string; symbolType: SymbolType }): RunEditor {
  return {
    // append random string to enable multiple editors for the same symbol
    id: symbol.id + "-" + Math.random().toString(16).substring(2, 8),
    type: "run",
    symbolId: symbol.id,
    symbolType: symbol.symbolType,
    path: "run " + symbol.name,
    localState: {},
    groupId: null,
  } as RunEditor;
}

export const useEditorState = defineStore("editor", {
  state: () => {
    return {
      // note that editor state should be JSON serializable
      currentProjectId: null as string | null,
      currentProjectVersionId: null as string | null,
      activeViewId: "explorer" as ViewId,
      left: makeEditorGroup("left", "Left"),
      right: makeEditorGroup("right", "Right"),
      focusedEditorId: null as string | null,
      focusedElementId: null as string | null,
      focusedElementType: null as string | null,
      mainSymbolId: null as string | null,
      editingElement: false,
      readonly: false,
      debug: false,
      showGenerated: false,
      showLineNumbers: true,
      showEditorGroupHeader: true,
      showGlobalHeader: true,
      showViewSelection: true,
      showViewContent: false,
      zenMode: false,
    };
  },
  getters: {
    editorGroups(state) {
      return [state.left, state.right];
    },
    editorGroup(): (id: string) => EditorGroup {
      return (id: string) => {
        const group = this.editorGroups.find((g) => g.id == id);
        if (group == null) throw new Error(`editor group ${id} not found`);
        return group;
      };
    },
    editors(state) {
      return state.left.editors.concat(state.right.editors);
    },
    focusedEditor(): Editor | undefined {
      return this.editors.find((e) => e.id == this.focusedEditorId);
    },
    focusedFileId(): string | null {
      return this.focusedEditor?.type == "file" ? (this.focusedEditor as FileEditor).fileId : null;
    },
    focusedRunId(): string | null {
      return this.focusedEditor?.type == "run" ? (this.focusedEditor as RunEditor).symbolId : null;
    },
    focusedGroup(): EditorGroup | undefined {
      if (this.focusedEditor?.groupId == null) return undefined;
      return this.editorGroup(this.focusedEditor?.groupId);
    },
    theme(): Theme {
      const appearance = useAppearanceState();
      return appearance.theme;
    },
    textSmall(): boolean {
      const appearance = useAppearanceState();
      return appearance.textSmall;
    },
    fullscreen(): boolean {
      const appearance = useAppearanceState();
      return appearance.fullscreen;
    },
    fontMono(): boolean {
      const appearance = useAppearanceState();
      return appearance.fontMono;
    },
  },
  actions: {
    setProject(projectId: string, versionId: string): void {
      this.$reset();
      this.currentProjectId = projectId;
      this.currentProjectVersionId = versionId;
    },

    setActiveView(viewId: ViewId): void {
      this.activeViewId = viewId;
    },

    openActiveView(viewId: ViewId): void {
      this.activeViewId = viewId;
      this.showViewContent = true;
    },

    setEditorScroll(editor: Editor, scroll: { x: number; y: number }): void {
      editor.scroll = scroll;
    },

    setEditorState(editor: Editor, key: string, value: unknown): void {
      editor.localState[key] = value;
    },

    _removeEditorFromGroup(editor: Editor): void {
      if (editor.groupId == null) return;
      const group = this.editorGroup(editor.groupId);
      if (group == null) return;
      group.editors = group.editors.filter((e) => e != editor);
      if (group.activeEditorId == editor.id) {
        // if active editor was removed, set first editor as active
        group.activeEditorId = group.editors[0]?.id || null;
        // if editor was focused, focus new active editor
        if (editor.id == this.focusedEditorId) {
          this.focusedEditorId = group.activeEditorId;
          // blur element
          this.focusedElementId = null;
        }
      }
      editor.groupId = null;
    },

    openEditor(editor: Editor, group?: EditorGroup): Editor {
      // if group wasn't passed, just return the editor if it's already open
      if (group == null && editor.groupId != null) {
        return editor;
      }
      console.log(`open editor ${editor.path} in group ${group?.id}`);
      group = group || this.focusedGroup || this.left;
      // change editor group if different
      if (editor.groupId != group.id) {
        if (editor.groupId != null) {
          // remove from old group
          this._removeEditorFromGroup(editor);
        }
        editor.groupId = group.id;
        group.editors.push(editor);
      }
      return editor;
    },

    closeEditor(editor: Editor): void {
      console.log(`close editor ${editor.path}`);
      if (editor.groupId != null) {
        this._removeEditorFromGroup(editor);
      }
    },

    moveEditor(editor: Editor, group: EditorGroup): void {
      const wasFocused = editor == this.focusedEditor;
      this.openEditor(editor, group);
      if (wasFocused) {
        this.focusEditor(editor);
      }
    },

    openFile(file: FileHeader, group?: EditorGroup): Editor {
      let editor = this.editors.find((e) => e.type == "file" && (e as FileEditor).fileId == file.id);
      if (!editor) {
        console.log(`create new file editor for ${file.id} ${file.path}`);
        editor = makeFileEditor(file);
      }
      return this.openEditor(editor, group);
    },

    openRun(symbol: { id: string; name: string; symbolType: SymbolType }, group?: EditorGroup): Editor {
      let editor = this.editors.find((e) => e.type == "run" && (e as RunEditor).symbolId == symbol.id);
      if (!editor) {
        console.log(`create new run editor for ${symbol.id} ${symbol.name}`);
        editor = makeRunEditor(symbol);
      }
      return this.openEditor(editor, group);
    },

    focusEditor(editor: Editor): void {
      if (this.focusedEditor?.id == editor.id) return;

      console.log(`focus editor ${editor.path} in group ${editor.groupId}`);
      if (!editor.groupId) {
        throw new Error("editor must be in a group: " + editor.path);
      }
      this.focusedEditorId = editor.id;
      this.editorGroup(editor.groupId).activeEditorId = editor.id;
    },

    focusFile(file: FileHeader, group?: EditorGroup): Editor {
      const editor = this.openFile(file, group);
      this.focusEditor(editor);
      return editor;
    },

    focusElement(element: StatementHeader | FileHeader, retainEditing = false) {
      if (this.focusedElementId == element.id) return;
      console.log(`focus element ${element.id}`);
      this.focusedElementId = element.id;
      this.focusedElementType = element.__typename || null;
      this.editingElement = this.editingElement && retainEditing;
    },

    editElement(element: StatementHeader | FileHeader) {
      if (this.focusedElementId != element.id || !this.editingElement) {
        this.focusElement(element);
        this.editingElement = true;
        console.log(`edit element ${element.id}`);
      }
    },

    stopEditingElement(element?: StatementHeader | FileHeader) {
      if (!element || element.id == this.focusedElementId) {
        this.editingElement = false;
      }
      console.log(`stop editing element ${element?.id}`);
    },

    blurElement(element?: StatementHeader | FileHeader) {
      if (!element || element.id == this.focusedElementId) {
        this.focusedElementId = null;
        this.focusedElementType = null;
        this.editingElement = false;
      }
    },

    setMainSymbol(symbol?: { id: string }): void {
      console.log("set main symbol", symbol?.id);
      this.mainSymbolId = symbol?.id ?? null;
    },

    setZenMode(zenMode: boolean) {
      const appearance = useAppearanceState();
      this.zenMode = zenMode;
      this.showGlobalHeader = !zenMode;
      this.showEditorGroupHeader = !zenMode;
      this.showLineNumbers = !zenMode;
      this.showViewSelection = !zenMode;
      appearance.fullscreen = zenMode;
    },

    async _doMigrateTo(versionId: string, refMappings: Record<string, string>): Promise<void> {
      // migrate by serializing state and replacing refs
      let stateJson = JSON.stringify(this.$state);
      for (const [sourceId, targetId] of Object.entries(refMappings)) {
        // replace all matches of ref.source with ref.target
        // (need to use regex to replace *all* matches)
        const re = new RegExp(`"${sourceId}"`, "g");
        stateJson = stateJson.replace(re, `"${targetId}"`);
      }
      this.$reset();
      this.$patch(JSON.parse(stateJson));

      // remove editors with refs we don't have anymore
      // note that this also closes any module-external refs
      // I tried to fix this by only removing refs we _used_ tohave (checking for origianl ref.target)
      // but that doesn't work for refs that were just created in first source version.
      const targetRefs = refMappings.map((r) => r.targetId);
      for (const editor of this.editors) {
        let editorRef = null;
        if (editor.type == "file") {
          editorRef = (editor as FileEditor).fileId;
        } else if (editor.type == "run") {
          editorRef = (editor as RunEditor).symbolId;
        }
        console.log("editor ref", editorRef, targetRefs.includes(editorRef));
        if (editorRef != null && !targetRefs.includes(editorRef)) {
          console.log(`close outdated editor ${editor.path} (${editor.id} pointed to ${editorRef})`);
          this.closeEditor(editor);
        }
      }
      this.currentProjectVersionId = versionId;
    },
  },
});

export function useEditorPersistence(intervalMs = 1000) {
  const editor = useEditorState();

  const save = () => {
    // save editor state by project id
    if (editor.currentProjectId == null) return;
    localStorage.setItem(`editor-state-${editor.currentProjectId}`, JSON.stringify(editor.$state));
  };

  const load = () => {
    // load editor state by project id
    if (editor.currentProjectId == null) return;
    const state = localStorage.getItem(`editor-state-${editor.currentProjectId}`);
    if (state) {
      try {
        editor.$patch(JSON.parse(state));
        console.log(`restored editor state for project ${editor.currentProjectId}`);
      } catch (e) {
        console.error(`failed to restore editor state for project ${editor.currentProjectId}`);
      }
    }
  };

  // save every interval
  const interval = setInterval(save, intervalMs);
  onBeforeUnmount(() => clearInterval(interval));

  return { save, load };
}

export function useEditorMigrations() {
  const migratingTo: Ref<string | null> = ref(null);
  const notifications = useNotifications();
  const editor = useEditorState();

  // TODO @Cleanup: project ref migration from vx to vy should be a server-side API endpoint
  const {
    load: getProjectMigrationRefs,
    loading: migrationLoading,
    error: migrationError,
    result: migrationRefs,
  } = useLazyQuery(
    graphql(/* GraphQL */ `
      query projectMigrationRefs($projectId: GlobalID!, $fromId: GlobalID!, $toId: GlobalID!) {
        project(id: $projectId) {
          versions(filters: { fromId: $fromId, toId: $toId }) {
            totalCount
            edges {
              node {
                id
                name
                tag
                createdAt
                parentRefs {
                  edges {
                    node {
                      sourceId
                      targetId
                    }
                  }
                }
              }
            }
          }
        }
      }
    `)
  );

  // complete the migration once we've gotten the refs
  watch(
    () => [migrationRefs, migrationLoading, migrationError],
    async () => {
      if (migratingTo.value == null) return;
      if (migrationLoading.value) return;

      if (migrationError.value != null) {
        // fail migration
        editor.currentProjectVersionId = migratingTo.value;
        console.error("unable to migrate, error getting intermediate refs", migrationError.value);
        notifications.show({
          kind: "warning",
          type: "editorMigration.failed",
          message: "Migrating editor failed",
          description: "Editor could not be migrated (local only).",
        });
        migratingTo.value = null;
      } else if (migrationRefs.value != null) {
        // got the intermediate ref mappings, do actual migration
        const intermediateVersions = [...(migrationRefs.value?.project?.versions.edges.map((e) => e.node) ?? [])];

        let intermediateRefs: { sourceId: string; targetId: string }[][];
        if (intermediateVersions[0].id == migratingTo.value) {
          // migrate backwards to an older version (reverse everything)
          intermediateVersions.sort((a, b) => (a.createdAt < b.createdAt ? 1 : -1));
          intermediateRefs = intermediateVersions
            .slice(0, -1) // skip the last target/source mapping as that would go 1 version further
            .map((v) =>
              v.parentRefs.edges.map((r) => r.node).map((r) => ({ sourceId: r.targetId, targetId: r.sourceId }))
            );
        } else {
          // migrate forwards
          intermediateVersions.sort((a, b) => (a.createdAt < b.createdAt ? -1 : 1));
          intermediateRefs = intermediateVersions.map((v) => v.parentRefs.edges.map((r) => r.node));
        }

        // intermediate refs are successive source -> target pairs
        // reduce into the final first source -> last target pair
        const refMappings: Record<string, string> = {};

        // init with first source -> target mapping
        for (const { sourceId, targetId } of intermediateRefs[0]) {
          refMappings[sourceId] = targetId;
        }
        // update target id over each intermediate source -> target mapping
        for (let i = 1; i < intermediateRefs.length; i++) {
          // reverse target -> source mapping up til now
          const reverseRefMappings: Record<string, string> = {};
          for (const sourceId of Object.keys(refMappings)) {
            reverseRefMappings[refMappings[sourceId]] = sourceId;
          }
          for (const { sourceId, targetId } of intermediateRefs[i]) {
            // this new source was the previous target
            const firstSourceId = reverseRefMappings[sourceId];
            if (firstSourceId != null) {
              refMappings[firstSourceId] = targetId;
            }
          }
        }

        await editor._doMigrateTo(migratingTo.value, refMappings);
        console.log(
          `migrated through ${intermediateVersions?.map((v) => `${v.name ?? "(Working)"} (${v.tag ?? v.id})`)}`
        );
        migratingTo.value = null;
      }
    },
    { deep: true }
  );

  function migrateTo(projectId: string, toVersionId: string, fromVersionId: string) {
    if (migratingTo.value != null) {
      if (migratingTo.value == toVersionId) {
        // nothing to do
        return;
      } else {
        throw new Error(`already migrating to another version: ${migratingTo.value} (not ${toVersionId})`);
      }
    }
    migratingTo.value = toVersionId;
    console.log(
      `migrate editor state for project ${projectId} to version ${toVersionId} (from version ${fromVersionId}))`
    );
    // get all ref mappings
    getProjectMigrationRefs(
      undefined,
      {
        projectId: projectId,
        fromId: fromVersionId,
        toId: toVersionId,
      },
      { fetchPolicy: "network-only" }
    );
  }

  return {
    migrating: computed(() => migratingTo.value != null),
    migrateTo,
  };
}
