# @destack/mail

Compose mail and submit it over SMTP or through Amazon SES.

## Messages

`MimeMessage.compose` builds an RFC 5322 message and derives its Message-ID from a digest of the From address and the `key`.

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

`SmtpClient.submit` sends a message to the envelope's recipients and returns the server's reply to each recipient and to the data.

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

## Security

`security` sets how the session protects its bytes, and `none` accepts only loopback hosts.

```ts
new SmtpClient({ host, port: 465, security: "tls", authentication, helo }); // TLS from the first byte
new SmtpClient({ host, port: 587, security: "starttls", authentication, helo }); // STARTTLS before authentication
new SmtpClient({ host: "127.0.0.1", port, security: "none", authentication, helo }); // plain text
```

## Authentication

`authentication` signs in with `plain` credentials or with `xoauth2`, which calls its `token` function once per session.

```ts
const authentication = {
    kind: "xoauth2",
    username: "notices@destack.app",
    token: () => tokens.current(),
};
```

## SES

`SesClient.send` sends a composed message through the SESv2 HTTP API with Signature Version 4, calls `credentials` once per send and returns the SES MessageId.

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

## SES errors

A refused send throws a `SesError` with its `code`, the SES `awsCode`, the HTTP `status` and `isRetryable`, which is true when the same message may pass later.

```ts
try {
    await client.send(envelope, message);
} catch (error) {
    if (error instanceof SesError && error.isRetryable) {
        await retryLater(message);
    }
}
```

## Signing

`signRequest` signs an AWS request with Signature Version 4 and returns its headers, canonical request and string to sign.

```ts
import { signRequest } from "@destack/mail/aws";

const signed = await signRequest(
    { method: "POST", url, headers: { "content-type": "application/json" }, body },
    { credentials, region: "eu-central-1", service: "ses", date: new Date() },
);
await fetch(url, { method: "POST", headers: signed.headers, body });
```

## Test server

`SmtpTestServer.listen` starts an SMTP server on 127.0.0.1 that gives the scripted `answers` and records each session's transcript and content.

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
