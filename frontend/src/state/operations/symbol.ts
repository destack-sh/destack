import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolContentOps() {
  const operations = useOperationsStore();

  // schema mutations

  const { mutate: updateStatementTypeNodeMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateStatementTypeNode($id: GlobalID!, $btl: String!) {
        updateStatementTypeNode(input: { id: $id, btl: $btl }) {
          ... on Statement {
            id
            btl
            revision
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function updateStatementTypeNode(id: string, oldBtl: string, newBtl: string) {
    await operations.perform({
      type: "symbol.schema.updateContent",
      do: async () => {
        await updateStatementTypeNodeMut({ id: id, btl: newBtl });
      },
      undo: async () => {
        await updateStatementTypeNodeMut({ id: id, btl: oldBtl });
      },
    });
  }

  const { mutate: updateStatementDescriptionMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateStatementDescription($id: GlobalID!, $description: String!) {
        updateStatementDescription(input: { id: $id, description: $description }) {
          ... on Statement {
            id
            description
            revision
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function updateStatementDescription(id: string, oldDescription: string, newDescription: string) {
    await operations.perform({
      type: "symbol.expectation.updateContent",
      do: async () => {
        await updateStatementDescriptionMut({ id: id, description: newDescription });
      },
      undo: async () => {
        await updateStatementDescriptionMut({ id: id, description: oldDescription });
      },
    });
  }

  // code mutations

  const { mutate: updateStatementCodeMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateStatementCode($id: GlobalID!, $code: String, $codeBuiltinId: String) {
        updateStatementCode(input: { id: $id, code: $code, codeBuiltinId: $codeBuiltinId }) {
          ... on Statement {
            id
            code
            codeBuiltinId
            revision
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function updateStatementCode(
    id: string,
    oldContent: { code?: string; codeBuiltinId?: string },
    newContent: { code?: string; codeBuiltinId?: string }
  ) {
    await operations.perform({
      type: "symbol.code.updateContent",
      do: async () => {
        await updateStatementCodeMut({
          id: id,
          code: newContent.code,
          codeBuiltinId: newContent.codeBuiltinId,
        });
      },
      undo: async () => {
        await updateStatementCodeMut({
          id: id,
          code: oldContent.code,
          codeBuiltinId: oldContent.codeBuiltinId,
        });
      },
    });
  }

  return { updateStatementTypeNode, updateStatementDescription, updateStatementCode };
}
