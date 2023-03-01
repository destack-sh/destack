import { graphql } from "@/gql";
import type {
  ProjectType,
  ProjectVisibility,
  UpdateProjectNameMutation,
  UpdateProjectVisibilityMutation,
} from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useProjectOps() {
  const operations = useOperationsStore();

  const { mutate: createMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createProject($input: ProjectCreateInput!) {
        createProject(input: $input) {
          ... on Project {
            ...ProjectHeader
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      refetchQueries: ["home", "profileHome"],
    }
  );

  async function create(ownerId: string, name: string, slug: string, type: ProjectType, visibility: ProjectVisibility) {
    return await operations.perform({
      type: "project.create",
      do: async () => {
        return await createMut({ input: { ownerId, name, slug, type, visibility } });
      },
    });
  }

  const { mutate: updateVisibilityMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateProjectVisibility($id: GlobalID!, $visibility: ProjectVisibility!) {
        updateProjectVisibility(input: { id: $id, visibility: $visibility }) {
          ... on Project {
            id
            visibility
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; visibility: ProjectVisibility }) =>
        ({
          __typename: "Mutation",
          updateProjectVisibility: {
            __typename: "Project",
            id: vars.id,
            visibility: vars.visibility,
          },
        } as UpdateProjectVisibilityMutation),
    }
  );

  async function updateVisibility(id: string, visibility: ProjectVisibility) {
    return await operations.perform({
      type: "project.updateVisibility",
      do: async () => {
        return await updateVisibilityMut({ id, visibility });
      },
    });
  }

  const { mutate: updateNameMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateProjectName($id: GlobalID!, $name: String!) {
        updateProjectName(input: { id: $id, name: $name }) {
          ... on Project {
            id
            name
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; name: string }) =>
        ({
          __typename: "Mutation",
          updateProjectName: {
            __typename: "Project",
            id: vars.id,
            name: vars.name,
          },
        } as UpdateProjectNameMutation),
    }
  );

  return { create, updateVisibility, updateName: updateNameMut };
}
