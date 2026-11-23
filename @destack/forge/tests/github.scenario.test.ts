import { AuditCaller } from "@destack/object";
import { eq } from "@destack/db";
import { found, type Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { RequestId } from "@destack/sync";
import { GitHubSignature } from "../src/github/index.ts";
import { repository } from "@destack/registry/object";
import forgePackage from "../src/package.ts";
import { type GitHubStandIn, TestForge, WEBHOOK_SECRET } from "../src/test/index.ts";
import { auditedCalls } from "@destack/object/test";

/** The fixture GitHub App's installation on the tenant's GitHub organization. */
const INSTALLATION = 4242;

/** Another account's installation of the fixture GitHub App. */
const OTHER_INSTALLATION = 5151;

/** How long GitHub's installation tokens last: an hour. */
const TOKEN_MILLISECONDS = 3_600_000;

/** Commits and tag objects of the fixture GitHub repository. */
const objects = {
    first: "aa218f56b14c9653891f9e74264a383fa43fefbd",
    second: "6dcb09b5b57875f334f61aebed695e2e4193db5e",
    third: "0d1a26e67d8f5eaf1f6ba5c57fc3c7d91ac0fd1c",
    release: "940bd336248efae0f9ee5bc7b2d5c985887b16ac",
    patch: "c3d0be41ecbe669545ee3e94d31ed9a4bc91ee3c",
};

/** GitHub's identifier of the fixture repository. */
const REPOSITORY_ID = 1296269;

/** Host example/site at GitHub: main and next, an annotated v1 and a lightweight v0. */
function site(github: GitHubStandIn) {
    github.add({
        id: REPOSITORY_ID,
        fullName: "example/site",
        installation: INSTALLATION,
        defaultBranch: "main",
        references: new Map([
            ["refs/heads/main", { type: "commit", sha: objects.first }],
            ["refs/heads/next", { type: "commit", sha: objects.second }],
            ["refs/tags/v0", { type: "commit", sha: objects.first }],
            ["refs/tags/v1", { type: "tag", sha: objects.release }],
        ]),
        tags: new Map([[objects.release, { type: "commit", sha: objects.first }]]),
    });
}

/** The repository and installation objects GitHub's repository events send. */
function source(installation: number) {
    return {
        repository: {
            id: REPOSITORY_ID,
            node_id: "MDEwOlJlcG9zaXRvcnkxMjk2MjY5",
            name: "site",
            full_name: "example/site",
            private: true,
            owner: { login: "example", id: 9919, type: "Organization" },
            html_url: "https://github.com/example/site",
            default_branch: "main",
            master_branch: "main",
        },
        sender: { login: "ada", id: 583231, type: "User" },
        installation: { id: installation, node_id: "MDIzOkludGVncmF0aW9uSW5zdGFsbGF0aW9uNDI0Mg==" },
    };
}

/** Sign a delivery as GitHub sends it to the forge's webhook mount. */
async function deliver(
    forge: TestForge,
    delivery: { readonly id: string; readonly event: string; readonly payload: unknown },
    secret = WEBHOOK_SECRET,
) {
    const body = JSON.stringify(delivery.payload);
    const headers = await new GitHubSignature().sign(
        { id: delivery.id, event: delivery.event, body, sentAt: forge.environment.clock.now() },
        secret,
        forge.environment.crypto,
    );
    headers.set("content-type", "application/json");

    return forge.fetch("/webhooks/github", { method: "POST", headers, body });
}

/** Connect the tenant's installation, and host example/site in it at GitHub. */
async function connected(forge: TestForge) {
    site(forge.github);
    const tenant = await forge.tenant();
    const connectionId = await forge.connect(tenant, INSTALLATION);

    return { tenant, connectionId, owner: forge.client(tenant.owner) };
}

/** Read GitHub's identifier of a repository at the forge. */
async function identifier(forge: TestForge, id: Identifier<"repository">) {
    const [row] = await forge.home.database
        .select({ providerRepositoryId: repository.table.providerRepositoryId })
        .from(repository.table)
        .where(eq(repository.table.id, id));

    return row?.providerRepositoryId;
}

test("serve a GitHub repository through its connected installation", async () => {
    const forge = await TestForge.open();
    const { tenant, connectionId, owner } = await connected(forge);

    // connect example/site, and record GitHub's identifier once the forge settles
    const audited = new Set(
        (await auditedCalls(forge.home.database)).map((call) => call.execution.id),
    );
    const created = await owner.repository.create({
        spaceId: forge.space,
        accountId: tenant.accountId,
        requestId: RequestId.create(forge.environment),
        name: "site",
        hosting: "github",
        remote: "https://github.com/example/site.git",
        connectionId,
    });
    await forge.simulation.idle();
    expect([
        created.provider,
        created.providerRepositoryId,
        await identifier(forge, created.id),
    ]).toEqual([null, null, "1296269"]);
    await owner.repository.refresh({
        spaceId: forge.space,
        id: created.id,
        requestId: RequestId.create(forge.environment),
    });
    const references = async () =>
        (
            await owner.reference.list({
                spaceId: forge.space,
                where: { parentId: created.id },
                orderBy: { name: "asc" },
            })
        ).items.map((item) => [item.name, item.object, item.commit, item.deletedAt === null]);
    expect(await references()).toEqual([
        ["refs/heads/main", objects.first, objects.first, true],
        ["refs/heads/next", objects.second, objects.second, true],
        ["refs/tags/v0", objects.first, objects.first, true],
        ["refs/tags/v1", objects.release, objects.first, true],
    ]);

    // receive a branch push, a tag creation, a branch deletion, a ping and another installation's push
    const hosted = found(forge.github.repositories, "example/site");
    hosted.references.set("refs/heads/main", { type: "commit", sha: objects.third });
    hosted.references.set("refs/tags/v2", { type: "tag", sha: objects.patch });
    hosted.tags.set(objects.patch, { type: "commit", sha: objects.third });
    hosted.references.delete("refs/heads/next");
    forge.github.requests.length = 0;
    const deliveries = [
        {
            id: "5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e01",
            event: "push",
            payload: {
                ref: "refs/heads/main",
                before: objects.first,
                after: objects.third,
                created: false,
                deleted: false,
                forced: false,
                base_ref: null,
                compare: `https://github.com/example/site/compare/${objects.first.slice(0, 12)}...${objects.third.slice(0, 12)}`,
                commits: [{ id: objects.third, message: "write README.md", distinct: true }],
                head_commit: { id: objects.third, message: "write README.md", distinct: true },
                pusher: { name: "ada", email: "ada@example.test" },
                ...source(INSTALLATION),
            },
        },
        {
            id: "5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e02",
            event: "create",
            payload: {
                ref: "v2",
                ref_type: "tag",
                master_branch: "main",
                description: null,
                pusher_type: "user",
                ...source(INSTALLATION),
            },
        },
        {
            id: "5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e03",
            event: "delete",
            payload: {
                ref: "next",
                ref_type: "branch",
                pusher_type: "user",
                ...source(INSTALLATION),
            },
        },
        {
            id: "5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e04",
            event: "ping",
            payload: {
                zen: "Keep it logically awesome.",
                hook_id: 30,
                hook: { type: "App", id: 30, events: ["create", "delete", "push"] },
                ...source(INSTALLATION),
            },
        },
        {
            id: "5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e05",
            event: "push",
            payload: {
                ref: "refs/heads/main",
                before: objects.third,
                after: objects.second,
                created: false,
                deleted: false,
                forced: true,
                base_ref: null,
                commits: [],
                head_commit: null,
                ...source(OTHER_INSTALLATION),
            },
        },
    ];
    const statuses: number[] = [];
    for (const delivery of deliveries) {
        statuses.push((await deliver(forge, delivery)).status);
    }

    // move main, add v2 at its peeled commit, delete next, and leave the other installation's push alone
    expect({ statuses, references: await references() }).toEqual({
        statuses: [202, 202, 202, 202, 202],
        references: [
            ["refs/heads/main", objects.third, objects.third, true],
            ["refs/heads/next", objects.second, objects.second, false],
            ["refs/tags/v0", objects.first, objects.first, true],
            ["refs/tags/v1", objects.release, objects.first, true],
            ["refs/tags/v2", objects.patch, objects.third, true],
        ],
    });

    // read the advertisement with a contents token once per delivery of the installation
    const refresh = [
        "POST api.github.com/app/installations/4242/access_tokens app",
        '  body {"repository_ids":[1296269],"permissions":{"contents":"read"}}',
        "GET github.com/example/site.git/info/refs?service=git-upload-pack installation 4242",
    ];
    expect(forge.github.requests).toEqual([...refresh, ...refresh, ...refresh]);

    // audit the identification and each refresh as the forge's own components, never their nested calls
    const success = "success";
    const target = { type: "repository", id: created.id };
    const refreshed = ["repository.refresh", target, success];
    expect(
        (await auditedCalls(forge.home.database))
            .filter(
                ({ execution }) =>
                    !audited.has(execution.id) &&
                    execution.context.package.id === forgePackage.id &&
                    execution.context.component !== undefined,
            )
            .map(({ method, execution }) => [
                method,
                execution.target.type === "reference" ? "reference" : execution.target,
                execution.outcome.kind,
            ]),
    ).toEqual([["repository.identify", target, success], refreshed, refreshed, refreshed]);

    // refuse a delivery signed with another secret before the forge sees it, and a read of the webhook
    const refused = await deliver(
        forge,
        {
            id: "5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e06",
            event: "delete",
            payload: {
                ref: "main",
                ref_type: "branch",
                pusher_type: "user",
                ...source(INSTALLATION),
            },
        },
        "another secret",
    );
    const read = await forge.fetch("/webhooks/github");
    expect([
        [refused.status, await refused.json()],
        [read.status, await read.text()],
    ]).toEqual([
        [
            401,
            {
                code: "UNAUTHORIZED",
                status: 401,
                message: "authentication required: webhook signature does not match",
                defined: true,
                reason: "SIGNATURE_UNAUTHENTICATED",
                domain: "@destack/object",
            },
        ],
        [405, ""],
    ]);

    // lease each token as Git's basic credential for the recorded remote, expiring an hour after its issue
    forge.github.requests.length = 0;
    const expiresAt = forge.environment.clock.now() + TOKEN_MILLISECONDS;
    expect([
        await owner.repository.open({ spaceId: forge.space, id: created.id, mode: "read" }),
        await owner.repository.open({ spaceId: forge.space, id: created.id, mode: "write" }),
    ]).toEqual([
        lease("read", "ghs_4242_contents_read", expiresAt),
        lease("write", "ghs_4242_contents_write", expiresAt),
    ]);

    // issue each token for GitHub's repository identifier alone, with the contents permission of its mode
    expect(forge.github.requests).toEqual([
        "POST api.github.com/app/installations/4242/access_tokens app",
        '  body {"repository_ids":[1296269],"permissions":{"contents":"read"}}',
        "POST api.github.com/app/installations/4242/access_tokens app",
        '  body {"repository_ids":[1296269],"permissions":{"contents":"write"}}',
    ]);

    // audit each lease as the owner's call and its success, without the credential
    const actor = { subject: tenant.owner.subject };
    expect(
        (await auditedCalls(forge.home.database))
            .filter(
                (call) =>
                    call.method === "repository.open" && call.execution.target.id === created.id,
            )
            .map((call) => [
                AuditCaller.actor(call.execution.context.caller),
                call.execution.target,
                call.execution.details,
                call.execution.outcome,
            ]),
    ).toEqual([
        [actor, target, {}, { kind: "success" }],
        [actor, target, {}, { kind: "success" }],
    ]);

    // open the repository as an anonymous Git remote without GitHub's identity or the installation
    const anonymous = await owner.repository.update({
        spaceId: forge.space,
        id: created.id,
        requestId: RequestId.create(forge.environment),
        hosting: "git",
        authentication: "anonymous",
        connectionId: null,
    });
    expect([
        anonymous.hosting,
        anonymous.providerRepositoryId,
        anonymous.authentication,
        anonymous.connectionId,
    ]).toEqual(["git", null, "anonymous", null]);
});

/** Build the lease of a GitHub installation token for the recorded remote. */
function lease(mode: "read" | "write", password: string, expiresAt: number) {
    return {
        url: "https://github.com/example/site.git",
        mode,
        headers: {
            authorization: `Basic ${new TextEncoder().encode(`x-access-token:${password}`).toBase64()}`,
        },
        expiresAt,
    };
}
