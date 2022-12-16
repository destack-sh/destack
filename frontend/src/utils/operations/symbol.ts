import { graphql } from "@/gql";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolOps() {
  const operations = useOperationsStore();

  const { mutate: renameSymbolMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation renameSymbol($id: GlobalID!, $name: String!) {
        renameSymbol(input: { id: $id, name: $name }) {
          ... on Symbol {
            id
            name
            typeNameDeclaration
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function rename(id: string, oldName: string, newName: string) {
    await operations.perform({
      type: "symbol.rename",
      do: async () => {
        await renameSymbolMut({ id: id, name: newName });
      },
      undo: async () => {
        await renameSymbolMut({ id: id, name: oldName });
      },
    });
  }

  return { rename };
}

export function useSymbolContentOps() {
  const operations = useOperationsStore();

  // task mutations

  const { mutate: updateTaskContentMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateTaskContent($id: GlobalID!, $description: String!) {
        updateTaskContent(input: { symbolId: $id, description: $description }) {
          id
          content {
            id
            ... on Task {
              description
            }
          }
        }
      }
    `)
  );

  async function updateTaskContent(id: string, oldDescription: string, newDescription: string) {
    await operations.perform({
      type: "symbol.task.updateContent",
      do: async () => {
        await updateTaskContentMut({ id: id, description: newDescription });
      },
      undo: async () => {
        await updateTaskContentMut({ id: id, description: oldDescription });
      },
    });
  }

  // expectation mutations

  const { mutate: updateExpectationContentMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateExpectationContent($id: GlobalID!, $description: String!) {
        updateExpectationContent(input: { symbolId: $id, description: $description }) {
          id
          content {
            id
            ... on Expectation {
              description
            }
          }
        }
      }
    `)
  );

  async function updateExpectationContent(id: string, oldDescription: string, newDescription: string) {
    await operations.perform({
      type: "symbol.expectation.updateContent",
      do: async () => {
        await updateExpectationContentMut({ id: id, description: newDescription });
      },
      undo: async () => {
        await updateExpectationContentMut({ id: id, description: oldDescription });
      },
    });
  }

  // code mutations

  const { mutate: updateCodeContentMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateCodeContent($id: GlobalID!, $code: String, $builtinId: String) {
        updateCodeContent(input: { symbolId: $id, code: $code, builtinId: $builtinId }) {
          id
          content {
            id
            ... on Code {
              builtinId
              code
            }
          }
        }
      }
    `)
  );

  async function updateCodeContent(
    id: string,
    oldContent: { code?: string; builtinId?: string },
    newContent: { code?: string; builtinId?: string }
  ) {
    await operations.perform({
      type: "symbol.code.updateContent",
      do: async () => {
        await updateCodeContentMut({
          id: id,
          code: newContent.code,
          builtinId: newContent.builtinId,
        });
      },
      undo: async () => {
        await updateCodeContentMut({
          id: id,
          code: oldContent.code,
          builtinId: oldContent.builtinId,
        });
      },
    });
  }

  return { updateTaskContent, updateExpectationContent, updateCodeContent };
}
