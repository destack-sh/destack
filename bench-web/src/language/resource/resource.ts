import { isResourceNode } from "@/language/core/const";
import { newChangeId, Transaction } from "@/language/runtime/transaction";
import { ResourceNodeData, ResourceStatus, Timestamp } from "@/proto/wire";
import { Command, CommandContext, getNodesForCommand, provideCommands } from "@/ui/command";

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

function applyResourceCommand(
  command: Command,
  context: CommandContext | undefined,
  method: (tx: Transaction, resource: ResourceNodeData) => void,
) {
  const { connection, graph, nodes } = getNodesForCommand(command, context, isResourceNode);
  if (connection == null) return false;
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Provision" } });
  nodes.forEach((node) => method(tx, node as ResourceNodeData));
  return true;
}

function isResourceCommandEnabled(
  command: Command,
  context: CommandContext | undefined,
  statusPredicate: (status: ResourceStatus) => boolean,
): boolean {
  const nodes = getNodesForCommand(command, context, isResourceNode).nodes;
  return nodes.every((n) => statusPredicate((n as ResourceNodeData).status));
}

// resource
export const RESOURCE_COMMANDS = provideCommands<"resource">({
  "resource.status.activate": {
    icon: "fas fa-power-off",
    title: "Provision",
    text: "Provision this Resource",
    isEnabled: (command, context) =>
      isResourceCommandEnabled(
        command,
        context,
        (status) => status != ResourceStatus.AVAILABLE && status != ResourceStatus.OFFLINE,
      ),
    command: (command, context) => applyResourceCommand(command, context, activateResource),
  },
  "resource.status.suspend": {
    icon: "fas fa-snooze",
    title: "Suspend",
    text: "Suspend this Resource",
    isEnabled: (command, context) =>
      isResourceCommandEnabled(command, context, (status) => status == ResourceStatus.AVAILABLE),
    command: (command, context) => applyResourceCommand(command, context, suspendResource),
  },
  "resource.status.decommission": {
    icon: "fas fa-skull",
    title: "Decommission",
    text: "Decommission this Resource",
    isEnabled: (command, context) =>
      isResourceCommandEnabled(command, context, (status) => status == ResourceStatus.AVAILABLE),
    command: (command, context) => applyResourceCommand(command, context, decommissionResource),
  },
});
