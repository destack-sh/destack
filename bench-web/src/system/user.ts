import { supervisor } from "@/proto/services";
import { NodeType } from "@/proto/wire";
import { nodeReference, toNodeReferenceRef } from "@/proto/wiring";
import { useGetNodes } from "@/system/connection";
import { clientInfo, userInfo } from "@/system/local";
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

export async function logIn(key: { username: string } | { email: string }, password: string) {
  // const { response: { user, client, accessToken } } = await supervisor.loginUser({ user: key, password });
  throw new Error("not implemented");
}
