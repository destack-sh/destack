/** One item of the chat: your question, the agent's call, its reply, or an approval. */
export type Item =
    | { kind: "ask"; text: string }
    | {
          kind: "call";
          server: string;
          title: string;
          request: string;
          response: string;
          isFailed: boolean;
      }
    | { kind: "reply"; text: string }
    | { kind: "approval"; text: string; isApproved: boolean };

/** One turn of the chat on one app: its page in fig. 1, and what each stack makes of it. */
export type Turn = {
    row: number;
    stacked: readonly Item[];
    destacked: { before: readonly Item[]; after: readonly Item[] };
};

/** The turns the chat takes, one app each: a task, a page's history, the waitlist, and a send. */
export const turns: readonly Turn[] = [
    {
        row: 2,
        stacked: [
            { kind: "ask", text: "Is the waitlist email actually done?" },
            {
                kind: "call",
                server: "linear",
                title: "Looked up DST-16",
                request: 'get_issue("DST-16")',
                response: '{ "state": "In Review", "assignee": "agent" }',
                isFailed: false,
            },
            {
                kind: "reply",
                text: "Linear says it's in review. I can't see whether the mails went out: your mail service isn't connected.",
            },
        ],
        destacked: {
            before: [
                { kind: "ask", text: "Is the waitlist email actually done?" },
                {
                    kind: "call",
                    server: "destack",
                    title: "Counted deliveries",
                    request: "destack sends --where 'campaign = \"invite-400\"' --count",
                    response: "400 delivered · 0 bounced",
                    isFailed: false,
                },
                {
                    kind: "call",
                    server: "destack",
                    title: "Updated DST-16",
                    request: "destack task DST-16 update --status done",
                    response: "DST-16 · review → done",
                    isFailed: false,
                },
            ],
            after: [
                {
                    kind: "reply",
                    text: "Yes. All 400 invites were delivered, so I closed DST-16.",
                },
            ],
        },
    },
    {
        row: 1,
        stacked: [
            { kind: "ask", text: "Who moved the demo to Wednesday, and why?" },
            {
                kind: "call",
                server: "notion",
                title: "Read page history",
                request: 'get_page_history("Destack launch")',
                response: "tool not available",
                isFailed: true,
            },
            {
                kind: "reply",
                text: "Notion doesn't expose page history to me, so I can't tell who changed it.",
            },
        ],
        destacked: {
            before: [
                { kind: "ask", text: "Who moved the demo to Wednesday, and why?" },
                {
                    kind: "call",
                    server: "destack",
                    title: "Read the demo's history",
                    request: "GET /api/calls?object=page:launch&field=demo",
                    response: '{ "by": "agent", "at": "09:13", "cause": "message:launch/0912" }',
                    isFailed: false,
                },
            ],
            after: [
                {
                    kind: "reply",
                    text: "I did, at 09:13. You asked in #launch: “Can we move the demo to Wednesday? Thursday is the waitlist send.”",
                },
            ],
        },
    },
    {
        row: 7,
        stacked: [
            { kind: "ask", text: "Anyone on the waitlist asking about pricing?" },
            {
                kind: "call",
                server: "slack",
                title: "Searched messages",
                request: 'search_messages("pricing")',
                response: "not_allowed_token_type",
                isFailed: true,
            },
            {
                kind: "reply",
                text: "I can't search Slack with this token, and the waitlist app has no connector.",
            },
        ],
        destacked: {
            before: [
                { kind: "ask", text: "Anyone on the waitlist asking about pricing?" },
                {
                    kind: "call",
                    server: "destack",
                    title: "Searched signups",
                    request:
                        'client.query.signup.findMany({ where: eq(signup.status, "waiting"), with: { messages: { where: like(message.text, "%pricing%") } } })',
                    response: '[{ email: "maya@northwind.io", messages: 1 }]',
                    isFailed: false,
                },
            ],
            after: [
                {
                    kind: "reply",
                    text: "Maya. She asked in #launch whether early pricing is still on. Want me to draft her a reply?",
                },
            ],
        },
    },
    {
        row: 8,
        stacked: [
            { kind: "ask", text: "Invite everyone still waiting." },
            {
                kind: "call",
                server: "googlesheets",
                title: "Read the waitlist sheet",
                request: 'read_range("Waitlist!A:C")',
                response: "403: insufficient authentication scopes",
                isFailed: true,
            },
            {
                kind: "call",
                server: "gmail",
                title: "Sent the draft",
                request: "send_draft(r-58312)",
                response: "sent to 0 recipients",
                isFailed: false,
            },
            {
                kind: "reply",
                text: "I sent it, but the draft had no recipients: I couldn't read the waitlist.",
            },
        ],
        destacked: {
            before: [
                { kind: "ask", text: "Invite everyone still waiting." },
                {
                    kind: "call",
                    server: "destack",
                    title: "Sent the draft",
                    request: "POST /api/mail/drafts/draft-01k2…/send",
                    response: "403 · sending to 1,840 people needs your approval",
                    isFailed: true,
                },
                {
                    kind: "approval",
                    text: "wants to send “Destack opens Friday” to 1,840 people.",
                    isApproved: false,
                },
            ],
            after: [
                {
                    kind: "approval",
                    text: "Approved by Florian · sending to 1,840 people",
                    isApproved: true,
                },
            ],
        },
    },
];
