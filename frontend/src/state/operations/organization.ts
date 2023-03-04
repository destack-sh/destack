import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useOrganizationOps() {
  const operations = useOperationsStore();

  const { mutate: createMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createOrganization($name: String!, $slug: String!) {
        createOrganization(input: { name: $name, slug: $slug }) {
          ... on Organization {
            id
            name
            slug
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function create(name: string, slug: string) {
    return await operations.perform({
      type: "organization.create",
      stateless: true,
      do: async () => {
        return await createMut({ name, slug });
      },
    });
  }

  return { create };
}
