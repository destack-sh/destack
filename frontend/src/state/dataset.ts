import { graphql } from "@/gql";
import { useSymbolInterfaceState, type StatementHeader } from "@/state/editor";
import type { Ref } from "vue";

export const DatasetContentType = graphql(/* GraphQL */ `
  fragment DatasetContent on Statement {
    records {
      data
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

export function useDatasetInterfaceState(statement: Ref<StatementHeader>): Ref<DatasetInterfaceState> {
  return useSymbolInterfaceState<DatasetInterfaceState>(statement, DEFAULT_STATE);
}
