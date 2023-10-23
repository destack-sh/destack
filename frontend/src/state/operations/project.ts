import { graphql } from "@/gql";
import type {
  ProjectVisibility,
  UpdateProjectNameMutation,
  UpdateProjectVisibilityMutation,
  UpdateProjectSharingMutation,
} from "@/gql/graphql";
import type { ModuleAccessLevel } from "@/state/auth";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useProjectOps() {
  const ops = useOperationsStore();

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

  async function create(ownerId: string, name: string, slug: string, visibility: ProjectVisibility) {
    return await ops.perform({
      type: "project.create",
      do: async () => {
        return await createMut({ input: { ownerId, name, slug, visibility } });
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
    return await ops.perform({
      type: "project.updateVisibility",
      do: async () => {
        return await updateVisibilityMut({ id, visibility });
      },
    });
  }

  const { mutate: updateSharingMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateProjectSharing(
        $id: GlobalID!
        $baseLevel: Int!
        $sharingEnabled: Boolean!
        $sharingToken: UUID!
        $sharingLevel: Int!
      ) {
        updateProjectSharing(
          input: {
            id: $id
            baseLevel: $baseLevel
            sharingEnabled: $sharingEnabled
            sharingToken: $sharingToken
            sharingLevel: $sharingLevel
          }
        ) {
          ... on Project {
            id
            baseLevel
            sharingEnabled
            sharingToken
            sharingLevel
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        baseLevel: ModuleAccessLevel;
        sharingEnabled: boolean;
        sharingToken: string;
        sharingLevel: ModuleAccessLevel;
      }) =>
        ({
          __typename: "Mutation",
          updateProjectSharing: {
            __typename: "Project",
            id: vars.id,
            baseLevel: vars.baseLevel,
            sharingEnabled: vars.sharingEnabled,
            sharingToken: vars.sharingToken,
            sharingLevel: vars.sharingLevel,
          },
        } as UpdateProjectSharingMutation),
    }
  );

  async function updateSharing(
    id: string,
    baseLevel: number,
    sharingEnabled: boolean,
    sharingToken: string,
    sharingLevel: number
  ) {
    return await ops.perform({
      type: "project.updateSharing",
      do: async () => {
        return await updateSharingMut({ id, baseLevel, sharingEnabled, sharingToken, sharingLevel });
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

  return { create, updateVisibility, updateSharing: updateSharing, updateName: updateNameMut };
}
