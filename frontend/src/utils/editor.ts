import type { File, SymbolDefinition } from "@/gql/graphql";
import type { Ref } from "vue";

// TODO @Architecture: should editor state just be apollo local state?
export const EDITOR_STATE_KEY = Symbol("selection");

export type FileHeader = Pick<File, "id" | "name" | "path" | "createdAt" | "updatedAt">;
export type SymbolDefinitionHeader = Pick<SymbolDefinition, "id" | "name" | "type" | "createdAt" | "updatedAt">;

export type EditorState = {
  focusedFile: Ref<FileHeader | null>;
  focusedDefinition: Ref<SymbolDefinitionHeader | null>;

  focusFile: (file: FileHeader) => void;
  focusDefinition: (definition: SymbolDefinitionHeader) => void;
};
