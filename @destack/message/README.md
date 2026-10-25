# @destack/message

`@destack/message` sends messages to one recipient on one channel, email, push, webhook or a desktop, and delivers webhook endpoints.

```ts
const email = emailProvider(new SmtpTransport(client, { from: "Destack <notices@destack.app>" }));
const push = pushProvider(() => Vapid.derive(keys, "https://destack.app", crypto), fetch);
await MimeMessage.compose({ from, to, subject, date: new Date(clock.now()), key, text, html });
await StandardWebhook.sign(secret, { id, timestamp, body }); // from @destack/object
```

## Messages

A `message` sends as it is created: sealed content through its channel's provider, or a desktop message shown at once, which its client marks delivered with `receive`.

```ts
await client.message.create({
    scope: spaceId,
    requestId,
    to: { channel: "email", address: "dana@example.com" },
    ...ObjectReference.columns("source", issue.reference(spaceId, issueId)),
    ciphertext: await MessageKey.of(messageKey, environment).seal(content, spaceId),
});
await client.message.create({
    scope: spaceId,
    requestId,
    to: { channel: "desktop", client: laptop },
    ...source,
});
await laptopClient.message.receive({ scope, id });
const { key } = await client.message.serverKey({ scope: spaceId }); // the VAPID key browsers subscribe with
const { key: messageKey } = await client.message.contentKey({ scope: spaceId }); // the key content is sealed to
```

## Endpoints

An `endpoint` receives the scope's calls as signed webhook messages, and the message installation consumes its vault secret.

```ts
await client.endpoint.create({
    scope,
    requestId,
    url,
    events: ["member.create"],
    format: "event",
    authentication,
    secret,
});
```

## Server

`@destack/message/server` serves messages and webhook endpoints from the installation's context: its keys, its bound `outbound` mail and its network.

```ts
import server from "@destack/message/server";

const extension = server.definition(context); // mail through the bound outboundMail, push signed with its VAPID key
```

## Providers

A `MessageProvider` sends the messages of one channel; a refusal carries a `MessageReason`, with the provider's own code apart.

```ts
Outcome.read(new Response(null, { status: 410 }), now);
// { kind: "failed", refusal: { reason: "RECIPIENT_GONE", code: "410", message: "" } }
Outcome.refuse({ reason: "PROVIDER_THROTTLED", message: "slow down" }, 30_000);
// { kind: "deferred", after: 30000, refusal: { reason: "PROVIDER_THROTTLED", message: "slow down" } }
await Outcome.post(fetch, request, clock); // a lost or timed-out post defers as PROVIDER_UNREACHABLE or PROVIDER_TIMEOUT
```

## Tables

`messageTables` lists the message package's tables.

```ts
import { messageTables } from "@destack/message/stack";

const database = defineDatabase({ name: "main", tables: messageTables });
```

## Tests

`TestMessage` serves a space with message installed in a simulation, its mail bound to a `TestMailTransport` and its webhook endpoint and push service served as `TestReceiver`s; `TestMailServer` is a scripted SMTP server listening on a simulated network's loopback.

```ts
await using server = await TestMailServer.listen(simulation.network.of("mail"), {
    secureTransport: "starttls",
    answers: { "RCPT TO:<eve@example.com>": "550 5.1.1 no such user" },
});
const kit = await TestMessage.open({ mails: [{ kind: "sent" }], hooks: [500] });
```
