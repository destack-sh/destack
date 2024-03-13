import { supervisor } from "@/proto/services";
import { ClientData, NodeReferenceData, NodeType, Region, UserData } from "@/proto/wire";
import { makeNode, nodeReference, toNodeReferenceRef, toProtoOneOf } from "@/proto/wiring";
import { useGetNodes } from "@/system/connection";
import { clientInfo, clientMeta, userInfo } from "@/system/local";
import { log } from "@/utils/log";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { v4 } from "uuid";
import { computed } from "vue";

export const nonce = v4();

export const isAuthenticated = computed(() => clientInfo.value?.accessToken != null);

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
}

/**
 * Logs out clients (may include current).
 */
export async function logOut(options?: { all?: boolean; clients?: { id: string }[] }) {
  if (clientInfo.value == null) throw new Error("not logged in");
  const clients = (options?.clients ?? [clientInfo.value]).map((c) => nodeReference(NodeType.CLIENT, c.id!));
  await supervisor.logoutUser({ clients, logoutAll: options?.all });
  if (options?.all || clients.some((c) => c.id == clientInfo.value?.id)) {
    // logged out current client
    log.info("user.logout", options);
    userInfo.value = null;
    clientInfo.value = null;
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
  const { response: { bench } } = await supervisor.createBench({ ...benchIn });
}
