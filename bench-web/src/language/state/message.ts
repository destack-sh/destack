import { getBaseFromNodeReference } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { NodeIn } from "@/language/core/node";
import { Transaction } from "@/language/runtime/transaction";
import { MessageData, MessageStatus, NodeReferenceData, NodeType } from "@/proto/wire";

/** Create a Message. */
export function createMessage(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    message: Partial<NodeIn<NodeType.MESSAGE>> & Required<Pick<NodeIn<NodeType.MESSAGE>, "type">>;
  },
): MessageData {
  const message = tx.create({
    metatype: NodeType.MESSAGE,
    ...options.message,
    status: options.message.status ?? MessageStatus.SENT,
  });

  return message;
}

export function getMessageAuthorPtr(message: MessageData): NodeReferenceData | null {
  if (message.createdByPtr == null) {
    return null;
  } else if (message.createdByPtr.nodeType == NodeType.USER || message.createdByPtr.nodeType == NodeType.BLOCK) {
    return message.createdByPtr;
  } else if (message.createdByPtr.nodeType == NodeType.RUN) {
    return getBaseFromNodeReference(message.createdByPtr);
  } else {
    return null;
  }
}

