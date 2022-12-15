import { graphql } from "@/gql";
import { useSymbolInterfaceState, type SymbolHeader } from "@/utils/editor";
import type { Ref } from "vue";

export const DatasetContentType = graphql(/* GraphQL */ `
  fragment DatasetContent on Dataset {
    id
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
} as DatasetInterfaceState;

export function useDatasetInterfaceState(symbol: Ref<SymbolHeader>): Ref<DatasetInterfaceState> {
  return useSymbolInterfaceState<DatasetInterfaceState>(symbol, DEFAULT_STATE);
}
