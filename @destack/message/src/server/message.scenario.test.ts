import { principal } from "@destack/access";
import { client } from "@destack/account/object";
import { eq } from "@destack/db";
import { implementPackage } from "@destack/object/server";
import { schema } from "@destack/schema";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { emailProvider } from "../email/index.ts";
import { MailError } from "@destack/mail/error";
import { message, messageAttempt, task } from "../object/index.ts";
import messagePackage, { outboundMail } from "../package.ts";
import { TestMessage } from "../test/index.ts";
import { serveMessages } from "./serve.ts";

/** The text of every email, which only its provider sees. */
const TEXT = "title is too long";

/** An email to Dana, the destination of most tests. */
const EMAIL = { channel: "email", address: "dana@example.com" } as const;

/** The content of every email. */
const CONTENT = { channel: "email", subject: "Regressed", text: TEXT } as const;

/** A compact JWE inside stored JSON, as sealed content is kept. */
const COMPACT_JWE = /eyJ[\w-]+\.[\w-]*\.[\w-]+\.[\w-]+\.[\w-]+/u;

/** The desktop client showing a message. */
const CLIENT = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000c1");

/** A throttled send, which SES refuses with its AWS code. */
const THROTTLED = MailError.of("PROVIDER_EXHAUSTED", "sending rate exceeds the SES limit", {
    data: { awsCode: "Throttling" },
});

/** A refusal for good of an unverified address. */
const REJECTED = MailError.of("MESSAGE_INVALID", "sender address is invalid: it is unverified", {
    data: { awsCode: "MessageRejected" },
});

/** A server sending email alone, as a machine without push or webhook providers serves messages. */
const EMAIL_ONLY = implementPackage(messagePackage, (context) =>
    serveMessages({
        providers: { email: emailProvider(context.resources.get(outboundMail)) },
        keys: context.keys,
    }),
);

/** List the message tables whose rows have the content's text or ciphertext. */
async function listLeaks(fixture: TestMessage): Promise<string[]> {
    const leaking: string[] = [];
    const tables = { message: message.table, attempt: messageAttempt.table };
    for (const [name, table] of Object.entries(tables)) {
        const text = JSON.stringify(await fixture.space.database.select().from(table));
        if (text.includes(TEXT)) {
            leaking.push(`${name}: text`);
        }
        if (COMPACT_JWE.test(text)) {
            leaking.push(`${name}: ciphertext`);
        }
    }

    return leaking;
}

test("settle messages by their provider's outcomes", async () => {
    // defer one email then send it, refuse one for good, and defer one until the retries exhaust
    const fixture = await TestMessage.open({
        mails: [THROTTLED, "sent", REJECTED, ...Array.from({ length: 8 }, () => THROTTLED)],
    });
    const retried = await fixture.send(EMAIL, CONTENT);
    await fixture.simulation.idle();
    const retrying = await fixture.sending(retried);
    await fixture.retry(retried);

    // refuse the second for good, and give the third up after the policy's attempts
    const refused = await fixture.send(EMAIL, CONTENT);
    await fixture.simulation.idle();
    const exhausted = await fixture.send(EMAIL, CONTENT);
    await fixture.simulation.idle();
    for (let attempt = 1; attempt < 8; attempt++) {
        await fixture.retry(exhausted);
    }

    // hand the provider each try's text, and keep neither text nor ciphertext in any row
    expect({
        retrying,
        sendings: [
            await fixture.sending(retried),
            await fixture.sending(refused),
            await fixture.sending(exhausted),
        ],
        texts: fixture.mails.sent.map((mail) => mail.text),
        leaking: await listLeaks(fixture),
    }).toEqual({
        retrying: {
            status: "pending",
            reason: null,
            isErased: false,
            attempts: ["deferred"],
            sent: ["unknown", "Retrying"],
        },
        sendings: [
            {
                status: "sent",
                reason: null,
                isErased: true,
                attempts: ["deferred", "sent"],
                sent: ["true", "Sent"],
            },
            {
                status: "failed",
                reason: "MESSAGE_INVALID",
                isErased: true,
                attempts: ["failed"],
                sent: ["false", "Failed"],
            },
            {
                status: "failed",
                reason: "PROVIDER_EXHAUSTED",
                isErased: true,
                attempts: Array.from({ length: 8 }, () => "deferred"),
                sent: ["false", "Failed"],
            },
        ],
        texts: Array.from({ length: 11 }, () => TEXT),
        leaking: [],
    });
});

