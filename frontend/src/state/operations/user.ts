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

  return { logout };
}
