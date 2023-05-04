import { graphql } from "@/gql";
import type { OrganizationMembershipLevel } from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useOrganizationOps() {
  const ops = useOperationsStore();

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
    return await ops.perform({
      type: "organization.create",
      stateless: true,
      do: async () => {
        return await createMut({ name, slug });
      },
    });
  }

  const { mutate: createInvitesMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createInvites(
        $id: GlobalID!
        $emails: [String!]!
        $level: OrganizationMembershipLevel!
        $message: String
      ) {
        createOrganizationInvites(input: { id: $id, emails: $emails, level: $level, message: $message }) {
          ... on Organization {
            id
            invites {
              totalCount
              edges {
                node {
                  id
                }
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function createInvites(id: string, emails: string[], level: OrganizationMembershipLevel, message?: string) {
    return await ops.perform({
      type: "organization.createInvites",
      stateless: true,
      do: async () => {
        return await createInvitesMut({ id, emails, level, message });
      },
    });
  }

  const { mutate: cancelInviteMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation cancelInvite($id: GlobalID!) {
        cancelOrganizationInvite(id: $id) {
          ... on Organization {
            id
            invites {
              totalCount
              edges {
                node {
                  id
                }
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function cancelInvite(id: string) {
    return await ops.perform({
      type: "organization.cancelInvite",
      stateless: true,
      do: async () => {
        return await cancelInviteMut({ id });
      },
    });
  }

  return { create, createInvites, cancelInvite };
}
