import { isResourceNodeType } from "@/language/const";
import { newChangeId, Transaction } from "@/language/transaction";
import { AnyNodeData, BrowserData, MachineData, NodeType, ResourceStatus, Timestamp } from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { Action, ActionContext, getNodesForAction, provideActions } from "@/ui/action";

export type ResourceData = MachineData | BrowserData; // | ...

/** Provision or activate a Resource. */
export function activateResource(tx: Transaction, resource: ResourceData) {
  tx.update(resource, { activatedAt: Timestamp.now() });
}

/** Decommission a Resource. */
export function decommissionResource(tx: Transaction, resource: ResourceData) {
  tx.update(resource, { decommissionedAt: Timestamp.now() });
}

/** Suspend a Resource. */
export function suspendResource(tx: Transaction, resource: ResourceData) {
  tx.update(resource, { suspendedAt: Timestamp.now() });
}

function applyResourceAction(
  action: Action,
  context: ActionContext,
  method: (tx: Transaction, resource: ResourceData) => void,
) {
  const { connection, graph, nodes } = getNodesForAction(action, context);
  if (connection == null) return false;
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Provision" } });
  nodes
    .filter((node) => isResourceNodeType(node.metatype as unknown as NodeType))
    .forEach((node) => method(tx, node as ResourceData));
}

// resource
export const RESOURCE_ACTIONS = provideActions<"resource">({
  "resource.status.activate": {
    icon: "fas fa-power-off",
    title: "Provision",
    text: "Provision this Resource",
    isEnabled: (action, context) => {
      const { nodes } = getNodesForAction(action, context);
      return (
        nodes.every((n) => isResourceNodeType(n.metatype as any)) &&
        nodes.some(
          (n) =>
            (n as ResourceData).status != ResourceStatus.UP &&
            (n as ResourceData).status != ResourceStatus.DECOMMISSIONED,
        )
      );
    },
    action: (action, context) => applyResourceAction(action, context, activateResource),
  },
  "resource.status.suspend": {
    icon: "fas fa-snooze",
    title: "Suspend",
    text: "Suspend this Resource",
    isEnabled: (action, context) => {
      const { nodes } = getNodesForAction(action, context);
      return (
        nodes.every((n) => isResourceNodeType(n.metatype as any)) &&
        nodes.some((n) => (n as ResourceData).status == ResourceStatus.UP)
      );
    },
    action: (action, context) => applyResourceAction(action, context, suspendResource),
  },
  "resource.status.decommission": {
    icon: "fas fa-skull",
    title: "Decommission",
    text: "Decommission this Resource",
    isEnabled: (action, context) => {
      const { nodes } = getNodesForAction(action, context);
      return (
        nodes.every((n) => isResourceNodeType(n.metatype as any)) &&
        nodes.some((n) => (n as ResourceData).status == ResourceStatus.UP)
      );
    },
    action: (action, context) => applyResourceAction(action, context, decommissionResource),
  },
});
