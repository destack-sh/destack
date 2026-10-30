import { principal } from "@destack/access";
import { client } from "@destack/account/object";
import { journal, JournalEntry } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { schema } from "@destack/schema";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { emailProvider } from "../email/index.ts";
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

/** A throttled send, which SES answers with its AWS code. */
const THROTTLED = {
    kind: "deferred",
    refusal: { reason: "PROVIDER_THROTTLED", code: "Throttling", message: "slow down" },
} as const;

/** A refusal for good of an unverified address. */
const REJECTED = {
    kind: "failed",
    refusal: { reason: "MESSAGE_REJECTED", code: "MessageRejected", message: "unverified" },
} as const;

/** A server sending email alone, as a machine without push or webhook providers serves messages. */
const EMAIL_ONLY = implementPackage(messagePackage, (context) =>
    serveMessages({
        providers: { email: emailProvider(context.resources.get(outboundMail)) },
        keys: context.keys,
    }),
);

/** List each journaled call of messages and attempts with the component making it, sorted. */
async function listCalls(fixture: TestMessage) {
    const rows = await fixture.served.database.select().from(journal);

    // sort by method and component, as calls in one millisecond have no order the journal records
    return rows
        .map(({ call: recorded }) => JournalEntry.parse(recorded))
        .filter((entry) => /^(message|attempt)\./u.test(entry.method))
        .map((entry) => [entry.method, entry.execution.context.component] as const)
        .toSorted((left, right) =>
            `${left[0]} ${left[1]}`.localeCompare(`${right[0]} ${right[1]}`),
        );
}

/** List the message tables whose rows have the content's text or ciphertext. */
async function listLeaks(fixture: TestMessage): Promise<string[]> {
    const leaking: string[] = [];
    const tables = { message: message.table, attempt: messageAttempt.table };
    for (const [name, table] of Object.entries(tables)) {
        const text = JSON.stringify(await fixture.served.database.select().from(table));
        if (text.includes(TEXT)) {
            leaking.push(`${name}: text`);
        }
        if (COMPACT_JWE.test(text)) {
            leaking.push(`${name}: ciphertext`);
        }
    }

    return leaking;
}

test("send a message through its channel's provider, retrying a temporary refusal, by its message controller", async () => {
    const fixture = await TestMessage.open({ mails: [THROTTLED, { kind: "sent" }] });
    const id = await fixture.send(EMAIL, CONTENT);

    // refuse it once, then send it on its next attempt
    await fixture.simulation.idle();
    const retrying = await fixture.sending(id);
    await fixture.retry(id);
    expect({
        retrying,
        sent: await fixture.sending(id),
        texts: fixture.mails.sent.map((mail) => mail.text),
        calls: await listCalls(fixture),
    }).toEqual({
        retrying: {
            status: "pending",
            reason: null,
            isErased: false,
            attempts: ["deferred"],
            sent: ["unknown", "Retrying"],
        },
        sent: {
            status: "sent",
            reason: null,
            isErased: true,
            attempts: ["deferred", "sent"],
            sent: ["true", "Sent"],
        },
        texts: [TEXT, TEXT],
        calls: [
            ["attempt.create", "message"],
            ["attempt.create", "message"],
            ["message.create", undefined],
            ["message.observe", "message"],
            ["message.observe", "message"],
        ],
    });
});

test("fail a message its provider refuses for good, or that the retries exhaust", async () => {
    // refuse one email for good, and the other eight times for now
    const fixture = await TestMessage.open({
        mails: [REJECTED, ...Array.from({ length: 8 }, () => THROTTLED)],
    });
    const refused = await fixture.send(EMAIL, CONTENT);
    await fixture.simulation.idle();
    const exhausted = await fixture.send(EMAIL, CONTENT);
    await fixture.simulation.idle();
    for (let attempt = 1; attempt < 8; attempt++) {
        await fixture.retry(exhausted);
    }

    // fail at once on the refusal for good, and after the policy's attempts on the temporary ones
    expect([await fixture.sending(refused), await fixture.sending(exhausted)]).toEqual([
        {
            status: "failed",
            reason: "MESSAGE_REJECTED",
            isErased: true,
            attempts: ["failed"],
            sent: ["false", "Failed"],
        },
        {
            status: "failed",
            reason: "PROVIDER_THROTTLED",
            isErased: true,
            attempts: Array.from({ length: 8 }, () => "deferred"),
            sent: ["false", "Failed"],
        },
    ]);
});

