import { graphql } from "@/gql";
import { UserStatus } from "@/gql/graphql";
import { useNotifications } from "@/state/notifications";
import { HTTP_API_BASE_URL, IS_LOCALHOST } from "@/utils/globals";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import posthog from "posthog-js";
import { computed, watchEffect } from "vue";
import { useRouter } from "vue-router";

export const NON_SOCIAL_AUTH_ENABLED = process.env.ENVIRONMENT === "development";

function _useAuth() {
  const { result: meResult, loading: meLoading } = useQuery(
    graphql(/* GraphQL */ `
      query me {
        me {
          id
          username
          slug
          email
          name
          createdAt
          updatedAt
          status
          organizationMemberships {
            totalCount
            edges {
              node {
                id
                createdAt
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
      }
    `)
  );

  const me = computed(() => meResult.value?.me);
  const memberships = computed(() => meResult.value?.me?.organizationMemberships?.edges.map((e) => e.node));
  const organizations = computed(() => me.value?.organizationMemberships?.edges.map((e) => e.node.organization));

  // identify user for posthog
  watchEffect(() => {
    if (me.value != null) {
      posthog.identify(me.value.id, { email: me.value.email, name: me.value.name });
      // alias plain uuid to global id
      // global ids are User:id base64 encoded
      const uuid = atob(me.value.id).split(":")[1];
      posthog.alias(me.value.id, uuid);
    } else {
      posthog.reset();
    }
  });

  return { loggedIn: computed(() => !!me.value), me, memberships, organizations, loading: meLoading };
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

export function useRedirectIfWaitlisted() {
  const auth = useAuth();
  const router = useRouter();
  const notifications = useNotifications();
  watchEffect(() => {
    if (!auth.loading.value && auth.loggedIn.value && auth.me.value?.status == UserStatus.Waitlisted) {
      console.log("user waitlisted, redirecting");
      router.replace({ name: "Waitlisted" });
      notifications.show({
        kind: "notice",
        type: "auth.redirect",
        message: "Waitlisted",
        description: "You're waiting eagerly for access. Soon!",
      });
    }
  });
}
