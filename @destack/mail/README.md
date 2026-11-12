# @destack/mail

Send email over SMTP or Amazon SES, or print it on machines without a provider.

## Overview

`@destack/mail` is a mail resource with transports after Nodemailer's: declared mail, SMTP, Amazon SES and a printer.

Your package declares its mail and sends emails:

- `defineMail`: declares a mail resource.
- `Mail`: one email to one address.
- `MailTransport`: sends one email, or rejects with a `MailError`.

Hosts send through a transport:

- `SmtpTransport` and `SmtpClient`: send over SMTP.
- `SesTransport` and `SesClient`: send through Amazon SES.
- `PrintTransport`: prints each email instead of sending it.
- `MailSender`: a deployment's mail setting: SES or printing.

Failures and tests:

- `MailError`: the reasons a send fails, checked with `MailError.is`.
- `TestMailTransport`: keeps sent emails, and answers with scripted outcomes.

Declare the mail, then send an email from your package's server:

```ts
import { defineMail } from "@destack/mail";
import { defineMount, definePackage } from "@destack/object";
import { implementPackage } from "@destack/object/server";

// declare the mail and a contact form's mount
const outbound = defineMail({ name: "outbound" });
const contact = defineMount({ name: "contact", path: "/contact" });
const site = definePackage({ resources: [outbound], mounts: [contact] });

export default implementPackage(site, (context) => {
    // read the bound transport
    const mail = context.resources.get(outbound);

    return {
        mounts: {
            contact: async (request) => {
                // send the form's text as one email
                const text = await request.text();
                const key = crypto.randomUUID();
                await mail.send({
                    to: "hello@example.com",
                    subject: "Contact",
                    text,
                    key,
                    date: Date.now(),
                });

                return new Response(null, { status: 204 });
            },
        },
    };
});
```

## Reference

### Mail

A `Mail` is one email to one address, and its schema checks each field.
`Mail.compose(mail, from, crypto)` writes it as a `MimeMessage`, with the HTML as an alternative to the text.

| Field     | What it is                                                           |
| --------- | -------------------------------------------------------------------- |
| `to`      | the address                                                          |
| `subject` | the subject, at most 998 characters                                  |
| `text`    | the plain text body, at most 64 KiB                                  |
| `html`    | the HTML body, absent for plain text only                            |
| `key`     | the key the `Message-ID` digests, so an email sent again is the same |
| `date`    | the time in the `Date` header, in milliseconds since the epoch       |

### Transports

Every transport has `send(mail)`, which rejects with a `MailError`.
`from` is a mailbox, such as `Destack <notices@destack.app>`, and a malformed one throws.

| Transport                             | Entry                 | What it does                                                                                     |
| ------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------ |
| `new SmtpTransport(client, { from })` | `@destack/mail/smtp`  | submits each email through an `SmtpClient`                                                       |
| `new SesTransport(client, { from })`  | `@destack/mail/ses`   | sends each email through an `SesClient`                                                          |
| `new PrintTransport(print)`           | `@destack/mail/print` | writes each email's address, subject and text                                                    |
| `new TestMailTransport(answers)`      | `@destack/mail/test`  | keeps each email in `sent`, answering each with the next of `answers`: `"sent"` or a `MailError` |

### SmtpClient

`new SmtpClient(options)` creates a client with RFC 5321's step timeouts.
`submit(envelope, message)` sends one message in its own session, and answers the server's replies.

| Option            | What it is                                                                                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `host`, `port`    | the server, such as port 465 for implicit TLS or 587 for STARTTLS                                                                                      |
| `secureTransport` | `"on"` for TLS at once, `"starttls"`, or `"off"` for plain text on loopback only                                                                       |
| `authentication`  | `{ kind: "plain", username, password }`, `{ kind: "xoauth2", username, token }` with a `token` function each session calls once, or `{ kind: "none" }` |
| `helo`            | the client's host name or address literal, which `EHLO` sends                                                                                          |
| `environment`     | the network the client dials and the clock its timeouts run on                                                                                         |

### SesClient

`new SesClient(options)` creates a client that signs with AWS Signature Version 4.
`send(envelope, message, now)` answers SES's `MessageId`, and a send times out after fifteen seconds.

