# @destack/message

`@destack/message` sends messages to one recipient on one channel, email, push, webhook or a desktop, and delivers webhook endpoints.

```ts
const email = emailProvider(new SmtpTransport(client, { from: "Destack <notices@destack.app>" }));
const push = pushProvider(() => Vapid.derive(keys, "https://destack.app"));
await MimeMessage.compose({ from, to, subject, date: new Date(), key, text, html });
await WebhookSignature.sign(secret, id, timestamp, body);
```

## Messages

A `message` is created scheduled or pending; its producer sends, defers or skips a scheduled one, and a desktop marks a sent one delivered with `receive`.

```ts
await client.message.create({
    scope: spaceId,
    requestId,
    to: { channel: "email", address: "dana@example.com" },
    source: issue.reference(spaceId, issueId),
    dueAt: now,
    status: "pending",
    ciphertext: await MessageKey.of(messageKey).seal(content, spaceId),
});
message.call("defer", { scope, id, dueAt: evening, isSummarized: true });
message.call("skip", { scope, id, reason: "read" });
message.call("send", { scope, id, to: { channel: "desktop", client: laptop } });
await laptopClient.message.receive({ scope, id });
const { key } = await client.message.serverKey({ scope: spaceId });
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

const extension = server(context); // mail through the bound outboundMail, push signed with its VAPID key
```

## Providers

A `MessageProvider` sends the messages of one channel, and `Outcome.read` reads an HTTP answer as an outcome.

```ts
Outcome.read(new Response(null, { status: 429, headers: { "Retry-After": "30" } }), Date.now());
Outcome.read(new Response(null, { status: 410 }), Date.now());
```

## Tables

`messageTables` lists the message package's tables.

```ts
import { messageTables } from "@destack/message/stack";

const database = defineDatabase({ name: "main", tables: messageTables });
```

## Tests

`SmtpTestServer` is a scripted SMTP server on 127.0.0.1, and `MailFixture` records each email sent through it.

```ts
await using server = await SmtpTestServer.listen({
    security: "starttls",
    certificate,
    answers: { "RCPT TO:<eve@example.com>": "550 5.1.1 no such user" },
});
const mail = new MailFixture();
```
