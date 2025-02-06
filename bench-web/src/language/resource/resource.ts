import { isResourceNode } from "@/language/core/const";
import { newChangeId, Transaction } from "@/language/runtime/transaction";
import { ResourceNodeData, ResourceStatus, Timestamp } from "@/proto/wire";
import { Action, ActionContext, getNodesForAction, provideActions } from "@/ui/action";

/** Provision or activate a Resource. */
export function activateResource(tx: Transaction, resource: ResourceNodeData) {
  tx.update(resource, { activatedAt: Timestamp.now() });
}

/** Decommission a Resource. */
export function decommissionResource(tx: Transaction, resource: ResourceNodeData) {
  tx.update(resource, { decommissionedAt: Timestamp.now() });
}

/** Suspend a Resource. */
export function suspendResource(tx: Transaction, resource: ResourceNodeData) {
  tx.update(resource, { suspendedAt: Timestamp.now() });
}

function applyResourceAction(
  action: Action,
  context: ActionContext | undefined,
  method: (tx: Transaction, resource: ResourceNodeData) => void,
) {
  const { connection, graph, nodes } = getNodesForAction(action, context, isResourceNode);
  if (connection == null) return false;
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Provision" } });
  nodes.forEach((node) => method(tx, node as ResourceNodeData));
  return true;
}

function isResourceActionEnabled(
  action: Action,
  context: ActionContext | undefined,
  statusPredicate: (status: ResourceStatus) => boolean,
): boolean {
  const nodes = getNodesForAction(action, context, isResourceNode).nodes;
  return nodes.every((n) => statusPredicate((n as ResourceNodeData).status));
}

// resource
export const RESOURCE_ACTIONS = provideActions<"resource">({
  "resource.status.activate": {
    icon: "fas fa-power-off",
    title: "Provision",
    text: "Provision this Resource",
    isEnabled: (action, context) =>
      isResourceActionEnabled(
        action,
        context,
        (status) => status != ResourceStatus.UP && status != ResourceStatus.DECOMMISSIONED,
      ),
    action: (action, context) => applyResourceAction(action, context, activateResource),
  },
  "resource.status.suspend": {
    icon: "fas fa-snooze",
    title: "Suspend",
    text: "Suspend this Resource",
    isEnabled: (action, context) => isResourceActionEnabled(action, context, (status) => status == ResourceStatus.UP),
    action: (action, context) => applyResourceAction(action, context, suspendResource),
  },
  "resource.status.decommission": {
    icon: "fas fa-skull",
    title: "Decommission",
    text: "Decommission this Resource",
    isEnabled: (action, context) => isResourceActionEnabled(action, context, (status) => status == ResourceStatus.UP),
    action: (action, context) => applyResourceAction(action, context, decommissionResource),
  },
});
