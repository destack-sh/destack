import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolContentOps() {
  const operations = useOperationsStore();

  // schema mutations

  const { mutate: updateTypeContentMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation update_type_content($id: GlobalID!, $btl: String!) {
        updateTypeContent(input: { statementId: $id, btl: $btl }) {
          id
          content {
            ...TypeContent
          }
        }
      }
    `)
  );

  async function updateTypeContent(id: string, oldBtl: string, newBtl: string) {
    await operations.perform({
      type: "symbol.schema.updateContent",
      do: async () => {
        await updateTypeContentMut({ id: id, btl: newBtl });
      },
      undo: async () => {
        await updateTypeContentMut({ id: id, btl: oldBtl });
      },
    });
  }

  // task mutations

  const { mutate: updateTaskContentMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateTaskContent($id: GlobalID!, $description: String!) {
        updateTaskContent(input: { statementId: $id, description: $description }) {
          id
          content {
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
        updateExpectationContent(input: { statementId: $id, description: $description }) {
          id
          content {
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
        updateCodeContent(input: { statementId: $id, code: $code, builtinId: $builtinId }) {
          id
          content {
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

  return { updateTypeContent, updateTaskContent, updateExpectationContent, updateCodeContent };
}
