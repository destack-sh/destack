import { graphql } from "@/gql";
import { useSymbolInterfaceState, type StatementHeader } from "@/utils/editor";
import type { Ref } from "vue";

export const SchemaContentType = graphql(/* GraphQL */ `
  fragment SchemaContent on Schema {
    id
    description
    element {
      ...SchemaElementContentDeep
    }
  }
`);

export type ViewMode = "pretty" | "json";
export const viewModes: ViewMode[] = ["pretty", "json"];
export type SchemaInterfaceState = {
  view: ViewMode;
};
export const DEFAULT_STATE = {
  view: "pretty",
} as SchemaInterfaceState;

export function useSchemaInterfaceState(statement: Ref<StatementHeader>): Ref<SchemaInterfaceState> {
  return useSymbolInterfaceState<SchemaInterfaceState>(statement, DEFAULT_STATE);
}