test("unsubscribe a browser its push service answers gone", async () => {
    // push to a browser the push service answers as gone
    const fixture = await TestMessage.open({ pushes: [410] });
    const { to } = await fixture.browser("/subscriptions/1");
    const id = await fixture.send(to, { channel: "push", data: {}, urgency: "normal", ttl: 60 });
    await fixture.simulation.idle();

    // fail it as gone, and start the browser's unsubscription as the space
    const started = await fixture.space.database.select().from(task.run.table);
    expect({
        sending: await fixture.sending(id),
        started: started.map((row) => ({
            input: row["input"],
            actor: ObjectReference.at("actor", row),
        })),
    }).toEqual({
        sending: {
            status: "failed",
            reason: "RECIPIENT_GONE_NOT_FOUND",
            isErased: true,
            attempts: ["failed"],
            sent: ["false", "Failed"],
        },
        started: [
            {
                input: {
                    call: client.call("unsubscribe", { id: to.client, userId: to.user }),
                },
                actor: principal.space.reference(Scope.universe.id, fixture.space.id),
            },
        ],
    });
});

test("show a desktop message until its client receives it", async () => {
    const fixture = await TestMessage.open();
    const messages = fixture.owner.client(messagePackage);
    const receive = (id: string) =>
        messages.message.receive({
            scope: fixture.space.id,
            id: message.identifier(id),
            requestId: RequestId.create(fixture.environment),
        });

    // send an email and show a desktop message on the owner's client, received twice alike
    const email = await fixture.send(EMAIL, CONTENT);
    const desktop = await fixture.send({
        channel: "desktop",
        client: fixture.space.owner.clientId,
    });
    const foreign = await fixture.send({ channel: "desktop", client: CLIENT });
    await receive(desktop);
    await receive(desktop);

    // refuse a receipt off a desktop or on another client, and mark the desktop one delivered
    const refused = [await refusal(receive(email)), await refusal(receive(foreign))];
    const [row] = await fixture.space.database
        .select({ status: message.table.status, ciphertext: message.table.ciphertext })
        .from(message.table)
        .where(eq(message.table.id, desktop));
    expect({ refused, desktop: row }).toEqual({
        refused: [
            [
                "PRECONDITION_FAILED",
                "MESSAGE_BLOCKED",
                `cannot receive message '${email}': it is not a sent desktop message`,
            ],
            [
                "FORBIDDEN",
                "CLIENT_DENIED",
                `message '${foreign}' is forbidden: it shows on another client`,
            ],
        ],
        desktop: { status: "delivered", ciphertext: null },
    });
});

test("refuse a push without content or a subscribed browser", async () => {
    // push to a subscribed browser without content, and to a client receiving no pushes
    const fixture = await TestMessage.open();
    const { to } = await fixture.browser("/1");
    expect([
        await refusal(fixture.send(to)),
        await refusal(fixture.send({ ...to, client: CLIENT })),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "CONTENT_REQUIRED",
            "encrypted content is required: a message on push is missing it",
        ],
        [
            "NOT_FOUND",
            "RECIPIENT_GONE_NOT_FOUND",
            `push subscription of client '${CLIENT}' not found`,
        ],
    ]);
});

test("sign pushes with the scope's server key", async () => {
    // read the server key, and push once to a subscribed browser
    const fixture = await TestMessage.open();
    const { key } = await fixture.owner
        .client(messagePackage)
        .message.serverKey({ scope: fixture.space.id });
    const { to } = await fixture.browser("/subscriptions/1");
    await fixture.send(to, { channel: "push", data: {}, urgency: "normal", ttl: 60 });
    await fixture.simulation.idle();

    // sign the push service's post with the same key
    const authorization = fixture.pushes.requests.map((request) =>
        request.headers.get("authorization")?.split(", k=").at(1),
    );
    expect(authorization).toEqual([key]);
});

test("refuse pushes on a machine without a push provider", async () => {
    // serve a machine sending email alone
    const fixture = await TestMessage.open({ server: EMAIL_ONLY });
    const messages = fixture.owner.client(messagePackage);
    const to = { channel: "push", client: CLIENT, user: fixture.space.owner.id } as const;

    // refuse the server key and a push
    const unsupported = [
        "NOT_IMPLEMENTED",
        "CHANNEL_UNSUPPORTED",
        "sending push messages on this machine is not supported",
    ];
    expect([
        await refusal(messages.message.serverKey({ scope: fixture.space.id })),
        await refusal(fixture.send(to, { channel: "push", data: {}, urgency: "normal", ttl: 60 })),
    ]).toEqual([unsupported, unsupported]);
});
