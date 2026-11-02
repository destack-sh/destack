# @destack/mail

`defineMail` declares a mail resource, and a `MailTransport` sends each email or rejects with a `MailError`, through SMTP by `/smtp`, Amazon SES by `/ses`, a printer by `/print` and a script by `/test`.

```ts
import { defineMail } from "@destack/mail/declare";
import { SmtpTransport } from "@destack/mail/smtp";

export const outboundMail = defineMail("outbound");
await outboundMail.get(context).send({ to, subject, text, key, date }); // resolves once sent
const smtp = new SmtpTransport(client, { from: "Destack <notices@destack.app>" }); // a 4xx reply rejects as COMMAND_DEFERRED, a 5xx as COMMAND_REJECTED
const ses = new SesTransport(new SesClient({ region, endpoint, credentials, environment }), {
    from,
});
const print = new PrintTransport((text) => process.stdout.write(text));
const mail = mailConnector("smtp", smtp); // binds declared mail to the transport
const scripted = new TestMailTransport([MailError.of("PROVIDER_TIMEOUT", "timed out"), "sent"]);
await using server = await TestMailServer.listen(network, { secureTransport: "starttls" }); // a scripted SMTP server
await MimeMessage.compose(
    { from, to, subject, date: new Date(clock.now()), key, text, html },
    crypto,
);
```
