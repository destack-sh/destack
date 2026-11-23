import { testSimulation } from "@destack/object/test";
import { Secret } from "@destack/resource";
import { expect, refusal, test } from "@destack/test";
import { GitHubApp, GITHUB_API } from "./index.ts";
import { APP_ID, GitHubStandIn, githubPrivateKey } from "../test/github.ts";

/** The fixture GitHub App's installation on the tenant's GitHub organization. */
const INSTALLATION = 4242;

/** How long GitHub's installation tokens last: an hour. */
const TOKEN_MILLISECONDS = 3_600_000;

/** The world the App signs in. */
const world = testSimulation().environment("github");

/** The fixture repository's default branch commit. */
const COMMIT = "aa218f56b14c9653891f9e74264a383fa43fefbd";

/** GitHub's identifier of the fixture repository. */
const REPOSITORY_ID = 1296269;

/** The time the app signs its JWTs at, the world's start. */
const NOW = world.clock.now();

/** Point the fixture GitHub App at a stand-in. */
function open(github: GitHubStandIn): GitHubApp {
    return new GitHubApp({
        id: APP_ID,
        environment: world,
        key: Secret.of(githubPrivateKey()),
        api: GITHUB_API,
        fetch: github.fetch,
    });
}

test("issue installation tokens limited to a repository and its use", async () => {
    // host example/site at GitHub
    const github = new GitHubStandIn(world);
    github.add({
        id: REPOSITORY_ID,
        fullName: "example/site",
        installation: INSTALLATION,
        defaultBranch: "main",
        references: new Map([["refs/heads/main", { type: "commit", sha: COMMIT }]]),
        tags: new Map(),
    });
    const app = open(github);

    // identify the repository by name and lease writing to it by identifier
    const fullName = GitHubApp.fullName("https://github.com/example/site.git");
    expect([
        await app.repository(String(INSTALLATION), fullName, NOW),
        await app.open(
            String(INSTALLATION),
            String(REPOSITORY_ID),
            "https://github.com/example/site.git",
            "write",
            NOW,
        ),
    ]).toEqual([
        { id: String(REPOSITORY_ID), fullName: "example/site" },
        {
            url: "https://github.com/example/site.git",
            mode: "write",
            headers: {
                authorization: `Basic ${new TextEncoder().encode("x-access-token:ghs_4242_contents_write").toBase64()}`,
            },
            expiresAt: NOW + TOKEN_MILLISECONDS,
        },
    ]);

    // call the app endpoint with the app JWT and the repository with a metadata token
    expect(github.requests).toEqual([
        "POST api.github.com/app/installations/4242/access_tokens app",
        '  body {"repositories":["site"],"permissions":{"metadata":"read"}}',
        "GET api.github.com/repos/example/site installation 4242",
        "POST api.github.com/app/installations/4242/access_tokens app",
        '  body {"repository_ids":[1296269],"permissions":{"contents":"write"}}',
    ]);

    // refuse remotes outside github.com
    expect(await refusal(() => GitHubApp.fullName("https://gitlab.com/example/site.git"))).toEqual([
        "BAD_REQUEST",
        "GITHUB_REPOSITORY_REQUIRED",
        "github.com repository is required: remote 'https://gitlab.com/example/site.git' is not one",
    ]);
});

test("read and uninstall the installation of a setup redirect", async () => {
    // read the installation, then uninstall it with the app's JWT
    const github = new GitHubStandIn(world);
    const app = open(github);
    const installation = app.installation({ installation_id: "4242", setup_action: "install" });
    await app.uninstall(installation, NOW);
    expect({ installation, requests: github.requests }).toEqual({
        installation: "4242",
        requests: ["DELETE api.github.com/app/installations/4242 app"],
    });

    // refuse a redirect naming no installation
    expect(await refusal(() => app.installation({ setup_action: "install" }))).toEqual([
        "BAD_REQUEST",
        "AUTHORIZATION_INVALID",
        "installation redirect is invalid: it has no installation",
    ]);
});
