import { type Content, MessageKey, message } from "@destack/message";
import type { MethodContext } from "@destack/object";
import { Duration, type Identifier } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import type { RecipientDestination } from "../message/index.ts";
import type { NotificationContent } from "../notification/content.ts";
import type { Activity } from "../object/activity.ts";
import { Notification } from "../object/notification.ts";
import type { InterruptionLevel } from "../preference/preference.ts";
import type { NotificationServer } from "./server.ts";

/** How urgently each interruption level's push goes out, as RFC 8030's urgencies. */
const URGENCY = {
    passive: "very-low",
    active: "normal",
    timeSensitive: "high",
    critical: "high",
} as const satisfies Record<InterruptionLevel, string>;

/** The notification a push shows, with its activity. */
export interface PushNotice {
    /** The notification. */
    readonly notification: Notification["id"];
    /** Its activity's last occurrence. */
    readonly occurrence: Activity;
}

/** Read the key the home's message content is sealed to. */
export async function readMessageKey(context: MethodContext): Promise<MessageKey> {
    const sealing = message.call("contentKey", message.in(context.scope));

    return MessageKey.of((await context.query(context.self, sealing)).key, context.environment);
}

/** Send a home's message for a notification, its content sealed to the home's message key. */
export async function createMessage(
    context: MethodContext,
    key: MessageKey,
    to: RecipientDestination,
    content: Content | undefined,
    source: ObjectReference,
): Promise<Identifier<"message">> {
    // seal the content, showing a desktop message without content
    const ciphertext = content === undefined ? undefined : await key.seal(content, context.scope);

    // create the message for its notification
    const [created] = await context.mutate(context.self, [
        message.call("create", {
            ...message.in(context.scope),
            to,
            ...ObjectReference.columns("source", source),
            ...(ciphertext === undefined ? {} : { ciphertext }),
        }),
    ]);

    return created.id;
}

/** Compose a message's content for its destination: none to a desktop, an email, or a push. */
export function compose(
    server: NotificationServer,
    to: RecipientDestination,
    content: NotificationContent,
    notice: PushNotice | undefined,
    home: string,
): Content | undefined {
    // show a desktop message without content
    if (to.channel === "desktop") {
        return undefined;
    }
    // email the content
    else if (to.channel === "email") {
        const text = [content.subtitle, content.body].filter((line) => line !== undefined);

        return { channel: "email", subject: content.title, text: text.join("\n\n") };
    }

    // push a summary, or one notification with its activity and actions
    const data =
        notice === undefined
            ? { scope: home, content, actions: [] }
            : {
                  notification: notice.notification,
                  scope: home,
                  content,
                  activity: { scope: notice.occurrence.scope, id: notice.occurrence.id },
                  actions: notice.occurrence.actions.map((action) => ({
                      name: action.name,
                      title: action.title,
                      isDestructive: action.isDestructive === true,
                      hasText: action.text !== undefined,
                  })),
              };
    const level = notice === undefined ? "passive" : notice.occurrence.interruption;

    return {
        channel: "push",
        data,
        urgency: URGENCY[level],
        ttl: Duration.seconds(server.pushTtl),
        ...(notice === undefined ? {} : { topic: Notification.topic(notice.notification) }),
    };
}
