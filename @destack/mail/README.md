Compose and submit mail in Destack.

## Usage

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

| Security | Port | Session |
|---|---|---|
| `tls` | 465 | TLS from the first byte |
| `starttls` | 587 | a STARTTLS upgrade before authentication |
| `none` | any | plain text, to loopback hosts only |

XOAUTH2 asks its token supplier once per session.

```ts
const authentication = {
    kind: "xoauth2",
    username: "notices@destack.app",
    token: () => tokens.current(),
};
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
