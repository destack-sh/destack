import { isResourceNodeType } from "@/language/const";
import { newChangeId, Transaction } from "@/language/transaction";
import { BrowserData, MachineData, NodeType, ResourceStatus, Timestamp } from "@/proto/wire";
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
  context: ActionContext | undefined,
  method: (tx: Transaction, resource: ResourceData) => void,
) {
  const { connection, graph, nodes } = getNodesForAction(action, context);
  if (connection == null) return false;
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Provision" } });
  nodes
    .filter((node) => isResourceNodeType(node.metatype as unknown as NodeType))
    .forEach((node) => method(tx, node as ResourceData));
}

function isResourceActionEnabled(
  action: Action,
  context: ActionContext | undefined,
  statusPredicate: (status: ResourceStatus) => boolean,
): boolean {
  const nodes = getNodesForAction(action, context).nodes;
  return (
    nodes.every((n) => isResourceNodeType(n.metatype as any)) &&
    nodes.some((n) => statusPredicate((n as ResourceData).status))
  );
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
