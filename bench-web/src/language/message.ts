import { getBaseFromNodeReference } from "@/language/const";
import { ReadNodeGraph } from "@/language/graph";
import { NodeIn } from "@/language/node";
import { Transaction } from "@/language/transaction";
import { MessageData, MessageStatus, NodeReferenceData, NodeType } from "@/proto/wire";
import { declareActions } from "@/ui/action";

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

// actions
declareActions<"chat">({
  "chat.message.reply": {
    title: "Reply",
    text: "Reply to the Message",
    icon: "fas fa-reply",
  },
  "chat.message.forward": {
    title: "Forward",
    text: "Forward the Message",
    icon: "fas fa-share",
  },
  "chat.message.edit": {
    title: "Edit",
    text: "Edit the Message",
    icon: "fas fa-pencil",
  },
});
