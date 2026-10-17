/** One item of the chat: your question, a call, a reply, an approval, or the run's tally. */
export type Item =
    | { kind: "ask"; text: string }
    | {
          kind: "call";
          server: string;
          title: string;
          request: string;
          response: string;
          isFailed: boolean;
          isExpanded: boolean;
      }
    | { kind: "reply"; text: string }
    | { kind: "approval"; text: string; isApproved: boolean }
    | { kind: "tally"; text: string; isWrong: boolean };

/** One step of the task: what the chat adds, the page it touches, and whether that changed. */
export type Step = { item: Item; row: number; isChanged: boolean };

/** The row of Chat in fig. 1, where the question was asked. */
const CHAT_ROW = 3;

/** The row of Mail in fig. 1, which sends the invites. */
const MAIL_ROW = 7;

/** The row of the waitlist in fig. 1, which keeps the signups. */
const WAITLIST_ROW = 8;

/** The task, which needs one person across three apps and two writes that agree. */
const ask: Item = {
    kind: "ask",
    text: "Invite everyone who asked about pricing in #launch, is still waiting, and didn't open our last mail. Mark them invited.",
};

/** The task through three working connectors: each call works, and the answer is still wrong. */
export const stacked: readonly Step[] = [
    { item: ask, row: CHAT_ROW, isChanged: false },
    {
        item: {
            kind: "call",
            server: "slack",
            title: "Searched #launch for pricing",
            request: 'search.messages({ query: "pricing in:#launch" })',
            response: "7 matches · from 7 user IDs",
            isFailed: false,
            isExpanded: false,
        },
        row: CHAT_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "call",
            server: "slack",
            title: "Read 7 Slack profiles",
            request: "users.info({ user }) × 7 → profile.email",
            response: "7 emails · 3 match a signup, 4 signed up with another address",
            isFailed: false,
            isExpanded: false,
        },
        row: CHAT_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "call",
            server: "supabase",
            title: "Queried the waitlist",
            request: "execute_sql(\"select email from signups where status = 'waiting'\")",
            response: "1,840 rows",
            isFailed: false,
            isExpanded: false,
        },
        row: WAITLIST_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "call",
            server: "resend",
            title: "Read the last mail's opens",
            request: "GET /emails/{id} × 400 → last_event",
            response: "429 rate_limit_exceeded at 2 a second · 212 of 400 read",
            isFailed: false,
            isExpanded: false,
        },
        row: MAIL_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "call",
            server: "resend",
            title: "Sent 3 invites",
            request: "POST /emails/batch · 3 emails",
            response: "3 ids",
            isFailed: false,
            isExpanded: false,
        },
        row: MAIL_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "call",
            server: "supabase",
            title: "Marked them invited",
            request: "execute_sql(\"update signups set status = 'invited' where email in (…)\")",
            response: "ERROR: cannot execute UPDATE in a read-only transaction",
            isFailed: true,
            isExpanded: false,
        },
        row: WAITLIST_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "reply",
            text: "I invited 3 of the 7: the other 4 signed up with an address their Slack profile doesn't show, and I only got through 212 of the 400 opens. The waitlist still has the 3 as waiting, because the Supabase connector is read-only.",
        },
        row: WAITLIST_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "tally",
            text: "411 API calls · 3 min 30 s · 3 of 7 · out of sync",
            isWrong: true,
        },
        row: WAITLIST_ROW,
        isChanged: false,
    },
];

/** The task in your space: one query joined through shared accounts, one approval, one run. */
export const destacked: readonly Step[] = [
    { item: ask, row: CHAT_ROW, isChanged: false },
    {
        item: {
            kind: "call",
            server: "destack",
            title: "Queried your space",
            request:
                'client.query.signup.findMany({\n  where: { status: "waiting" },\n  with: { account: { with: {\n    messages: { where: { channel: "launch", text: { like: "%pricing%" } } },\n    opens: { where: { mail: "invite-400" } },\n  } } },\n})',
            response: "7 signups with a pricing message and no open · 38 ms",
            isFailed: false,
            isExpanded: true,
        },
        row: WAITLIST_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "call",
            server: "destack",
            title: "Started invite-7",
            request: "invite.start({ signups })",
            response: "APPROVAL_REQUIRED · your grant asks before mail goes out",
            isFailed: false,
            isExpanded: false,
        },
        row: WAITLIST_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "approval",
            text: "wants to send “Destack opens Friday” to 7 people and mark them invited.",
            isApproved: false,
        },
        row: WAITLIST_ROW,
        isChanged: false,
    },
    {
        item: { kind: "approval", text: "Approved by Florian · 7 people", isApproved: true },
        row: WAITLIST_ROW,
        isChanged: false,
    },
    {
        item: {
            kind: "call",
            server: "destack",
            title: "Ran invite-7",
            request:
                'step.call("send", message.call("create", …)) × 7\nstep.call("mark", signup.call("invite", …)) × 7',
            response: "7 sent · 7 marked · each step recorded once",
            isFailed: false,
            isExpanded: false,
        },
        row: WAITLIST_ROW,
        isChanged: true,
    },
    {
        item: { kind: "reply", text: "Done. All 7 are invited and marked as invited." },
        row: WAITLIST_ROW,
        isChanged: true,
    },
    {
        item: {
            kind: "tally",
            text: "1 query · 1 approval · 1 run · 4 s · 7 of 7",
            isWrong: false,
        },
        row: WAITLIST_ROW,
        isChanged: true,
    },
];
