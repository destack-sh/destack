# @destack/message

`MailTransport` is Nodemailer's transport, with `SmtpTransport`, `SesTransport` over the SESv2 API and `PrintTransport` as its SMTP, SES and stream transports, `pushProvider` is the `web-push` library's `sendNotification` with RFC 8291 encryption and RFC 8292 VAPID, and an `endpoint` is a Svix endpoint delivering Standard Webhooks.

```ts
// nodemailer.createTransport
const email = emailProvider(new SmtpTransport(client, { from: "Destack <notices@destack.app>" }));
// Nodemailer's MailComposer
await MimeMessage.compose({ from, to, subject, date: new Date(), key, text, html });
// web-push's setVapidDetails
const push = pushProvider(async (scope) =>
    Vapid.derive(await deriver(scope), "https://destack.app"),
);
// a Svix endpoint
await client.endpoint.create({
    scope,
    requestId,
    url,
    events: ["member.create"],
    format: "event",
    authentication,
    secret,
});
await WebhookSignature.sign(secret, id, timestamp, body); // Standard Webhooks' "v1,…"
```

## Messages

`message.create` requests one pending message to one recipient, its content sealed by the producer to the space's message key, and `message.serverKey` reads the VAPID key browsers subscribe to the space's pushes with.

```ts
// pushManager.subscribe's applicationServerKey
const { key: serverKey } = await client.message.serverKey({ scope: spaceId });
await client.message.create({
    scope: spaceId,
    requestId: await RequestId.derive(regressedAt, `${issueId} regressed`),
    to: { channel: "email", address: "dana@example.com" },
    source: issue.reference(spaceId, issueId), // the object it delivers for
    status: "pending", // until its provider sends it or refuses it for good
    ciphertext: await MessageKey.of(messageKey).seal(
        { channel: "email", subject: "Regressed", text: "title is too long" },
        spaceId,
    ),
});
```

## Server

`implementMessages` serves a space's messages, attempts and endpoints, sends each message through its channel's provider, and delivers each endpoint's calls from the store's own call events.

```ts
import { implementMessages } from "@destack/message/server";

const messages = implementMessages({
    providers: {
        email: emailProvider(transport),
        push: pushProvider(vapid),
        webhook: webhookProvider(open, fetch),
    },
    deriver,
});
```

## Providers

A `MessageProvider` sends the messages of one channel, and `Outcome.read` reads an HTTP answer as an outcome.

```ts
// { kind: "retry", after: 30_000, error }
Outcome.read(new Response(null, { status: 429, headers: { "Retry-After": "30" } }), Date.now());
// { kind: "failed", error: { code: GONE } }
Outcome.read(new Response(null, { status: 410 }), Date.now());
```

## Tables

`messageTables` lists the tables a space database includes.

```ts
import { messageTables } from "@destack/message/stack";

const space = defineDatabase({ name: "space", tables: [...messageTables, journal] });
```

## Tests

`SmtpTestServer.listen` starts a scripted SMTP server on 127.0.0.1, and `MailFixture` records each email it sends.

```ts
await using server = await SmtpTestServer.listen({
    security: "starttls",
    certificate,
    answers: { "RCPT TO:<eve@example.com>": "550 5.1.1 no such user" },
});
const mail = new MailFixture();
mail.sent; // [{ to: "ada@example.com", subject: "Hello", … }]
```
