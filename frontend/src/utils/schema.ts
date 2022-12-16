import { graphql } from "@/gql";
import { useSymbolInterfaceState, type SymbolHeader } from "@/utils/editor";
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

export function useSchemaInterfaceState(symbol: Ref<SymbolHeader>): Ref<SchemaInterfaceState> {
  return useSymbolInterfaceState<SchemaInterfaceState>(symbol, DEFAULT_STATE);
}
