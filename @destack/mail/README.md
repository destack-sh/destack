Compose mail and submit it over SMTP or through Amazon SES.

## Messages

`MimeMessage` composes an RFC 5322 message whose Message-ID digests the From address and the key.

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

`SmtpClient` submits the message to the envelope's recipients and returns the server's replies.

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

`security` sets how the session protects its bytes.

| Security   | Port | Session                                  |
| ---------- | ---- | ---------------------------------------- |
| `tls`      | 465  | TLS from the first byte                  |
| `starttls` | 587  | a STARTTLS upgrade before authentication |
| `none`     | any  | plain text, to loopback hosts only       |

## Authentication

`authentication` signs in with `plain` credentials or `xoauth2`, which asks its token supplier once per session.

```ts
const authentication = {
    kind: "xoauth2",
    username: "notices@destack.app",
    token: () => tokens.current(),
};
```

## SES

`SesClient` sends a composed message through the SESv2 HTTP API and returns SES's MessageId, signing with Signature Version 4 over WebCrypto in Bun, workerd and browsers.

```ts
import { SesClient } from "@destack/mail/ses";

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

The client asks its credential supplier once per send.

## SES errors

A refused send throws a `SesError` with its code, SES's AWS code in `awsCode`, its HTTP status in `status`, and `isRetryable` when the same message may pass later.

## Signing

`signRequest` signs any AWS request and returns its headers, canonical request and string to sign.

```ts
import { signRequest } from "@destack/mail/ses";

const signed = await signRequest(
    { method: "POST", url, headers: { "content-type": "application/json" }, body },
    { credentials, region: "eu-central-1", service: "ses", date: new Date() },
);
await fetch(url, { method: "POST", headers: signed.headers, body });
```

## Test server

`SmtpTestServer` answers on 127.0.0.1 as scripted and records each session's transcript and content.

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
