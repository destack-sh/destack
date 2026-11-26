import { t } from "@destack/locale";
import { current, defineAction } from "@destack/object";
import { schema } from "@destack/schema";
import { messageType } from "./object.example.ts";

/** React to a message, or take the reaction back. */
export const reactToMessage = defineAction({
    name: "message.react",
    title: t.context("action")`React`,
    category: t.context("title")`Message`,
    input: { message: current(messageType), emoji: schema.string() },
});

/** Answer a message in a thread. */
export const replyInThread = defineAction({
    name: "message.thread",
    title: t`Reply in thread`,
    category: t.context("title")`Message`,
    keybinding: "t",
    input: { message: current(messageType) },
});

/** Answer a message with its words over the answer. */
export const quoteMessage = defineAction({
    name: "message.quote",
    title: t.context("action")`Quote`,
    category: t.context("title")`Message`,
    keybinding: "q",
    input: { message: current(messageType) },
});

/** Write a message anew. */
export const editMessage = defineAction({
    name: "message.edit",
    title: t.context("action")`Edit`,
    category: t.context("title")`Message`,
    keybinding: "e",
    input: { message: current(messageType) },
});

/** Copy the words of a message. */
export const copyMessage = defineAction({
    name: "message.copy",
    title: t.context("action")`Copy text`,
    category: t.context("title")`Message`,
    keybinding: "c",
    input: { message: current(messageType) },
});

/** Pin a message to its conversation. */
export const pinMessage = defineAction({
    name: "message.pin",
    title: t.context("action")`Pin`,
    category: t.context("title")`Message`,
    keybinding: "p",
    input: { message: current(messageType) },
});

/** Read a conversation up to before a message. */
export const markUnread = defineAction({
    name: "message.markUnread",
    title: t`Mark unread from here`,
    category: t.context("title")`Message`,
    keybinding: "u",
    input: { message: current(messageType) },
});

/** Delete a message and its thread. */
export const deleteMessage = defineAction({
    name: "message.delete",
    title: t.context("action")`Delete`,
    category: t.context("title")`Message`,
    keybinding: { key: "delete", mac: ["backspace", "delete"] },
    isDestructive: true,
    input: { message: current(messageType) },
});

/** Answer a question once more. */
export const retryAnswer = defineAction({
    name: "answer.retry",
    title: t.context("action")`Try again`,
    category: t.context("title")`Message`,
    keybinding: "r",
    input: { message: current(messageType) },
});

/** Start a conversation with the assistant. */
export const newChat = defineAction({
    name: "chat.create",
    title: t.context("action")`New chat`,
    category: t.context("title")`Chat`,
    keybinding: "shift+n",
});
