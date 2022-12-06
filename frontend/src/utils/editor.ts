import type { File, SymbolDefinition } from "@/gql/graphql";
import type { ComputedRef, Ref } from "vue";

// TODO @Architecture: should editor state just be apollo local state?
export const EDITOR_STATE_KEY = Symbol("selection");

export type FileHeader = Pick<File, "id" | "name" | "path" | "createdAt" | "updatedAt">;
export type SymbolDefinitionHeader = Pick<SymbolDefinition, "id" | "name" | "type" | "createdAt" | "updatedAt">;

export type Editor = {
  type: "file" | "symbol" | "run";
  path: Ref<string>;
  group: Ref<EditorGroup | null>;
};

export type FileEditor = Editor & {
  type: "file";
  file: Ref<FileHeader>;
};

export type EditorGroup = {
  id: string;
  name: Ref<string>;
  editors: Ref<Editor[]>;
};

export type EditorState = {
  left: EditorGroup;
  right: EditorGroup;

  editorGroups: ComputedRef<EditorGroup[]>;
  editors: ComputedRef<Editor[]>;

  focusedEditor: Ref<Editor | null>;
  focusedFile: Ref<FileHeader | null>;
  focusedDefinition: Ref<SymbolDefinitionHeader | null>;

  openEditor: (editor: Editor, group?: EditorGroup) => void;
  openFile: (file: FileHeader, group?: EditorGroup) => Editor;
  focusEditor: (editor: Editor) => void;
  focusFile: (file: FileHeader, group?: EditorGroup) => Editor;
  focusDefinition: (definition: SymbolDefinitionHeader) => void;
};