| Option             | What it is                                                                   |
| ------------------ | ---------------------------------------------------------------------------- |
| `region`           | the AWS region, such as `eu-central-1`                                       |
| `endpoint`         | the SES origin, `SesClient.endpoint(region)` or a local inbox in development |
| `credentials`      | a function answering the AWS credentials, called once per send               |
| `configurationSet` | the configuration set whose event destinations and suppression apply         |
| `environment`      | the clock, WebCrypto and network it sends with                               |

### MailSender

`MailSender.transport(sender, options)` opens a sender's transport.
`mailConnector(code, transport)` binds declared mail to a transport.

| Sender                          | Transport                                               |
| ------------------------------- | ------------------------------------------------------- |
| `{ kind: "ses", region, from }` | an `SesTransport`, signed with the `credentials` option |
| `{ kind: "print" }`             | a `PrintTransport`, writing to the `print` option       |

## Examples

### Send over SMTP

`SmtpTransport` sends through an `SmtpClient`.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { SmtpClient, SmtpTransport } from "@destack/mail/smtp";

// submit to the server with STARTTLS and PLAIN
const client = new SmtpClient({
    host: "smtp.example.com",
    port: 587,
    secureTransport: "starttls",
    authentication: { kind: "plain", username: "notices", password: "secret" },
    helo: "destack.app",
    environment: BunEnvironment.system,
});

// send one email from the notices mailbox
const smtp = new SmtpTransport(client, { from: "Destack <notices@destack.app>" });
await smtp.send({
    to: "ada@example.com",
    subject: "Welcome",
    text: "Hello Ada",
    key: "welcome-ada",
    date: Date.UTC(2026, 10, 12),
});
```

### Open a deployment's transport

`MailSender.transport` opens SES or a printer, as the setting says.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import type { AwsCredentials } from "@destack/identity/aws";
import type { MailTransport } from "@destack/mail";
import { MailSender } from "@destack/mail/sender";

/** Open the transport a deployment's mail setting describes. */
export function openTransport(
    setting: string,
    credentials: () => Promise<AwsCredentials>,
): MailTransport {
    // read the sender from the setting's JSON
    const sender = MailSender.parse(JSON.parse(setting));

    // open SES with the credentials, or print to the log
    return MailSender.transport(sender, {
        credentials,
        print: (text) => console.log(text),
        environment: BunEnvironment.system,
    });
}
```

### Test code that sends mail

`TestMailTransport` answers each send with the next scripted outcome.

```ts
import { MailError } from "@destack/mail";
import { TestMailTransport } from "@destack/mail/test";
import { expect, test } from "@destack/test";

test("send the welcome email once the provider answers again", async () => {
    // refuse the first email, and send the second
    const unreachable = MailError.of("PROVIDER_UNREACHABLE", "provider is unreachable");
    const transport = new TestMailTransport([unreachable, "sent"]);
    const mail = { to: "ada@example.com", subject: "Hi", text: "Hi", key: "1", date: 0 };

    // send twice, the first send failing
    await expect(transport.send(mail)).rejects.toBe(unreachable);
    await transport.send(mail);

    expect(transport.sent).toEqual([mail, mail]);
});
```

## Comparison

### Nodemailer

Transports basically follow Nodemailer's: one `send` over SMTP, SES or a printer.
Packages never see credentials, and an email's `key` keeps its `Message-ID` stable.

| Nodemailer                               | `@destack/mail`                       | Difference                                                   |
| ---------------------------------------- | ------------------------------------- | ------------------------------------------------------------ |
| `createTransport(options)`               | `new SmtpTransport(client, { from })` | The sender is fixed per transport.                           |
| `transporter.sendMail(message)`          | `transport.send(mail)`                | Sends to one address, with a `key` and a `date`.             |
| `messageId`                              | `key`                                 | The `Message-ID` digests the sender's address and the key.   |
| `auth: { user, pass }`, `type: "OAuth2"` | `authentication`                      | `plain` or `xoauth2`, with the token asked once per session. |
| `secure`, `requireTLS`                   | `secureTransport`                     | Plain text only to loopback.                                 |
| SES transport                            | `SesTransport`                        | Calls SES's v2 HTTP API, without the AWS SDK.                |
| `jsonTransport`, `streamTransport`       | `PrintTransport`, `TestMailTransport` | Prints the email, or keeps it and answers as scripted.       |
