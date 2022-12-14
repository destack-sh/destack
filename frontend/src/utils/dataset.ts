import { graphql } from "@/gql";
import { EDITOR_INTERFACE_STATE, type EditorInterfaceState, type SymbolHeader } from "@/utils/editor";
import { computed, inject, type Ref } from "vue";

export const DatasetContentType = graphql(/* GraphQL */ `
  fragment DatasetContent on Dataset {
    id
    schema {
      ...SchemaElementContentDeep
    }
    length
    records {
      data
      index
    }
  }
`);

export type ViewMode = "table" | "json" | "jsonl";
export const viewModes: ViewMode[] = ["table", "jsonl", "json"];
export type DatasetInterfaceState = {
  view: ViewMode;
  showTableHeader: boolean;
};
export const DEFAULT_STATE = {
  view: "table",
  showTableHeader: false,
};

export function useDatasetInterfaceState(symbol: Ref<SymbolHeader>): Ref<DatasetInterfaceState> {
  const editorInterfaceState = inject<EditorInterfaceState>(EDITOR_INTERFACE_STATE);
  // local state is stored by symbol id in the opaque editor interface state
  const state = computed({
    get() {
      return editorInterfaceState?.get(symbol.value.id, DEFAULT_STATE) as DatasetInterfaceState;
    },
    set(value: DatasetInterfaceState) {
      editorInterfaceState?.set(symbol.value.id, value);
    },
  });

  return state;
}
