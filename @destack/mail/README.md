# @destack/mail

Send email over SMTP, Amazon SES, or a printer in tests.

## Overview

A package declares the mail it sends, and the host binds it to a transport.

- **Mail**: one email to one address, deduplicated by its `key`; `send` resolves once the transport accepted it, else rejects with a `MailError`.
- **Transport**: how a host delivers mail: `SmtpTransport`, `SesTransport`, `PrintTransport`, or `TestMailTransport` in tests.

```ts
import { defineMail } from "@destack/mail";
import { PrintTransport } from "@destack/mail/print";

export const outbound = defineMail({ name: "outbound" });
const transport = new PrintTransport((text) => console.log(text));
await transport.send({
    to: "ada@example.com",
    subject: "Welcome",
    text: "Hello Ada",
    key: "welcome-ada",
    date: Date.UTC(2026, 10, 12),
});
```

## Examples

### Send from a package's server

A server reads its declared mail from its installation's resources.

```ts
import { defineMail } from "@destack/mail";
import { defineMount, definePackage } from "@destack/object";
import { implementPackage } from "@destack/object/server";

const outbound = defineMail({ name: "outbound" });
const contact = defineMount({ name: "contact", path: "/contact" });
const site = definePackage({ resources: [outbound], mounts: [contact] });

export default implementPackage(site, (context) => {
    const mail = context.resources.get(outbound);

    return {
        mounts: {
            contact: async (request) => {
                const text = await request.text();
                const key = crypto.randomUUID();
                const to = "hello@example.com";
                await mail.send({ to, subject: "Contact", text, key, date: Date.now() });

                return new Response(null, { status: 204 });
            },
        },
    };
});
```

### Send over SMTP

`SmtpTransport` submits each email in a session of its own, a 4xx reply rejecting as `COMMAND_DEFERRED` and a 5xx as `COMMAND_REJECTED`.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { SmtpClient, SmtpTransport } from "@destack/mail/smtp";

const client = new SmtpClient({
    host: "smtp.example.com",
    port: 587,
    secureTransport: "starttls",
    authentication: { kind: "plain", username: "notices", password: "secret" },
    helo: "destack.app",
    environment: BunEnvironment.system,
});
const smtp = new SmtpTransport(client, { from: "Destack <notices@destack.app>" });
```

### Test code that sends mail

`TestMailTransport` keeps what it sent and answers each email with the next scripted answer.

```ts
import { MailError } from "@destack/mail";
import { TestMailTransport } from "@destack/mail/test";

const unreachable = MailError.of("PROVIDER_UNREACHABLE", "provider is unreachable");
const transport = new TestMailTransport(["sent", unreachable]);
await transport.send({ to: "ada@example.com", subject: "Hi", text: "Hi", key: "1", date: 0 });
transport.sent.length; // 1
```

## Comparison

### Nodemailer

Transports follow Nodemailer's: one `send` over SMTP, SES or a stream; Destack's differ in two ways.

- A package never sees a transport's credentials: the host binds the declared mail to a transport.
- Every email has a `key`, which its `Message-ID` digests, so a retried send is the same email.
