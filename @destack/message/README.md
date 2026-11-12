# @destack/message

Send messages on a channel, email, push, webhook or a desktop.

## Overview

The message installation sends each message to one recipient on one channel, its content sealed to the installation's key until it sends.

- **Message**: one message to one destination about one source object; `create` sends it, and a desktop client marks it delivered with `receive`.
- **Destination**: an email address, a browser's push subscription, a webhook or a desktop client.
- **MessageKey**: the installation's key that producers seal content to; `contentKey` reads its public half.
- **Endpoint**: a URL receiving the scope's calls as signed webhook messages.

```ts
import { MessageKey } from "@destack/message/client";
import type messagePackage from "@destack/message/package";
import { BunEnvironment } from "@destack/environment/bun";
import type { ClientOf } from "@destack/object/client";
import type { ObjectReference } from "@destack/sync";

async (client: ClientOf<typeof messagePackage>, source: ObjectReference, requestId: string) => {
    const scope = source.scope;
    const { key } = await client.message.contentKey({ scope });
    const content = {
        channel: "email",
        subject: "Your issue was resolved",
        text: "Issue 42 is resolved.",
    } as const;
    const ciphertext = await MessageKey.of(key, BunEnvironment.system).seal(content, scope);
    await client.message.create({
        scope,
        requestId,
        to: { channel: "email", address: "dana@example.com" },
        sourcePackageId: source.packageId,
        sourceType: source.type,
        sourceScope: source.scope,
        sourceId: source.id,
        ciphertext,
    });
};
```

## Examples

### Install message

A space installs message with its outbound mail bound to a transport and its keys in a vault.

```sh
destack install @destack/message
```

### Receive a desktop message

A desktop message shows at once on its signed-in client, which marks it delivered with `receive`.

```ts
import type messagePackage from "@destack/message/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof messagePackage>,
    scope: string,
    id: Identifier<"message">,
    requestId: string,
) => {
    await client.message.receive({ scope, id, requestId });
};
```

### Subscribe a webhook endpoint

An endpoint receives the scope's calls as Standard Webhooks messages, signed with a vault secret the installation consumes.

```ts
import type messagePackage from "@destack/message/package";
import type { ClientOf } from "@destack/object/client";
import type { ObjectReference } from "@destack/sync";

async (
    client: ClientOf<typeof messagePackage>,
    scope: string,
    secret: ObjectReference,
    requestId: string,
) => {
    await client.endpoint.create({
        scope,
        requestId,
        url: "https://hooks.example.com/destack",
        events: ["member.create"],
        format: "event",
        authentication: { kind: "signature" },
        secret,
    });
};
```

### Test sending

`TestMessage` serves a space with message installed, its mail bound to a test transport and its webhook and push receivers in process.

```ts
import { TestMessage } from "@destack/message/test";

const kit = await TestMessage.open({ mails: ["sent"], hooks: [200] });
```

## Comparison

### Novu

Channels, providers and per-recipient messages follow Novu; three things differ.

- Content is sealed to the installation's key by the producer, so only the sending installation reads it.
- Each message refers to its source object, so access to the message follows access to the source.
- A desktop client is a separate channel, delivered at once and marked delivered by the client.
