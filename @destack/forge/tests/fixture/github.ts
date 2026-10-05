import type {
    ConnectionCallback,
    ConnectionGrant,
    ConnectionProvider,
    ConnectionRequest,
    ConnectionRevocation,
} from "@destack/account/server";
import { aligned, found, present, schema } from "@destack/schema";
import { githubPublicKey } from "./key.ts";
import { verifyToken } from "./token.ts";

/** The fixture GitHub App's client ID. */
export const APP_ID = "Iv23liFixtureApp";

/** The REST API base the stand-in serves. */
export const GITHUB_API = new URL("https://api.github.test");

/** The host serving the stand-in's Git remotes. */
const GITHUB_HOST = "github.com";

/** The lifetime claims of an app JWT, in epoch seconds. */
const JwtLifetime = schema.looseObject({ iat: schema.number(), exp: schema.number() });

/** A Git object as GitHub's REST API names it. */
interface GitObject {
    /** The object type: commit or tag. */
    readonly type: "commit" | "tag";
    /** The object name. */
    readonly sha: string;
}

/** One repository the stand-in keeps. */
export interface StandInRepository {
    /** GitHub's numeric repository identifier. */
    readonly id: number;
    /** The owner and name. */
    readonly fullName: string;
    /** The installation the repository is installed in. */
    readonly installation: number;
    /** The default branch's short name. */
    readonly defaultBranch: string;
    /** The references by full name. */
    readonly references: Map<string, GitObject>;
    /** The annotated tag objects by name with their tagged objects. */
    readonly tags: Map<string, GitObject>;
}

/** An in-process stand-in for exactly the GitHub endpoints the platform calls, REST and Git's advertisement, recording each request. */
export class GitHubStandIn {
    /** The repositories by owner and name. */
    readonly repositories = new Map<string, StandInRepository>();
    /** Each request as `METHOD path?query credential`, in order. */
    readonly requests: string[] = [];
    /** The installation each issued token belongs to. */
    readonly #tokens = new Map<string, number>();

    /** Keep a repository. */
    add(repository: StandInRepository): void {
        this.repositories.set(repository.fullName, repository);
    }

    /** Serve one request as GitHub does. */
    readonly fetch = async (input: RequestInfo | URL, options?: RequestInit): Promise<Response> => {
        // record the request with the credential it presents, as a bearer or as Git's basic password
        const request = new Request(input, options);
        const url = new URL(request.url);
        const secret = presentedSecret(request.headers.get("authorization"));
        const installation = secret === undefined ? undefined : this.#tokens.get(secret);
        const credential = installation === undefined ? "app" : `installation ${installation}`;
        this.requests.push(
            `${request.method} ${url.host}${url.pathname}${url.search} ${credential}`,
        );

        // advertise a repository's references to its installation's tokens over Git's smart HTTP
        const advertised = /^\/([^/]+\/[^/]+)\.git\/info\/refs$/u.exec(url.pathname);
        if (url.host === GITHUB_HOST && advertised !== null) {
            const hosted = this.repositories.get(aligned(advertised, 1));
            if (hosted === undefined || hosted.installation !== installation) {
                return new Response("Repository not found.", { status: 404 });
            }

            return new Response(advertise(hosted), {
                headers: { "content-type": "application/x-git-upload-pack-advertisement" },
            });
        }
        if (
            request.headers.get("x-github-api-version") !== "2022-11-28" ||
            request.headers.get("user-agent") === null
        ) {
            return Response.json({ message: "missing version or user agent" }, { status: 400 });
        }

        // create an installation token for an app JWT
        const token = /^\/app\/installations\/(\d+)\/access_tokens$/u.exec(url.pathname);
        if (request.method === "POST" && token !== null) {
            if (secret === undefined) {
                throw new TypeError("an installation token request presents no app jwt");
            }
            const { claims } = await verifyToken(secret, await githubPublicKey());
            const lifetime = JwtLifetime.safeParse(claims);
            if (
                claims["iss"] !== APP_ID ||
                !lifetime.success ||
                lifetime.data.exp - lifetime.data.iat > 600
            ) {
                return Response.json({ message: "bad jwt" }, { status: 401 });
            }
            const issued = `ghs_${token[1]}_${this.#tokens.size + 1}`;
            this.#tokens.set(issued, Number(token[1]));
            this.requests.push(`  body ${JSON.stringify(await request.json())}`);

            return Response.json(
                {
                    token: issued,
                    expires_at: "2026-09-27T13:00:00Z",
                    permissions: { contents: "read", metadata: "read" },
                    repository_selection: "selected",
                },
                { status: 201 },
            );
        }

        // serve the repository the path names to its installation's tokens
        const path = /^\/repos\/([^/]+\/[^/]+)(\/.*)?$/u.exec(url.pathname);
        const repository = path === null ? undefined : this.repositories.get(aligned(path, 1));
        if (path === null || repository === undefined || repository.installation !== installation) {
            return Response.json({ message: "Not Found" }, { status: 404 });
        }
        const rest = path[2] ?? "";

        // read the repository
        if (rest === "") {
            return Response.json({
                id: repository.id,
                node_id: `R_${repository.id}`,
                name: repository.fullName.split("/")[1],
                full_name: repository.fullName,
                private: true,
                default_branch: repository.defaultBranch,
            });
        }

        return Response.json({ message: "Not Found" }, { status: 404 });
    };
}

