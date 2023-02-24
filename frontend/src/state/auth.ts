import { graphql, useFragment } from "@/gql";
import { UserContentType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { HTTP_API_BASE_URL, IS_LOCALHOST } from "@/utils/globals";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import { defineStore } from "pinia";
import { computed, watchEffect } from "vue";
import { useRouter } from "vue-router";

export const NON_SOCIAL_AUTH_ENABLED = process.env.ENVIRONMENT === "development";

export const useAuthStore = defineStore("auth", {
  state: () => ({}),
  actions: {},
});

function _useAuth() {
  const state = useAuthStore();
  const { result: meResult, loading: meLoading } = useQuery(
    graphql(/* GraphQL */ `
      query me {
        me {
          ...UserContent
        }
      }
    `)
  );

  const me = computed(() => useFragment(UserContentType, meResult.value?.me));

  return { loggedIn: computed(() => !!me.value), me, loading: meLoading };
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
    url: `${HTTP_API_BASE_URL}/login/google-oauth2/`,
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

export function useRedirectIfNotLoggedIn(redirectTo = { name: "Signup" }) {
  const auth = useAuth();
  const router = useRouter();
  const notifications = useNotifications();
  watchEffect(() => {
    if (!auth.loading.value && !auth.loggedIn.value) {
      console.log("user not logged in, redirecting");
      router.replace(redirectTo);
      notifications.show({
        kind: "notice",
        type: "auth.redirect",
        message: "Log in first",
        description: "You need to be logged in to do this.",
      });
    }
  });
}
