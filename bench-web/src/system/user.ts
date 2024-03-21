import { supervisor } from "@/proto/services";
import {
  ClientData,
  NodeReferenceData,
  NodeType,
  Region,
  UserData,
  UserStatus,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { makeNode, nodeReference, toNodeReferenceRef, toProtoOneOf } from "@/proto/wiring";
import { ACTION_COMING_SOON, contributeActionMap } from "@/system/action";
import { useGetNodes } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { clientInfo, clientMeta, userInfo } from "@/system/local";
import { canvas } from "@/system/space";
import { toaster } from "@/system/toast";
import { log } from "@/utils/log";
import type { ViewDataIn } from "@/views/canvas";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { v4 } from "uuid";
import { computed } from "vue";

export const nonce = v4();

export const isAuthenticated = computed(() => clientInfo.value?.accessToken != null);
export const isUnauthenticated = computed(() => !isAuthenticated.value);

export const { graph: userGraph, connection: userConnection } = useGetNodes(
  computed(() => ({
    roots: [nodeReference(NodeType.USER, userInfo.value?.id!)],
    options: { descendantTypes: [NodeType.CLIENT] },
    enabled: isAuthenticated.value,
    watch: true,
  })),
);
export const user = userGraph.getRef(
  computed(() => (isAuthenticated.value ? { type: NodeType.USER, id: userInfo.value?.id! } : null)),
);
export const client = userGraph.getRef(
  computed(() => (clientInfo.value?.id == null ? null : { type: NodeType.CLIENT, id: clientInfo.value.id })),
);
export const clients = userGraph.getChildrenRef(toNodeReferenceRef(user), NodeType.CLIENT);
export const isActivated = computed(() => user.value?.status == UserStatus.ACTIVATED);

function makeCurrentClient(): ClientData {
  return makeNode({
    metatype: NodeType.CLIENT,
    ...clientMeta.value,
  });
}

function onLogIn(info: { user: UserData; client: ClientData; accessToken: string }) {
  userInfo.value = {
    id: info.user.id,
    email: info.user.email!,
    name: info.user.name!,
    slug: info.user.slug!,
  };
  clientInfo.value = {
    id: info.client.id,
    accessToken: info.accessToken,
  };
}

/**
 * Sign up a new user and simultaneously log in as the current client.
 */
export async function signUp(userIn: { name?: string; slug: string; email: string }, password: string) {
  if (isAuthenticated.value) throw new Error("already logged in");
  const {
    response: { user, client, accessToken },
  } = await supervisor.signupUser({
    ...userIn,
    password,
    client: makeCurrentClient(),
  });
  if (user == null || client == null) throw new Error("unexpected null user or client");
  onLogIn({ user, client, accessToken });
  toaster.info({ icon: "fas fa-right-from-bracket", title: "Signed Up", text: `Welcome, ${user.slug}.` });
}

/**
 * Log in a user as the current client.
 */
export async function logIn(userIn: { slug: string } | { email: string }, password: string) {
  if (clientInfo.value != null) throw new Error("already logged in");
  const {
    response: { user, client, accessToken },
  } = await supervisor.loginUser({ user: toProtoOneOf(userIn), password, client: makeCurrentClient() });
  if (user == null || client == null) throw new Error("unexpected null user or client");
  onLogIn({ user, client, accessToken });
  toaster.info({ icon: "fas fa-right-from-bracket", title: "Logged In", text: `Welcome back, ${user.slug}.` });
}

/**
 * Logs out clients (may include current).
 */
export async function logOut(options?: { all?: boolean; clients?: { id: string }[] }) {
  if (clientInfo.value == null) throw new Error("not logged in");
  await supervisor.logoutUser({
    clients: options?.clients?.map((c) => nodeReference(NodeType.CLIENT, c.id!)) ?? [],
    logoutAll: options?.all,
  });
  if (options == null || options?.all || options?.clients?.some((c) => c.id == clientInfo.value?.id)) {
    // logged out current client
    log.info("user.logout", options);
    userInfo.value = null;
    clientInfo.value = null;
    toaster.info({ icon: "fas fa-right-to-bracket", title: "Logged out", text: "Thanks for all the fish." });
  }
}

/** Reports an authentication error from a request using our current credentials */
export function onAuthenticationError(error: RpcError) {
  // TODO :Robustness: handle user auth error & badge auth error separately
  log.error("user.unauthenticated");
  userInfo.value = null;
  clientInfo.value = null;
}

export async function createBench(benchIn: {
  owner: NodeReferenceData;
  slug: string;
  region: Region;
  isMain: boolean;
}) {
  if (clientInfo.value == null) throw new Error("not logged in");
  const {
    response: { bench },
  } = await supervisor.createBench({ ...benchIn });
}

function userWizardView(view: { title: string }): ViewDataIn {
  return { type: ViewType.USER_WIZARD, icon: "fas fa-right-from-bracket", ...view };
}

contributeActionMap<"user">({
  "user.signup": {
    icon: "fas fa-right-from-bracket",
    title: "Sign Up",
    enabled: isUnauthenticated,
    action: () => canvas.upsertView(userWizardView({ title: "Sign Up" })),
  },
  "user.login": {
    icon: "fas fa-right-from-bracket",
    title: "Log In",
    enabled: isUnauthenticated,
    action: () => canvas.upsertView(userWizardView({ title: "Log In" })),
  },
  "user.logout": {
    icon: "fas fa-right-to-bracket",
    title: "Log Out",
    enabled: isAuthenticated,
    action: () => logOut(),
  },
  "user.activate": {
    icon: "fas fa-rocket-launch",
    enabled: computed(() => isAuthenticated.value && !isActivated.value),
    title: "Activate Bench",
    action: () =>
      canvas.upsertView({ type: ViewType.BENCH_WIZARD, icon: "fas fa-rocket-launch", title: "Activate Bench" }),
  },
  "user.goToHome": {
    icon: "fas fa-home",
    enabled: isActivated,
    title: "Go to My Bench",
    action: ACTION_COMING_SOON,
  },
});
