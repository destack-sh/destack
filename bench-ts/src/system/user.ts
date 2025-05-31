import { autoloader } from "@/globals";
import { makeNode } from "@/language/core/node";
import { commitTransactionBuffers } from "@/language/core/transaction";
import { supervisor, type OperationOptions } from "@/proto/services";
import {
  ClientData,
  NodeType,
  ObjectType,
  Region,
  UserData,
  UserProperty,
  UserWizardViewStage,
  ViewType,
} from "@/proto/wire";
import {
  EMPTY_SCOPE,
  nodeReference,
  propertyReference,
  toNodeRef,
  wrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import local from "@/system/client";
import { clearConnections, useGetConnection } from "@/system/connection";
import { bench, canvas, goToBench } from "@/system/space";
import { provideCommands } from "@/ui/command";
import { makeIcon } from "@/ui/icon";
import type { ViewIn } from "@/ui/space";
import { toaster } from "@/ui/toast";
import { TELEMETRY } from "@/utils/globals";
import { log } from "@/utils/log";
import { getRegionSlug } from "@/utils/region";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { whenever } from "@vueuse/core";
import posthog from "posthog-js";
import { computed } from "vue";

export const isAuthenticated = computed(() => local.clientInfo.value?.accessToken != null);
export const isUnauthenticated = computed(() => !isAuthenticated.value);

export const { graph: userGraph, connection: userConnection } = useGetConnection(
  { name: "current.user", live: true, paramsPretty: computed(() => ({ slug: local.userInfo.value?.slug })) },
  computed(() => ({
    scope: EMPTY_SCOPE,
    roots: [nodeReference(NodeType.USER, local.userInfo.value?.id!)],
    options: {
      descendantTypes: [NodeType.CLIENT],
      includePropertiesPtr: [propertyReference(ObjectType.USER, UserProperty.email)],
    },
    isEnabled: isAuthenticated.value,
  })),
);
export const user = userGraph.getRef(
  computed(() => (isAuthenticated.value ? { nodeType: NodeType.USER, id: local.userInfo.value?.id! } : null)),
);
export const client = userGraph.getRef(
  computed(() =>
    local.clientInfo.value?.id == null ? null : { nodeType: NodeType.CLIENT, id: local.clientInfo.value.id },
  ),
);
export const clients = userGraph.getChildrenRef(
  computed(() => (user.value != null ? toNodeRef(user.value) : null)),
  NodeType.CLIENT,
);

// identify user for telemetry
whenever(
  user,
  () => {
    if (user.value == null) return;
    if (TELEMETRY) {
      posthog.identify(user.value.id, {
        name: user.value.name,
        email: user.value.email,
        region: getRegionSlug(user.value.region),
        slug: user.value.slug,
        is_staff: user.value.isStaff,
      });
    }
  },
  { immediate: true },
);

function makeCurrentClient(): ClientData {
  return makeNode(
    {
      metatype: NodeType.CLIENT,
      benchPtr: undefined,
      ...local.clientMeta.value,
      name: local.clientMeta.value.deviceType,
    },
    { omit: ["id"] },
  );
}

function onLogIn(info: { user: UserData; client: ClientData; accessToken: string }) {
  local.setUser({
    user: {
      id: info.user.id,
      email: info.user.email!,
      name: info.user.name!,
      slug: info.user.slug!,
    },
    client: {
      id: info.client.id,
      accessToken: info.accessToken,
    },
  });
  log.info("user.login", local.userInfo.value);
}

function onLogout() {
  local.clearUser();
  local.clearBench();
  local.clearSpace();
  clearConnections();
  autoloader.reset();
}

/**
 * Sign up a new user and simultaneously log in as the current client.
 */
export async function signUp(
  userIn: { name?: string; slug: string; email: string; region: Region },
  password: string,
  options?: OperationOptions,
) {
  if (isAuthenticated.value) throw new Error("already logged in");
  const {
    response: { user, client, accessToken },
  } = await supervisor.signupUser(
    {
      name: userIn.name,
      slug: userIn.slug,
      email: userIn.email,
      region: userIn.region,
      password,
      client: makeCurrentClient(),
      activate: true,
    },
    options,
  );
  if (user == null || client == null) throw new Error("unexpected null user or client");
  onLogIn({ user, client, accessToken });
  toaster.success({ icon: "fas fa-right-from-bracket", title: "Signed up", text: `Welcome, ${user.slug}.` });
  posthog.capture("user_signup");
  return { user, client };
}

/**
 * Log in a user as the current client.
 */
export async function logIn(
  userIn: { slug: string } | { email: string },
  password: string,
  options?: OperationOptions,
): Promise<{ user: UserData; client: ClientData }> {
  if (local.clientInfo.value != null) throw new Error("already logged in");
  const {
    response: { user, client, accessToken },
  } = await supervisor.loginUser({ user: wrapProtoOneOf(userIn), password, client: makeCurrentClient() }, options);
  if (user == null || client == null) throw new Error("unexpected null user or client");
  onLogIn({ user, client, accessToken });
  toaster.success({ icon: "fas fa-right-from-bracket", title: "Logged in", text: `Welcome back, ${user.slug}.` });
  posthog.capture("user_login");
  return { user, client };
}

/**
 * Logs out clients (may include current).
 */
export async function logOut(logOut?: { all?: boolean; clients?: { id: string }[] }, options?: OperationOptions) {
  if (local.clientInfo.value == null) throw new Error("not logged in");
  await commitTransactionBuffers();
  await supervisor.logoutUser(
    {
      clients: logOut?.clients?.map((c) => nodeReference(NodeType.CLIENT, c.id!)) ?? [],
      logoutAll: logOut?.all,
    },
    options,
  );
  if (logOut == null || logOut?.all || logOut?.clients?.some((c) => c.id == local.clientInfo.value?.id)) {
    // logged out current client
    log.info("user.logout", logOut);
    onLogout();
    posthog.capture("user_logout");
    toaster.success({ icon: "fas fa-right-to-bracket", title: "Logged out", text: "Thanks for all the fish." });
  }
}

/** Reports an authentication error from a request using our current credentials */
export function onAuthenticationError(error: RpcError) {
  // NOTE :Cleanup: should handle user auth error & badge auth error separately
  log.error("user.unauthenticated");
  local.clearUser();
  local.clearBench();
}

function userWizardView(view: { title: string; stage: UserWizardViewStage }): ViewIn {
  return {
    type: ViewType.WIZARD,
    icon: makeIcon("fas fa-right-from-bracket"),
    title: view.title,
  };
}

provideCommands<"user">({
  "user.auth.signup": {
    icon: "fas fa-right-from-bracket",
    title: "Sign Up",
    text: "Create a new account.",
    isEnabled: isUnauthenticated,
    command: () => {
      canvas.addView(userWizardView({ title: "Sign up", stage: UserWizardViewStage.SIGN_UP }), {
        ifPresent: "upsertAndFocus",
      });
    },
  },
  "user.auth.login": {
    icon: "fas fa-right-from-bracket",
    title: "Log In",
    text: "Log in to an existing account.",
    isEnabled: isUnauthenticated,
    command: () => {
      canvas.addView(userWizardView({ title: "Log in", stage: UserWizardViewStage.LOG_IN }), {
        ifPresent: "upsertAndFocus",
      });
    },
  },
  "user.auth.logout": {
    icon: "fas fa-right-to-bracket",
    title: "Log Out",
    text: "Log out of the current client.",
    isEnabled: isAuthenticated,
    command: () => logOut(),
  },
  "user.navigate.goToHome": {
    icon: "fas fa-home",
    title: "Go Home",
    text: "Go back to your Bench.",
    isEnabled: () => {
      return isAuthenticated.value;
    },
    command: async () => {
      if (bench.value?.id == user.value!.benchPtr?.id) {
        toaster.success({
          icon: "fas fa-home",
          title: "Already home",
          text: "You are already on your Bench.",
          override: "user.navigate.goToHome",
        });
      } else {
        await goToBench({ bench: user.value!.benchPtr! as TypedNodeReferenceData<NodeType.BENCH> });
      }
    },
  },
});
