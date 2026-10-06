# @destack/mail

Compose mail and send it over SMTP or Amazon SES.

## Provider

`mailProvider` sends the email messages of the `message` kind through a `MailTransport`.

```ts
import { implementMessages } from "@destack/message/server";
import { PrintTransport } from "@destack/mail/print";
import { mailProvider } from "@destack/mail/provider";

const messages = implementMessages({
    providers: [mailProvider(new PrintTransport((text) => process.stdout.write(text)))],
    database,
    callKey,
    cell,
    spaces,
});
```

## Messages

`MimeMessage.compose` builds an RFC 5322 message.

```ts
import { MimeMessage } from "@destack/mail/mime";

const message = await MimeMessage.compose({
    from: "Destack <notices@destack.app>",
    to: ["ada@example.com"],
    subject: "New sign-in",
    date: new Date(),
    key: "security-notice-01",
    text: "A new device signed in.",
    html: "<p>A new device signed in.</p>",
});
```

## SMTP

`SmtpClient.submit` sends a message to the envelope's recipients.

```ts
import { SmtpClient } from "@destack/mail/smtp";

const client = new SmtpClient({
    host: "smtp.example.com",
    port: 587,
    security: "starttls",
    authentication: { kind: "plain", username: "notices", password },
    helo: "host.destack.app",
});
const submission = await client.submit(
    { sender: "notices@destack.app", recipients: ["ada@example.com", "audit@destack.app"] },
    message,
);
// { recipients: [{ address: "ada@example.com", reply: { code: 250, enhancedCode: "2.1.5", text: "ok" } }, ...], data: { code: 250, ... } }
```

### Security

`security` sets how the session protects its bytes.

```ts
new SmtpClient({ host, port: 465, security: "tls", authentication, helo }); // TLS from the first byte
new SmtpClient({ host, port: 587, security: "starttls", authentication, helo }); // STARTTLS before authentication
new SmtpClient({ host: "127.0.0.1", port, security: "none", authentication, helo }); // plain text
```

### Authentication

`authentication` signs in with `plain` credentials or with `xoauth2`.

```ts
const authentication = {
    kind: "xoauth2",
    username: "notices@destack.app",
    token: () => tokens.current(),
};
```

## SES

`SesClient.send` sends a composed message through the SESv2 HTTP API.

```ts
import { SesClient } from "@destack/mail/aws";

const client = new SesClient({
    region: "eu-central-1",
    credentials: async () => ({ accessKeyId, secretAccessKey, sessionToken }),
    configurationSet: "destack",
});
const messageId = await client.send(
    { sender: "notices@destack.app", recipients: ["ada@example.com"] },
    message,
);
```

### Transport

`SesTransport` sends each email from one sender through a `SesClient`.

```ts
import { SesTransport } from "@destack/mail/aws";

const transport = new SesTransport(client, { from: "Destack <notices@destack.app>" });
```

### Signing

`awsSigner` opens the Signature Version 4 signer of `@smithy/signature-v4` for an AWS service over WebCrypto (`@aws-crypto/sha256-browser`).

```ts
import { awsSigner } from "@destack/mail/aws";

const signer = awsSigner({
    service: "ses",
    region: "eu-central-1",
    credentials: async () => credentials,
});
const signed = await signer.sign({
    method: "POST",
    protocol: "https:",
    hostname,
    path,
    query: {},
    headers,
    body,
});
await fetch(url, { method: "POST", headers: signed.headers, body }); // authorization, x-amz-date, x-amz-security-token
```

## Errors

`MimeError`, `SmtpError` and `SesError` have a stable `code`.

```ts
try {
    await client.send(envelope, message);
} catch (error) {
    if (error instanceof SesError && error.isRetryable) {
        await retryLater(message);
    }
}
```

## Tests

`SmtpTestServer.listen` starts a scripted SMTP server on 127.0.0.1.

```ts
import { SmtpTestServer, TestCertificate } from "@destack/mail/test";

const certificate = await TestCertificate.generate();
await using server = await SmtpTestServer.listen({
    security: "starttls",
    certificate,
    plain: { username: "ada", password: "secret" },
    answers: { "RCPT TO:<eve@example.com>": "550 5.1.1 no such user" },
});
```

### Fixture

`MailFixture` is a `MailTransport` that records each email it sends.

```ts
import { MailFixture } from "@destack/mail/test";

const mail = new MailFixture();
await mail.send({ to: "ada@example.com", subject: "Hello", text: "Hi Ada", key: "hello-01" });
mail.sent; // [{ to: "ada@example.com", subject: "Hello", … }]
```
