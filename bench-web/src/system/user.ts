import { supervisor, type OperationOptions } from "@/proto/services";
import {
  BenchData,
  BenchType,
  ClientData,
  NodeReferenceData,
  NodeType,
  Region,
  UserData,
  UserProperty,
  UserStatus,
  ViewType,
} from "@/proto/wire";
import { makeNode, nodeReference, propertyReference, toNodeReferenceRef, toProtoOneOf } from "@/proto/wiring";
import { ACTION_COMING_SOON, contributeActionMap } from "@/system/action";
import local, { persistentInfo } from "@/system/client";
import { clearConnections, useGetConnection } from "@/system/connection";
import { canvas, goToBench } from "@/system/space";
import { toaster } from "@/system/toast";
import { log } from "@/utils/log";
import type { ViewDataIn } from "@/views/canvas";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { computed, ref } from "vue";

export const isAuthenticated = computed(() => local.clientInfo.value?.accessToken != null);
export const isUnauthenticated = computed(() => !isAuthenticated.value);

export const { graph: userGraph, connection: userConnection } = useGetConnection(
  { name: "user", live: true, paramsPretty: computed(() => ({ slug: local.userInfo.value?.slug })) },
  computed(() => ({
    roots: [nodeReference(NodeType.USER, local.userInfo.value?.id!)],
    options: {
      descendantTypes: [NodeType.CLIENT],
      includePropertiesPtr: [propertyReference(BenchType.USER, UserProperty.email)],
    },
    enabled: isAuthenticated.value,
  })),
);
export const user = userGraph.getRef(
  computed(() => (isAuthenticated.value ? { type: NodeType.USER, id: local.userInfo.value?.id! } : null)),
);
export const client = userGraph.getRef(
  computed(() =>
    local.clientInfo.value?.id == null ? null : { type: NodeType.CLIENT, id: local.clientInfo.value.id },
  ),
);
export const clients = userGraph.getChildrenRef(toNodeReferenceRef(user), NodeType.CLIENT);
export const clientsSorted = computed(() => {
  // sort clients so we are first, then descending by last active
  return clients.value.slice().sort((a, b) => {
    if (a.id == local.clientInfo.value?.id) return -1;
    if (b.id == local.clientInfo.value?.id) return 1;
    if (a.lastSeenAt == null) return -1;
    if (b.lastSeenAt == null) return 1;
    return Number(b.lastSeenAt.seconds - a.lastSeenAt.seconds);
  });
});
export const isActivated = computed(() => user.value?.status == UserStatus.ACTIVATED);

function makeCurrentClient(): ClientData {
  return makeNode(
    {
      metatype: NodeType.CLIENT,
      placeId: persistentInfo.value!.placeId,
      benchPtr: undefined,
      ...local.clientMeta.value,
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

/**
 * Sign up a new user and simultaneously log in as the current client.
 */
export async function signUp(
  userIn: { name?: string; slug: string; email: string },
  password: string,
  options?: OperationOptions,
) {
  if (isAuthenticated.value) throw new Error("already logged in");
  const {
    response: { user, client, accessToken },
  } = await supervisor.signupUser(
    {
      ...userIn,
      password,
      client: makeCurrentClient(),
    },
    options,
  );
  if (user == null || client == null) throw new Error("unexpected null user or client");
  onLogIn({ user, client, accessToken });
  toaster.info({ icon: "fas fa-right-from-bracket", title: "Signed Up", text: `Welcome, ${user.slug}.` });
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
  } = await supervisor.loginUser({ user: toProtoOneOf(userIn), password, client: makeCurrentClient() }, options);
  if (user == null || client == null) throw new Error("unexpected null user or client");
  onLogIn({ user, client, accessToken });
  toaster.info({ icon: "fas fa-right-from-bracket", title: "Logged In", text: `Welcome back, ${user.slug}.` });
  return { user, client };
}

/**
 * Logs out clients (may include current).
 */
export async function logOut(logOut?: { all?: boolean; clients?: { id: string }[] }, options?: OperationOptions) {
  if (local.clientInfo.value == null) throw new Error("not logged in");
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
    local.clearUser();
    if (user.value?.mainBenchPtr?.id == local.benchPtr.value?.id) {
      // reset local space
      local.clearBench();
    }
    clearConnections();
    toaster.info({ icon: "fas fa-right-to-bracket", title: "Logged out", text: "Thanks for all the fish." });
  }
}

/** Reports an authentication error from a request using our current credentials */
export function onAuthenticationError(error: RpcError) {
  // TODO :Robustness: handle user auth error & badge auth error separately
  log.error("user.unauthenticated");
  local.clearUser();
}

export async function createBench(
  benchIn: {
    owner: NodeReferenceData;
    slug: string;
    region: Region;
    isMain: boolean;
  },
  options?: OperationOptions,
): Promise<{ bench: BenchData }> {
  if (local.clientInfo.value == null) throw new Error("not logged in");
  const {
    response: { bench },
  } = await supervisor.createBench({ ...benchIn }, options);
  if (bench == null) throw new Error("failed to create bench");
  return { bench };
}

function userWizardView(view: { title: string }): ViewDataIn {
  return { type: ViewType.USER_WIZARD, icon: "fas fa-right-from-bracket", ...view };
}

contributeActionMap<"user">({
  "user.auth.signup": {
    icon: "fas fa-right-from-bracket",
    title: "Sign Up",
    text: "Create a new account.",
    enabled: isUnauthenticated,
    action: () => canvas.upsertView(userWizardView({ title: "Sign Up" })),
  },
  "user.auth.login": {
    icon: "fas fa-right-from-bracket",
    title: "Log In",
    text: "Log in to an existing account.",
    enabled: isUnauthenticated,
    action: () => canvas.upsertView(userWizardView({ title: "Log In" })),
  },
  "user.auth.logout": {
    icon: "fas fa-right-to-bracket",
    title: "Log Out",
    text: "Log out of the current client.",
    enabled: isAuthenticated,
    action: () => logOut(),
  },
  "user.auth.logoutAll": {
    icon: "fas fa-right-to-bracket",
    title: "Log Out All",
    text: "Log out all clients (including current).",
    enabled: isAuthenticated,
    action: () => logOut({ all: true }),
  },
  "user.auth.activate": {
    icon: "fas fa-rocket-launch",
    enabled: computed(() => isAuthenticated.value && !isActivated.value),
    title: "Activate Bench",
    text: "Activate your account by creating your Bench.",
    action: () =>
      canvas.upsertView({ type: ViewType.BENCH_WIZARD, icon: "fas fa-rocket-launch", title: "Activate Bench" }),
  },
  "user.misc.goToHome": {
    icon: "fas fa-home",
    enabled: isActivated,
    title: "Go to My Bench",
    text: "Go back to your Bench.",
    action: async () => {
      await goToBench({ bench: user.value!.mainBenchPtr! });
    },
  },
  "user.settings.editKeybindings": {
    icon: "fas fa-keyboard",
    enabled: ref(false),
    title: "Edit Keybindings",
    text: "Customize your keybindings everywhere.",
    action: ACTION_COMING_SOON,
  },
});
