import { graphql } from "@/gql";
import type { AddCompilationInput } from "@/gql/graphql";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useCompilationOps() {
  const operations = useOperationsStore();

  const { mutate: addCompilationMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation addCompilation($input: AddCompilationInput!) {
        addCompilationTarget(input: $input) {
          compilation {
            id
            name
            createdAt
            updatedAt
          }
        }
      }
    `),
    { refetchQueries: ["projectVersionContent"] }
  );

  async function add(input: AddCompilationInput) {
    await operations.perform({
      type: "add-compilation",
      do: async () => {
        await addCompilationMut({ input });
      },
    });
  }

  const { mutate: compileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation compile($compilationId: GlobalID!) {
        compile(input: { compilationId: $compilationId }) {
          compilation {
            id
            name
            createdAt
            updatedAt
            targetTask {
              ...TaskContent
            }
            targetCode {
              ...CodeContent
            }
          }
        }
      }
    `),
    // TODO @Performance: don't refetch all file contents post compilation
    //  just update the cache with new source mappings (and remove old ones)
    //  This applies to all mutations, not just this one.
    { refetchQueries: ["projectVersionContent", "fileContentById"] }
  );

  async function compile(id: string) {
    await operations.perform({
      type: "compile",
      do: async () => {
        await compileMut({ compilationId: id });
      },
    });
  }

  return { add, compile };
}
