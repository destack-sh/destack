import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { MailSender } from "./sender.ts";

test("print each email a printing sender sends to the runtime's log, and refuse a sender of no known kind", async () => {
    // send one email through a printing sender
    const printed: string[] = [];
    const transport = MailSender.transport(MailSender.parse({ kind: "print" }), {
        credentials: () => Promise.reject(new Error("a printing sender signs nothing")),
        print: (text) => printed.push(text),
        environment: testSimulation().environment("deployment"),
    });
    await transport.send({
        to: "ada@example.com",
        subject: "Your sign-in code",
        text: "123456",
        key: "sign-in",
        date: 0,
    });

    // print its address, subject and body, and parse SES and printing senders alone
    expect([
        printed,
        MailSender.safeParse({ kind: "ses", region: "eu-central-1", from: "a@b.c" }).success,
        MailSender.safeParse({ kind: "smtp" }).success,
    ]).toEqual([["email to ada@example.com: Your sign-in code\n\n123456\n"], true, false]);
});
