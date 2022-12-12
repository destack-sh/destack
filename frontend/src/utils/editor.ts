import type { File, Project, ProjectVersion, Symbol } from "@/gql/graphql";
import { defineStore } from "pinia";

export type ProjectHeader = Pick<Project, "id" | "name" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<File, "id" | "name" | "path" | "createdAt" | "updatedAt">;
export type SymbolHeader = Pick<Symbol, "id" | "name" | "type" | "createdAt" | "updatedAt">;

export type RunConfiguration = {
  name: string;
  symbolId: string;
};

export type Editor = {
  type: "file" | "symbol" | "run";
  id: string;
  path: string;
  group: EditorGroup | null;
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
  type: "run";
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
    id: file.id + "-" + Math.random().toString(36),
    type: "file",
    fileId: file.id,
    path: file.path + ".instruct",
    group: null,
  } as FileEditor;
}

export function makeRunConfiguration(symbol: SymbolHeader): RunConfiguration {
  return {
    name: `Run: ${symbol.name}`,
    symbolId: symbol.id,
  } as RunConfiguration;
}

export function makeRunEditor(config: RunConfiguration): RunEditor {
  return {
    id: "run-" + config.symbolId + Math.random().toString(36),
    type: "run",
    path: config.name,
    config: config,
    group: null,
  } as RunEditor;
}

export const useEditorState = defineStore("editor", {
  state: () => {
    return {
      currentProjectId: null as string | null,
      left: makeEditorGroup("left", "Left"),
      right: makeEditorGroup("right", "Right"),
      focusedEditor: null as Editor | null,
      focusedElementId: null as string | null,
      readonly: false,
    };
  },
  getters: {
    editorGroups(state) {
      return [state.left, state.right];
    },
    editors(state) {
      return state.left.editors.concat(state.right.editors);
    },
    focusedFileId(state): string | null {
      return state.focusedEditor?.type == "file" ? (state.focusedEditor as FileEditor).fileId : null;
    },
    focusedGroup(state) {
      return state.focusedEditor?.group;
    },
  },
  actions: {
    setProject(project: ProjectHeader): void {
      this.currentProjectId = project.id;
    },

    openEditor(editor: Editor, group?: EditorGroup): void {
      console.log(`open editor ${editor.path} in group ${group?.name}`);
      group = group || this.left;
      // change editor group if different
      if (editor.group != group) {
        if (editor.group != null) {
          // remove from old group
          editor.group.editors = editor.group.editors.filter((e) => e != editor);
          if (editor.group.activeEditor == editor) {
            editor.group.activeEditor = editor.group.editors[0] || null;
          }
        }
        editor.group = group;
        group.editors.push(editor);
      }
    },

    closeEditor(editor: Editor): void {
      console.log(`close editor ${editor.path}`);
      if (editor.group != null) {
        // remove from old group
        editor.group.editors = editor.group.editors.filter((e) => e != editor);
        if (editor.group.activeEditor == editor) {
          editor.group.activeEditor = editor.group.editors[0] || null;
        }
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
      console.log(`open file ${file.id} ${file.path}`);
      if (!editor) {
        console.log(`create new file editor for ${file.id} ${file.path}`);
        editor = makeFileEditor(file);
      }
      // if group wasn't passed, just return the editor if it's already open
      if (!group && editor.group) {
        return editor;
      } else {
        // otherwise open in the group or the fallback group
        group = group || this.focusedGroup || this.left; // use active group if available
        if (editor.group != group) {
          this.openEditor(editor, group);
        }
        return editor;
      }
    },

    focusEditor(editor: Editor): void {
      console.log(`focus editor ${editor.path} in group ${editor.group?.id}`);
      if (!editor.group) {
        throw new Error("editor must be in a group: " + editor.path);
      }
      this.focusedEditor = editor;
      editor.group.activeEditor = editor;
    },

    focusFile(file: FileHeader, group?: EditorGroup): Editor {
      const editor = this.openFile(file, group);
      this.focusEditor(editor);
      return editor;
    },

    focusDefinition(file: FileHeader, symbol: SymbolHeader, group?: EditorGroup) {
      this.focusFile(file, group);
      this.focusElement(symbol);
    },

    focusElement(element: SymbolHeader | FileHeader) {
      this.focusedElementId = element.id;
    },
  },
});
