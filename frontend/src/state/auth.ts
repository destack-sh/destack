import { graphql, useFragment } from "@/gql";
import { UserContentType } from "@/state/fragments";
import { HTTP_API_BASE_URL, IS_LOCALHOST } from "@/utils/globals";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import { defineStore } from "pinia";
import { computed } from "vue";

export const NON_SOCIAL_AUTH_ENABLED = process.env.ENVIRONMENT === "development";

export const useAuthStore = defineStore("auth", {
  state: () => ({}),
  actions: {},
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

  return { loggedIn: computed(() => !!me.value), me };
}

export const useAuth = createSharedComposable(_useAuth);

// :SocialAuthProviders
export const SOCIAL_AUTH_PROVIDERS = [
  {
    name: "GitHub",
    url: `${HTTP_API_BASE_URL}/login/github/`,
    enabled: true,
  },
  {
    name: "GitLab",
    url: `${HTTP_API_BASE_URL}/login/gitlab/`,
    enabled: !IS_LOCALHOST,
  },
  {
    name: "Google",
    url: `${HTTP_API_BASE_URL}/login/google/`,
    enabled: !IS_LOCALHOST,
  },
];

export function encodeProviderUrl(url?: string, next?: string): string | undefined {
  if (!url) {
    return undefined;
  }
  if (!next) {
    return url;
  }
  return `${url}?next=${encodeURIComponent(next)}`;
}
