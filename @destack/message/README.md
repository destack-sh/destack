# @destack/message

Send messages on a channel, email, push, webhook or a desktop.

## Overview

`@destack/message` is outbound messaging: email, Web Push, Standard Webhooks and desktop messages, with encrypted content and retries.

Producers send messages:

- `message`: one message to one destination.
- `Destination`: an email address, a browser's push subscription, a webhook endpoint or a desktop client.
- `Content`: an email, a push payload or a webhook body.
- `MessageKey`: the key producers encrypt content to.

The installation sends them:

- `MessageProvider`: sends on one channel.
- `Outcome`: one try's result: sent, deferred or failed.
- `attempt`: one recorded try.

Scopes subscribe webhooks:

- `endpoint`: a URL receiving signed webhooks.
- `MessageError`: the reasons a message is refused, checked with `MessageError.is`.

Encrypt an email's content, then send it:

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { MessageKey } from "@destack/message/client";
import type messagePackage from "@destack/message/package";
import type { ClientOf } from "@destack/object/client";
import type { ObjectReference } from "@destack/sync";

/** Email a person that an issue was resolved. */
export async function emailResolved(
    client: ClientOf<typeof messagePackage>,
    issue: ObjectReference,
    requestId: string,
) {
    // encrypt the content for the scope
    const scope = issue.scope;
    const { key } = await client.message.contentKey({ scope });
    const content = {
        channel: "email",
        subject: "Issue resolved",
        text: "Issue 42 is resolved.",
    } as const;
    const ciphertext = await MessageKey.of(key, BunEnvironment.system).seal(content, scope);

    // send it
    await client.message.create({
        scope,
        requestId,
        to: { channel: "email", address: "dana@example.com" },
        sourcePackageId: issue.packageId,
        sourceType: issue.type,
        sourceScope: issue.scope,
        sourceId: issue.id,
        ciphertext,
    });
}
```

## Reference

### Channels

Each channel has its destination and its content.
A push's `urgency` is `"very-low"`, `"low"`, `"normal"` or `"high"`, and its `topic` replaces an earlier push (RFC 8030).

| Channel     | `Destination`                           | `Content`                                                                          |
| ----------- | --------------------------------------- | ---------------------------------------------------------------------------------- |
| `"email"`   | `{ address }`                           | `{ subject, text, html }`                                                          |
| `"push"`    | `{ client, user }`, a browser           | `{ data, urgency, ttl, topic }`, `data` at most 3993 bytes of JSON                 |
| `"webhook"` | `{}`, the endpoint the source names     | `{ type, format: "event", data }`, or `{ type, format: "json" \| "ndjson", data }` |
| `"desktop"` | `{ client }`, a signed-in native client | none                                                                               |

### Sending

A deferred try retries from 5 seconds, doubling to 15 minutes, eight times.
`emailProvider(transport)`, `pushProvider(vapid, fetch)` and `webhookProvider(fetch)` send each channel.
`Outcome.read(response, now)` reads an HTTP answer, honoring `Retry-After`.

| Answer                        | Outcome                                                                                |
| ----------------------------- | -------------------------------------------------------------------------------------- |
| a 2xx status                  | `sent`                                                                                 |
| 404 or 410                    | `failed` with `RECIPIENT_GONE_NOT_FOUND`                                               |
| 408, 425, 429 or a 5xx status | `deferred` with `PROVIDER_UNREACHABLE`, `PROVIDER_UNAVAILABLE` or `PROVIDER_EXHAUSTED` |
| any other status              | `failed` with `MESSAGE_INVALID`                                                        |

## Examples

### Push to a person's browser

A push goes to one browser client of a person, with a payload, an urgency, a time to live, and a topic that replaces an earlier push.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { MessageKey } from "@destack/message/client";
import type messagePackage from "@destack/message/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";

/** Push a new comment to a person's browser, replacing an unread one about the same issue. */
export async function pushComment(
    client: ClientOf<typeof messagePackage>,
    issue: ObjectReference,
    browser: { readonly client: Identifier<"client">; readonly user: Identifier<"user"> },
    requestId: string,
) {
    // encrypt the payload for the scope
    const scope = issue.scope;
    const { key } = await client.message.contentKey({ scope });
    const content = {
        channel: "push",
        data: { issue: issue.id, text: "New comment" },
        urgency: "normal",
        ttl: 3600,
        topic: `issue-${issue.id.slice(-8)}`,
    } as const;
    const ciphertext = await MessageKey.of(key, BunEnvironment.system).seal(content, scope);

    // send it to the browser
    await client.message.create({
        scope,
        requestId,
        to: { channel: "push", ...browser },
        sourcePackageId: issue.packageId,
        sourceType: issue.type,
        sourceScope: issue.scope,
        sourceId: issue.id,
        ciphertext,
    });
}
```

### Subscribe a webhook endpoint

An endpoint receives a scope's member changes as signed events.

```ts
import type messagePackage from "@destack/message/package";
import type { ClientOf } from "@destack/object/client";
import type { ObjectReference } from "@destack/sync";

/** Send a scope's member changes to a team's hook, signed with a vault secret. */
export async function subscribeMembers(
    client: ClientOf<typeof messagePackage>,
    scope: string,
    secret: ObjectReference,
    requestId: string,
) {
    await client.endpoint.create({
        scope,
        requestId,
        url: "https://hooks.example.com/destack",
        events: ["member.create", "member.delete"],
        format: "event",
        authentication: { kind: "signature" },
        secret,
    });
}
```

### Test sending

`TestMessage` serves a space with scripted mail, webhooks and push.

```ts
import { TestMessage } from "@destack/message/test";
import { expect, test } from "@destack/test";

test("send an email on the first try", async () => {
    // serve the space
    const kit = await TestMessage.open({ mails: ["sent"] });

    // send an email, and read how it went
    const id = await kit.send(
        { channel: "email", address: "dana@example.com" },
        { channel: "email", subject: "Hello", text: "Hi Dana" },
    );
    const sending = await kit.sending(id);

    expect(sending.status).toBe("sent");
});
```

## Comparison

### Standard Webhooks

Endpoints sign each message as Standard Webhooks does.

| Standard Webhooks                                      | `@destack/message`         | Difference                                                 |
| ------------------------------------------------------ | -------------------------- | ---------------------------------------------------------- |
| `webhook-id`, `webhook-timestamp`, `webhook-signature` | the same                   | The same.                                                  |
| `{ type, timestamp, data }`                            | `format: "event"`          | The same, or pages as JSON or NDJSON.                      |
| a 15 second timeout                                    | the same                   | The same.                                                  |
| none                                                   | `{ kind: "header", name }` | Sends the secret in a header for hooks without signatures. |

### Web Push

Pushes follow Web Push: RFC 8291 encryption, VAPID, and RFC 8030's headers.

| Web Push                            | `@destack/message`         | Difference                            |
| ----------------------------------- | -------------------------- | ------------------------------------- |
| `aes128gcm` encryption (RFC 8291)   | the same                   | The same.                             |
| VAPID (RFC 8292)                    | `serverKey`                | Derived from the installation's keys. |
| `TTL`, `Urgency`, `Topic`           | `ttl`, `urgency`, `topic`  | The same, set per message's content.  |
| 404 and 410 for a gone subscription | `RECIPIENT_GONE_NOT_FOUND` | Fails the message for good.           |
