# @destack/message

`MailTransport` is Nodemailer's transport, with `SmtpTransport`, `SesTransport` over the SESv2 API and `PrintTransport` as its SMTP, SES and stream transports, `pushProvider` is the `web-push` library's `sendNotification` with RFC 8291 encryption and RFC 8292 VAPID, and an `endpoint` is a Svix endpoint delivering Standard Webhooks.

```ts
const email = emailProvider(new SmtpTransport(client, { from: "Destack <notices@destack.app>" })); // nodemailer.createTransport
await MimeMessage.compose({ from, to, subject, date: new Date(), key, text, html }); // Nodemailer's MailComposer
const push = pushProvider(async (scope) => Vapid.derive(await deriver(scope), "https://destack.app")); // web-push's setVapidDetails
await client.endpoint.create({ scope, requestId, url, events: ["member.create"], format: "event", authentication, secret }); // a Svix endpoint
await WebhookSignature.sign(secret, id, timestamp, body); // Standard Webhooks' "v1,…"
```

## Messages

`message.create` requests one pending message to one recipient, its content sealed by the producer to the space's message key.

```ts
const { key } = await messages.key.read({ scope: spaceId });
await client.message.create({
    scope: spaceId,
    requestId: await RequestId.derive(regressedAt, `${issueId} regressed`),
    to: { channel: "email", address: "dana@example.com" },
    source: issue.reference(spaceId, issueId), // the object it delivers for
    status: "pending", // until its provider sends it or refuses it for good
    ciphertext: await MessageKey.of(key).seal({ channel: "email", subject: "Regressed", text: "title is too long" }, spaceId),
});
```

## Service

`implementMessages` serves a space's messages, attempts and endpoints, and sends each message through its channel's provider.

```ts
import { implementMessages } from "@destack/message/server";

const messages = implementMessages({
    providers: { email: emailProvider(transport), push: pushProvider(vapid), webhook: webhookProvider(open, fetch) },
    deriver,
    events: history.store,
});
```

## Providers

A `MessageProvider` sends the messages of one channel, and `Outcome.read` reads an HTTP answer as an outcome.

```ts
Outcome.read(new Response(null, { status: 429, headers: { "Retry-After": "30" } }), Date.now()); // { kind: "retry", after: 30_000, error }
Outcome.read(new Response(null, { status: 410 }), Date.now()); // { kind: "failed", error: { code: GONE } }
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
await using server = await SmtpTestServer.listen({ security: "starttls", certificate, answers: { "RCPT TO:<eve@example.com>": "550 5.1.1 no such user" } });
const mail = new MailFixture();
mail.sent; // [{ to: "ada@example.com", subject: "Hello", … }]
```
