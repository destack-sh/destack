# @destack/message

Send messages by email, push or webhook through providers.

## Messages

`message.create` requests one message to one recipient.

```ts
await client.message.create({
    spaceId,
    requestId,
    id: deliveryId, // the requester's idempotency key
    to: { channel: "email", address: "dana@example.com" },
    content: { channel: "email", subject: "Regressed", text: "title is too long" },
});
```

### Attempts

Each `attempt` records one provider's try at a message, and the `Sent` condition carries the failure code once it fails.

```ts
const tries = await objects.query.attempt.findMany({ where: { parentId: id } }); // outcome: sent | retry | failed
```

## Providers

A `MessageProvider` sends the messages of one channel.

```ts
const print: MessageProvider = {
    channel: "email",
    send: async (message) => (console.log(message.content), { outcome: "sent" }),
};
```

## Service

`implementMessages` sends a cell's due messages through their providers.

```ts
const messages = implementMessages({
    database,
    callKey,
    providers: [mailProvider(transport), webhookProvider()],
    cell: hostId,
    spaces,
});
```

## Push

`pushProvider` sends push messages encrypted for each browser (RFC 8291) and signed with VAPID keys (RFC 8292).

```ts
const vapid = new Vapid(await Vapid.importKeys(jwk), "https://destack.app");
const push = pushProvider(vapid);
```

## Webhooks

`webhookProvider` posts webhook messages signed per Standard Webhooks.

```ts
const secret = generateSecret(); // "whsec_…"
await objects.mutate.message.create({
    to: { channel: "webhook", url: "https://hooks.example.com/destack" },
    content: { channel: "webhook", type: "issue.regressed", data: { issue: id } },
    secret,
});
```

## Signatures

`WebhookSignature.sign` computes the `webhook-signature` header of a message.

```ts
const signature = await WebhookSignature.sign(secret, id, timestamp, body); // "v1,…"
```