test("forget a browser its push service forgot, as the space, once its push fails as gone", async () => {
    // push to a browser the push service answers as gone
    const fixture = await TestMessage.open({ pushes: [410] });
    const { to } = await fixture.browser("/subscriptions/1");
    const id = await fixture.send(to, { channel: "push", data: {}, urgency: "normal", ttl: 60 });
    await fixture.simulation.idle();

    // fail it as gone, and start the browser's unsubscription as the space
    const started = await fixture.served.database.select().from(task.run.table);
    expect({
        sending: await fixture.sending(id),
        started: started.map((row) => ({
            input: row["input"],
            actor: ObjectReference.at("actor", row),
        })),
    }).toEqual({
        sending: {
            status: "failed",
            reason: "RECIPIENT_GONE",
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

test("open a message's content only for its provider, and drop it from every message row as it settles", async () => {
    // send one email and fail another for good
    const fixture = await TestMessage.open({ mails: [{ kind: "sent" }, REJECTED] });
    const sent = await fixture.send(EMAIL, CONTENT);
    const failed = await fixture.send(EMAIL, CONTENT);
    await fixture.simulation.idle();

    // hand the provider the text, while no row keeps it or its ciphertext
    expect({
        sendings: [await fixture.sending(sent), await fixture.sending(failed)].map((each) => [
            each.status,
            each.isErased,
        ]),
        texts: fixture.mails.sent.map((mail) => mail.text),
        leaking: await listLeaks(fixture),
    }).toEqual({
        sendings: [
            ["sent", true],
            ["failed", true],
        ],
        texts: [TEXT, TEXT],
        leaking: [],
    });
});

test("send a provider's message as it is created, and show a desktop one at once until its own client receives it", async () => {
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

    // refuse a receipt off a desktop or on another client, a push without its content, and a push to a client receiving none
    const { to } = await fixture.browser("/1");
    const refused = [
        await refusal(receive(email)),
        await refusal(receive(foreign)),
        await refusal(fixture.send(to)),
        await refusal(fixture.send({ ...to, client: CLIENT })),
    ];
    const rows = await fixture.served.database
        .select({
            id: message.table.id,
            status: message.table.status,
            ciphertext: message.table.ciphertext,
        })
        .from(message.table);
    const row = (id: string) => rows.find((each) => each.id === id);
    expect({
        refused,
        rows: [row(email), row(desktop)].map((each) => [each?.status, each?.ciphertext !== null]),
    }).toEqual({
        refused: [
            [
                "CONFLICT",
                "DESKTOP_MESSAGE_REQUIRED",
                `message ${email} is not a sent desktop message`,
            ],
            ["FORBIDDEN", "CLIENT_FOREIGN", `message ${foreign} shows on another client`],
            [
                "BAD_REQUEST",
                "SEALED_CONTENT_MISSING",
                "a message on push is missing its encrypted content",
            ],
            ["NOT_FOUND", "RECIPIENT_GONE", `client ${CLIENT} receives no pushes`],
        ],
        rows: [
            ["sent", false],
            ["delivered", false],
        ],
    });
});

test("sign pushes with the key browsers subscribe with, and refuse pushes on a machine sending none", async () => {
    // push once on a machine with a push provider
    const fixture = await TestMessage.open();
    const { key } = await fixture.owner
        .client(messagePackage)
        .message.serverKey({ scope: fixture.space.id });
    const { to } = await fixture.browser("/subscriptions/1");
    await fixture.send(to, { channel: "push", data: {}, urgency: "normal", ttl: 60 });
    await fixture.simulation.idle();
    const authorization = fixture.pushes.requests.map((request) =>
        request.headers.get("authorization")?.split(", k=").at(1),
    );

    // refuse the key and a push on a machine sending email alone
    const silent = await TestMessage.open({ server: EMAIL_ONLY });
    const refused = [
        await refusal(
            silent.owner.client(messagePackage).message.serverKey({ scope: fixture.space.id }),
        ),
        await refusal(silent.send(to, { channel: "push", data: {}, urgency: "normal", ttl: 60 })),
    ];
    expect({ authorization, refused }).toEqual({
        authorization: [key],
        refused: [
            ["NOT_IMPLEMENTED", "CHANNEL_UNSERVED", "this machine does not send push messages"],
            ["NOT_IMPLEMENTED", "CHANNEL_UNSERVED", "this machine does not send push messages"],
        ],
    });
});