/** The fixture GitHub App's installations as the account service connects them: each installation in its GitHub organisation. */
export class GitHubInstallations implements ConnectionProvider {
    /** The provider's name connections record. */
    readonly name = "github";
    /** The provider instance. */
    readonly issuer = `https://${GITHUB_HOST}`;
    /** The installed application. */
    readonly applicationId = APP_ID;
    /** The provider installs the application. */
    readonly kind = "installation";
    /** The installations withdrawn so far. */
    readonly revoked: ConnectionRevocation[] = [];
    /** The GitHub organisation of each installation. */
    readonly #organisations: ReadonlyMap<string, string>;

    /** Install the application in organisations, by installation. */
    constructor(organisations: ReadonlyMap<string, string>) {
        this.#organisations = organisations;
    }

    /** Build the installation page carrying the state. */
    authorize(request: ConnectionRequest): URL {
        const page = new URL("/apps/fixture/installations/new", this.issuer);
        page.searchParams.set("state", request.state);

        return page;
    }

    /** Read the installation the callback names, with the permissions the application holds. */
    async complete(callback: ConnectionCallback): Promise<ConnectionGrant> {
        const installationId = present(callback.parameters["installation_id"], "the installation");

        return {
            subject: found(this.#organisations, installationId),
            scopes: [],
            permissions: { contents: "read", metadata: "read" },
            installationId,
        };
    }

    /** Record a withdrawn installation. */
    async revoke(grant: ConnectionRevocation): Promise<void> {
        this.revoked.push(grant);
    }
}

/** Encode a repository's upload-pack advertisement like GitHub. */
function advertise(repository: StandInRepository): Uint8Array<ArrayBuffer> {
    // name HEAD's commit and branch and each reference, following tag objects to their commit
    const head = `refs/heads/${repository.defaultBranch}`;
    const lines = [
        `${found(repository.references, head).sha} HEAD\0symref=HEAD:${head} agent=git/github-7c1a`,
    ];
    for (const name of [...repository.references.keys()].toSorted()) {
        let object = found(repository.references, name);
        lines.push(`${object.sha} ${name}`);
        if (object.type === "tag") {
            while (object.type === "tag") {
                object = found(repository.tags, object.sha);
            }
            lines.push(`${object.sha} ${name}^{}`);
        }
    }

    // frame the service line, a flush, the references and a closing flush as pkt-lines
    return new TextEncoder().encode(
        `${packet("# service=git-upload-pack")}0000${lines.map(packet).join("")}0000`,
    );
}

/** Frame a line as a pkt-line with its newline. */
function packet(line: string): string {
    const length = new TextEncoder().encode(line).length + 5;

    return `${length.toString(16).padStart(4, "0")}${line}\n`;
}

/** Read the secret an authorization header presents, as a bearer or as Git's basic password. */
function presentedSecret(authorization: string | null): string | undefined {
    // present nothing without a header
    if (authorization === null) {
        return undefined;
    }

    // read the bearer token, or the password of a basic credential
    const [scheme, presented] = authorization.split(" ");
    if (presented === undefined) {
        throw new TypeError(`authorization ${scheme} presents no credential`);
    }
    const password =
        scheme === "Basic" ? new TextDecoder().decode(Uint8Array.fromBase64(presented)) : undefined;

    return password === undefined ? presented : aligned(password.split(":"), 1);
}
