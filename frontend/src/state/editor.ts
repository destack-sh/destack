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
import { reverseRecord } from "@/utils/functools";
import { defineStore } from "pinia";
import { computed, inject, onBeforeUnmount, type Ref } from "vue";

export type ProjectHeader = Pick<Project, "id" | "name" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<File, "__typename" | "id" | "name" | "path" | "createdAt" | "updatedAt">;
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
  [SymbolType.Dataset]: "data",
  [SymbolType.Model]: "model",
  [SymbolType.Expectation]: "expect",
  [SymbolType.Task]: "task",
  [SymbolType.Value]: "value",
  [SymbolType.Capability]: "capability",
  [SymbolType.Requirement]: "require",
  [SymbolType.Runconfig]: "run",
  [SymbolType.Compilation]: "build",
};
export const SYMBOL_TYPE_BY_KEYWORD: Record<string, SymbolType> = reverseRecord(SYMBOL_TYPE_KEYWORD);
export const MODIFIER_KEYWORD: Record<StatementModifier, string> = {
  [StatementModifier.Like]: "like",
  [StatementModifier.Unlike]: "unlike",
  [StatementModifier.Check]: "check",
  [StatementModifier.With]: "with",
  [StatementModifier.Var]: "var",
  [StatementModifier.Extend]: "extend",
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

export type RunConfiguration = {
  name: string;
  symbolId: string;
};

export type ViewId = "explorer" | "history";

export type Editor = {
  type: "file" | "execute";
  id: string;
  path: string;
  scroll?: { x: number; y: number };
  localState: Record<string, unknown>; // opaque (JSONable) local state for each editor
  groupId: string | null; // id instead of EditorGroup to avoid circular dependency
};

export const EDITOR_INTERFACE_STATE = Symbol();
export type EditorInterfaceState = {
  get(key: string, default_?: unknown): unknown;
  set(key: string, value: unknown): void;
};

export type FileEditor = Editor & {
  type: "file";
  fileId: string;
};

export type SymbolEditor = Editor & {
  type: "symbol";
  symbolId: string;
};

export type RunEditor = Editor & {
  type: "execute";
  config: RunConfiguration;
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

export function makeRunConfiguration(statement: StatementHeader): RunConfiguration {
  return {
    name: `Run: ${statement.name}`,
    symbolId: statement.id,
  } as RunConfiguration;
}

export function makeRunEditor(config: RunConfiguration): RunEditor {
  return {
    id: "execute-" + config.symbolId + Math.random().toString(16),
    type: "execute",
    path: config.name,
    config: config,
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

    async migrateTo(version: ProjectVersionHeader, intermediateRefs?: RefMapping[][]): Promise<void> {
      const projectId = this.currentProjectId;

      if (intermediateRefs != null) {
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
      } else {
        // just reset if we don't have any iyntermediate refs
        this.$reset();
      }

      this.currentProjectId = projectId;
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

    openEditor(editor: Editor, group?: EditorGroup): void {
      console.log(`open editor ${editor.path} in group ${group?.id}`);
      group = group || this.left;
      // change editor group if different
      if (editor.groupId != group.id) {
        if (editor.groupId != null) {
          // remove from old group
          this._removeEditorFromGroup(editor);
        }
        editor.groupId = group.id;
        group.editors.push(editor);
      }
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
      // if group wasn't passed, just return the editor if it's already open
      if (!group && editor.groupId) {
        return editor;
      } else {
        // otherwise open in the group or the fallback group
        group = group || this.focusedGroup || this.left; // use active group if available
        if (editor.groupId != group.id) {
          this.openEditor(editor, group);
        }
        return editor;
      }
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

export function useSymbolInterfaceState<T>(statement: Ref<StatementHeader>, defaultState: T): Ref<T> {
  const editorInterfaceState = inject<EditorInterfaceState>(EDITOR_INTERFACE_STATE);
  // local state is stored by statement id in the opaque editor interface state
  const state = computed({
    get() {
      return editorInterfaceState?.get(statement.value.id, defaultState) as T;
    },
    set(value: T) {
      editorInterfaceState?.set(statement.value.id, value);
    },
  });

  return state;
}
