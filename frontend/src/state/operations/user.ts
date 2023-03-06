import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";
import { useRouter } from "vue-router";

export function useUserOps() {
  const operations = useOperationsStore();
  const router = useRouter();

  const { mutate: logoutMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation logout {
        logout {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function logout() {
    return await operations.perform({
      type: "user.logout",
      stateless: true,
      do: async () => {
        await logoutMut();
        // reload the page to clear the cache
        router.go(0);
      },
    });
  }

  const { mutate: completeSignupMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation completeSignup($input: UserCompleteSignupInput!) {
        completeSignup(input: $input) {
          ... on User {
            id
            username
            slug
            email
            name
            createdAt
            updatedAt
            completedSignup
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function completeSignup(id: string, username: string, fullName: string) {
    return await operations.perform({
      type: "user.completeSignup",
      stateless: true,
      do: async () => {
        return await completeSignupMut({ input: { id, username, fullName } });
      },
    });
  }

  const { mutate: acceptOrganizationInviteMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation acceptOrganizationInvite($id: GlobalID!) {
        acceptOrganizationInvite(id: $id) {
          ... on User {
            id
            username
            slug
            email
            name
            createdAt
            updatedAt
            completedSignup
            # refetch memberships
            organizationMemberships {
              totalCount
              edges {
                node {
                  id
                  level
                  organization {
                    id
                    name
                    slug
                  }
                }
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function acceptOrganizationInvite(id: string) {
    return await operations.perform({
      type: "user.acceptOrganizationInvite",
      stateless: true,
      do: async () => {
        return await acceptOrganizationInviteMut({ id });
      },
    });
  }

  return { logout, completeSignup, acceptOrganizationInvite };
}
