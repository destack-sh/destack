import type { File, Project, ProjectVersion, SymbolDefinition } from "@/gql/graphql";
import { defineStore } from "pinia";

export type ProjectHeader = Pick<Project, "id" | "name" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<File, "id" | "name" | "path" | "createdAt" | "updatedAt">;
export type SymbolDefinitionHeader = Pick<SymbolDefinition, "id" | "name" | "type" | "createdAt" | "updatedAt">;

export type RunConfiguration = {
  name: string;
  symbol: SymbolDefinitionHeader;
};

export function makeRunConfiguration(file: FileHeader, symbol: SymbolDefinitionHeader): RunConfiguration {
  return {
    name: `Run: ${file.path}.${symbol.name}`,
    symbol: symbol,
  } as RunConfiguration;
}

export type Editor = {
  type: "file" | "symbol" | "run";
  id: string;
  path: string;
  group: EditorGroup | null;
};

export type FileEditor = Editor & {
  type: "file";
  file: FileHeader;
};

export type SymbolEditor = Editor & {
  type: "symbol";
  symbol: SymbolDefinitionHeader;
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
    file: file,
    path: file.path + ".instruct",
    group: null,
  } as FileEditor;
}

export function makeSymbolEditor(symbol: SymbolDefinitionHeader): SymbolEditor {
  return {
    // append random string to enable multiple editors for the same symbol
    id: symbol.id + "-" + Math.random().toString(36),
    type: "symbol",
    path: symbol.name,
    group: null,
  } as SymbolEditor;
}

export function makeRunEditor(config: RunConfiguration): RunEditor {
  return {
    id: "run-" + config.symbol.id + Math.random().toString(36),
    type: "run",
    path: config.name,
    config: config,
    group: null,
  } as RunEditor;
}

// TODO @Architecture: should editor state be apollo local state?
export const useEditorState = defineStore("editor", {
  state: () => {
    return {
      // if feels especially wrong to have non-editor state from project here
      // (it's also linked in the Editor states)
      currentProject: null as ProjectHeader | null,
      left: makeEditorGroup("left", "Left"),
      right: makeEditorGroup("right", "Right"),
      focusedEditor: null as Editor | null,
      focusedDefinition: null as SymbolDefinitionHeader | null,
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
    focusedFile(state) {
      return state.focusedEditor?.type == "file" ? (state.focusedEditor as FileEditor).file : null;
    },
    focusedGroup(state) {
      return state.focusedEditor?.group;
    },
  },
  actions: {
    setProject(project: ProjectHeader): void {
      this.currentProject = project;
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
      let editor = this.editors.find((e) => e.type == "file" && (e as FileEditor).file?.id == file.id);
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

    focusDefinition(file: FileHeader, definition: SymbolDefinitionHeader, group?: EditorGroup) {
      // TODO @Feature: auto-focus the file that contains the definition
      this.focusFile(file, group);
      this.focusedDefinition = definition;
    },
  },
});
