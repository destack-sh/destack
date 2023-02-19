import {
  StatementModifier,
  SymbolType,
  TypeTag,
  type File,
  type Project,
  type ProjectVersion,
  type RefMapping,
  type Statement,
} from "@/gql/graphql";
import { useNotifications } from "@/state/notifications";
import { reverseRecord } from "@/utils/functools";
import { useLazyQuery } from "@vue/apollo-composable";
import { defineStore } from "pinia";
import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";
import { graphql } from "@/gql";

export type ProjectHeader = Pick<Project, "id" | "name" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<
  File,
  "__typename" | "id" | "name" | "path" | "createdAt" | "updatedAt" | "deletedAt" | "generated"
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
  activeEditor: Editor | null;
};

function makeEditorGroup(id: string, name: string): EditorGroup {
  return {
    id,
    name,
    editors: [],
    activeEditor: null,
  };
}

export function makeFileEditor(file: FileHeader): FileEditor {
  return {
    // append random string to enable multiple editors for the same file
    id: file.id + "-" + Math.random().toString(16).substring(2, 8),
    type: "file",
    fileId: file.id,
    path: file.path + ".x",
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
      focusedEditor: null as Editor | null,
      focusedElementId: null as string | null,
      focusedElementType: null as string | null,
      mainSymbolId: null as string | null,
      editingElement: false,
      readonly: false,
      debug: false,
      fullscreen: false,
      showGenerated: false,
      showLineNumbers: true,
      showEditorGroupHeader: true,
      showGlobalHeader: true,
      showViewSelection: true,
      showViewContent: true,
      zenMode: false,
      fontMono: false,
      textSmall: true,
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
    focusedFileId(state): string | null {
      return state.focusedEditor?.type == "file" ? (state.focusedEditor as FileEditor).fileId : null;
    },
    focusedRunId(state): string | null {
      return state.focusedEditor?.type == "run" ? (state.focusedEditor as RunEditor).symbolId : null;
    },
    focusedGroup(): EditorGroup | undefined {
      if (this.focusedEditor?.groupId == null) return undefined;
      return this.editorGroup(this.focusedEditor?.groupId);
    },
  },
  actions: {
    setProject(project: ProjectHeader, version: ProjectVersionHeader): void {
      this.$reset();
      this.currentProjectId = project.id;
      this.currentProjectVersionId = version.id;
    },

    setActiveView(viewId: ViewId): void {
      this.activeViewId = viewId;
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
      if (group.activeEditor == editor) {
        // if active editor was removed, set first editor as active
        group.activeEditor = group.editors[0] || null;
        // if editor was focused, focus new active editor
        if (editor == this.focusedEditor) {
          this.focusedEditor = group.activeEditor;
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
      this.focusedEditor = editor;
      this.editorGroup(editor.groupId).activeEditor = editor;
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
      this.zenMode = zenMode;
      this.showGlobalHeader = !zenMode;
      this.showEditorGroupHeader = !zenMode;
      this.showLineNumbers = !zenMode;
      this.showViewSelection = !zenMode;
      this.fullscreen = zenMode;
    },

    async _doMigrateTo(versionId: string, intermediateRefs: RefMapping[][]): Promise<void> {
      // migrate by serializing state and replacing refs
      let stateJson = JSON.stringify(this.$state);
      for (const refs of intermediateRefs) {
        for (const ref of refs) {
          // replace all matches of ref.source with ref.target
          // (need to use regex to replace *all* matches)
          const re = new RegExp(`"${ref.source}"`, "g");
          stateJson = stateJson.replace(re, `"${ref.target}"`);
        }
      }
      this.$reset();
      this.$patch(JSON.parse(stateJson));

      // remove editors with refs we don't have anymore
      const latestRefs = intermediateRefs[intermediateRefs.length - 1].map((r) => r.target);
      for (const editor of this.editors) {
        if (editor.type == "file" && !latestRefs.includes((editor as FileEditor).fileId)) {
          console.log(`removing outdated editor for file ${(editor as FileEditor).fileId}`);
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

  const {
    load: getProjectMigrationRefs,
    loading: migrationLoading,
    error: migrationError,
    result: migrationRefs,
  } = useLazyQuery(
    graphql(/* GraphQL */ `
      query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {
        project(id: $projectId) {
          versions(filters: { afterId: $afterId }) {
            id
            name
            createdAt
            parentsRefs {
              source
              target
            }
          }
        }
      }
    `)
  );

  // the second half of applying a migration (since we can't await lazy queries directly)
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
          type: "editorMigration.fail",
          message: "Migrating editor failed",
          description: "Editor could not be migrated (local only).",
        });
        migratingTo.value = null;
      } else if (migrationRefs.value != null) {
        // got the intermediate ref mappings, do actual migration
        const intermediateVersions = [...(migrationRefs.value?.project?.versions ?? [])];
        const intermediateRefs = intermediateVersions
          ?.sort((a, b) => a.createdAt - b.createdAt)
          .map((v) => v.parentsRefs);
        await editor._doMigrateTo(migratingTo.value, intermediateRefs);

        console.log(`migrated through ${intermediateVersions?.map((v) => v.id)} intermediate versions`);
        notifications.show({
          kind: "success",
          type: "editorMigration.success",
          message: "Migrated editor",
          description: "Editor migrated to new project version.",
        });
        migratingTo.value = null;
      }
    },
    { deep: true }
  );

  function migrateTo(projectId: string, toVersionId: string, fromVersionId: string) {
    if (migratingTo.value != null) {
      throw new Error("already migrating");
    }
    // migrating flag triggers migration
    migratingTo.value = toVersionId;
    console.log(
      `migrate editor state for project ${projectId} to version ${toVersionId} (from version ${fromVersionId}))`
    );
    // get all ref mappings
    getProjectMigrationRefs(
      undefined,
      {
        projectId: projectId,
        afterId: fromVersionId, // assumes that toVersionId is newer than fromVersionId
      },
      { fetchPolicy: "network-only" }
    );
  }

  return {
    migrating: computed(() => migratingTo.value != null),
    migrateTo,
  };
}
