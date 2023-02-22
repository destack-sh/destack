import { graphql, useFragment } from "@/gql";
import { UserContentType } from "@/state/fragments";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import { defineStore } from "pinia";
import { computed, toRef } from "vue";

export const NON_SOCIAL_AUTH_ENABLED = process.env.ENVIRONMENT === "development";

export const useAuthStore = defineStore("auth", {
  state: () => ({
    token: null as string | null,
    loggedIn: false,
  }),
  actions: {
    setLoggedIn(token: string): void {
      this.token = token;
      this.loggedIn = true;
    },
    setLoggedOut(): void {
      this.token = null;
      this.loggedIn = false;
    },
  },
});

function _useAuth() {
  const state = useAuthStore();
  const { result: meResult } = useQuery(
    graphql(/* GraphQL */ `
      query me {
        me {
          ...UserContent
        }
      }
    `)
  );

  const me = computed(() => useFragment(UserContentType, meResult.value?.me));

  return { loggedIn: toRef(state, "loggedIn"), me };
}

export const useAuth = createSharedComposable(_useAuth);
