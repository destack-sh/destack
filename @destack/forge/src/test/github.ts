import type { Crypto, Environment } from "@destack/environment";
import { found, type JsonObject, nth, schema } from "@destack/schema";

/** The fixture GitHub App's client ID. */
export const APP_ID = "Iv23liFixtureApp";

/** The secret the fixture GitHub App signs webhook deliveries with. */
export const WEBHOOK_SECRET = "It's a Secret to Everybody";

/** The longest lifetime GitHub accepts for an app's JWT: ten minutes. */
const APP_TOKEN_SECONDS = 600;

/** The host serving the stand-in's Git remotes. */
export const GITHUB_HOST = "github.com";

/** The lifetime claims of an app JWT, in epoch seconds. */
const JwtLifetime = schema.looseObject({ iat: schema.number(), exp: schema.number() });

/** The permissions an installation token request grants. */
const Grant = schema.looseObject({
    permissions: schema.record(schema.string(), schema.string()).default({}),
});

/** A fixed RS256 PKCS #8 key of the fixture GitHub App, as generating one takes 100 ms. */
const PRIVATE_KEY =
    "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCZYGBaXnG/aT61p8aryFLQSkP9iXHIIB8swHMKojFDuShYUIrCPQy/QlKwExWvbuqs1WP7E48zo2PbnbPLGOa6IDoq2wtNeWxhveOXn8BROMPIZ+qAXlsNfBjaNpKtuQUyJDm+7TcaHo0dLe7oCSu3snQw19JNFuLfMKM6pazvS/XvbA6O//WIPlrSu3sVjuuw3qigPgcjz09Y3vdG+CbZ/49Jtev/ti6ojCS5rhmRmVUEx15E1Wp+bZ3kOl+OB2ov+sO3+txOQyfQJ1pIUD+zit+kJSVzns8izFoMCd0J8pAryBoGAVZZbJXfiPwpUFou6rVkNGTvqRKYoU0j4YUrAgMBAAECggEAAfp1nCFuNZlMzQLjgUcpgNMVrBydgleeQQJI5xHQ4JklyaD+206zE13aTDEsgxdk/AEMUiBeRte9I0sIpIiPnVOt4XLzEjBdDgaluHEXau+7vo3V2h8JPeTJVjn3MMC5wjHkvfKkJ38Ri5c0D1n94PleU/BTd0wXniulGDLoTWFWXPkP9tUOLrI/V0MDPn3o1QkyaLfCtHR3t9TedqRA7qwlHBKBbSoK0MERVnvdUgJEPgOCtLGp0zFHWtlC+krHI0ROo47AEYB3X/oGM7T/OmYDOEdvQdF0mASdn/ICHIg/rIqcPuN7xpc+kzVN9aEIYIsLc77k//v3VYqNvY9YgQKBgQDL9lTGy9uEfTUQEhQjLIuay/ZFq4G9RKq2pAXtBj35G6fjXoPji/Pf8qNRFNj5HV22bAzRdfzmRhqMdSy5kyWN2+85uokGbJ5hfsaa4TY+ZdGDlq9DmCr+tu4RH4IXj2ioJ9W/lbTLHN8J87AWwvxSSj4pwgiNo7nLFxwXV0KKgQKBgQDAghErwglhkfBoig1VZgOHV2s0Vk0JIo0ocm27GQ3DT6P27KSRtHrSBxjQduQ/iHb2jKBaG9JXqsXmeuyctA2Fq/rU94h4XT76u9bhGVL1OMmQuLMHOo7nnsjueBTeJ8+p2qixcMmW23bGkwBZdji1wQC8OvuFIyRMegHgmKSBqwKBgQCYnVNGBRfWhRGFWuGyiAxV4bP+4ZwMkSrjysucVYVdtnHjUb+CLiBnO9k0PFM14+FRNHxK5uw7Kc2Ht82ldhMtmJniKk+JfRDthrz4+vdprSoeOkweWegh+6MbMOZY1rfKyzPHTS+go5PIkIz8Gx5OPLtS55sa0J2e90XPuT1TAQKBgASt5K70m3fErVxNJr/Rp/pNNKzkACFdtqKqDWLD3CLIN8sTu3hTM70RMRg4G5ozvfiGOIuYK4e3/fF5QXEZq055fLJahfWvBo3frPxxuiN9yuB6rNdbJAgbQvUQ9CjmPNT7HKibfjrtgLiY8CQ8jGpWk6b/pAd7cDFvscX8z9rbAoGBAMCrBv78RspBB/AziQ24/Xbu6wuJCmVylDly28rJQ3KiS1TTWoK+dzyd4QCs1WeuFY4acVRtF0Dh4Ce/zzkIY76Q45Kz00Ybz98O50VaWkMRlpCujDlLSLqHjiwbW63t1PRD8bjQgvc7pxo765ufc58Iz53NpWFgT4srZ9QM5F+U";

/** The fixture GitHub App's public key, as SPKI. */
const PUBLIC_KEY =
    "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAmWBgWl5xv2k+tafGq8hS0EpD/YlxyCAfLMBzCqIxQ7koWFCKwj0Mv0JSsBMVr27qrNVj+xOPM6Nj252zyxjmuiA6KtsLTXlsYb3jl5/AUTjDyGfqgF5bDXwY2jaSrbkFMiQ5vu03Gh6NHS3u6Akrt7J0MNfSTRbi3zCjOqWs70v172wOjv/1iD5a0rt7FY7rsN6ooD4HI89PWN73Rvgm2f+PSbXr/7YuqIwkua4ZkZlVBMdeRNVqfm2d5DpfjgdqL/rDt/rcTkMn0CdaSFA/s4rfpCUlc57PIsxaDAndCfKQK8gaBgFWWWyV34j8KVBaLuq1ZDRk76kSmKFNI+GFKwIDAQAB";

/** The RS256 parameters of the fixture GitHub App's keys. */
const RS256 = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" } as const;

/** The Web Crypto verification parameters of each JWS algorithm the stand-ins accept. */
const ALGORITHMS: ReadonlyMap<string, AlgorithmIdentifier | EcdsaParams> = new Map([
    ["RS256", { name: "RSASSA-PKCS1-v1_5" }],
    ["ES256", { name: "ECDSA", hash: "SHA-256" }],
]);

/** The JWT header fields the stand-ins read. */
const Header = schema.looseObject({ alg: schema.string() });

/** The claims of a JWT. */
const Claims = schema.jsonObject();

/** How long an installation token lasts: an hour, as GitHub issues them. */
const TOKEN_MILLISECONDS = 3_600_000;

/** A Git object as GitHub's REST API describes it. */
interface GitObject {
    /** The object type: commit or tag. */
    readonly type: "commit" | "tag";
    /** The object name. */
    readonly sha: string;
}

/** One repository the stand-in hosts. */
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

/** A stand-in for the GitHub endpoints the platform calls, recording each request. */
export class GitHubStandIn {
    /** The repositories by owner and name. */
    readonly repositories = new Map<string, StandInRepository>();
    /** Each request as `METHOD path?query credential`, in order. */
    readonly requests: string[] = [];
    /** The installation each issued token belongs to. */
    readonly #tokens = new Map<string, number>();
    /** The clock tokens expire on and the crypto the app's JWTs are verified with. */
    readonly #environment: Pick<Environment, "clock" | "crypto">;

    /** Stand in for GitHub, on a world's clock and crypto. */
    constructor(environment: Pick<Environment, "clock" | "crypto">) {
        this.#environment = environment;
    }

    /** Take a repository. */
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

        // match a Git advertisement or an installation token request
        const advertised = /^\/([^/]+\/[^/]+)\.git\/info\/refs$/u.exec(url.pathname);
        const token = /^\/app\/installations\/(\d+)\/access_tokens$/u.exec(url.pathname);
        const installed = /^\/app\/installations\/(\d+)$/u.exec(url.pathname);

        // advertise a repository's references to its installation's tokens over Git's smart HTTP
        if (url.host === GITHUB_HOST && advertised !== null) {
            return this.#advertise(nth(advertised, 1), installation);
        }
        // refuse API requests without GitHub's API version and a user agent
        else if (
            request.headers.get("x-github-api-version") !== "2022-11-28" ||
            request.headers.get("user-agent") === null
        ) {
            return Response.json({ message: "missing version or user agent" }, { status: 400 });
        }
        // create an installation token for an app JWT
        else if (request.method === "POST" && token !== null) {
            return this.#issue(request, secret, Number(token[1]));
        }
        // uninstall the app for an app JWT
        else if (request.method === "DELETE" && installed !== null) {
            return this.#uninstall(secret);
        }
        // read a repository its installation opens
        else {
            return this.#read(url, installation);
        }
    };

    /** Advertise a repository's references to its installation's tokens. */
    #advertise(fullName: string, installation: number | undefined): Response {
        // hide repositories of other installations
        const hosted = this.repositories.get(fullName);
        if (hosted === undefined || hosted.installation !== installation) {
            return new Response("Repository not found.", { status: 404 });
        }

        return new Response(advertise(hosted), {
            headers: { "content-type": "application/x-git-upload-pack-advertisement" },
        });
    }

    /** Uninstall the app for a verified app JWT. */
    async #uninstall(secret: string | undefined): Promise<Response> {
        if (secret === undefined || !(await isAppToken(secret, this.#environment.crypto))) {
            return Response.json({ message: "bad jwt" }, { status: 401 });
        }

        return new Response(null, { status: 204 });
    }

    /** Create an installation token for a verified app JWT. */
    async #issue(
        request: Request,
        secret: string | undefined,
        installation: number,
    ): Promise<Response> {
        // verify the app's JWT and its lifetime
        if (secret === undefined) {
            throw new TypeError("an installation token request presents no app jwt");
        }
        if (!(await isAppToken(secret, this.#environment.crypto))) {
            return Response.json({ message: "bad jwt" }, { status: 401 });
        }

        // issue a token of the installation named by the permissions it grants, recording the request's body
        const body: unknown = await request.json();
        const granted = Object.entries(Grant.parse(body).permissions).map(
            ([name, level]) => `${name}_${level}`,
        );
        const issued = [`ghs_${installation}`, ...granted].join("_");
        this.#tokens.set(issued, installation);
        this.requests.push(`  body ${JSON.stringify(body)}`);

        return Response.json(
            {
                token: issued,
                expires_at: new Date(
                    this.#environment.clock.now() + TOKEN_MILLISECONDS,
                ).toISOString(),
                permissions: { contents: "read", metadata: "read" },
                repository_selection: "selected",
            },
            { status: 201 },
        );
    }

    /** Read the repository of a path for its installation's tokens. */
    #read(url: URL, installation: number | undefined): Response {
        // find the repository the installation opens
        const path = /^\/repos\/([^/]+\/[^/]+)(\/.*)?$/u.exec(url.pathname);
        const repository = path === null ? undefined : this.repositories.get(nth(path, 1));
        if (path === null || repository === undefined || repository.installation !== installation) {
            return Response.json({ message: "Not Found" }, { status: 404 });
        }

        // read the repository itself
        if (path[2] === undefined) {
            return Response.json({
                id: repository.id,
                node_id: `R_${repository.id}`,
                name: repository.fullName.split("/")[1],
                full_name: repository.fullName,
                private: true,
                default_branch: repository.defaultBranch,
            });
        }
        // hide the paths below it
        else {
            return Response.json({ message: "Not Found" }, { status: 404 });
        }
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

    return password === undefined ? presented : nth(password.split(":"), 1);
}

/** Report whether a bearer is the app's JWT, issued by the app for at most ten minutes. */
async function isAppToken(secret: string, crypto: Crypto): Promise<boolean> {
    const { claims } = await verifyToken(secret, await githubPublicKey(crypto), crypto);
    const lifetime = JwtLifetime.safeParse(claims);

    return (
        claims["iss"] === APP_ID &&
        lifetime.success &&
        lifetime.data.exp - lifetime.data.iat <= APP_TOKEN_SECONDS
    );
}

/** Write the fixture GitHub App's private key as a PKCS #8 PEM, as its connection stores it. */
export function githubPrivateKey(): string {
    const lines = PRIVATE_KEY.match(/.{1,64}/gu) ?? [];

    return `-----BEGIN PRIVATE KEY-----\n${lines.join("\n")}\n-----END PRIVATE KEY-----\n`;
}

/** Import the fixture GitHub App's public key. */
export function githubPublicKey(crypto: Crypto): Promise<CryptoKey> {
    return crypto.importKey("spki", Uint8Array.fromBase64(PUBLIC_KEY), RS256, false, ["verify"]);
}

/** Verify a compact JWT's signature, returning its header algorithm and claims. */
export async function verifyToken(
    token: string,
    key: CryptoKey,
    crypto: Crypto,
): Promise<{ readonly alg: string; readonly claims: JsonObject }> {
    // decode the header and claims
    const [header, claims, signature] = token.split(".");
    if (header === undefined || claims === undefined || signature === undefined) {
        throw new TypeError("a compact jwt has three parts");
    }
    const { alg } = Header.parse(decode(header));

    // require the key's signature over the header and claims
    const isValid = await crypto.verify(
        found(ALGORITHMS, alg),
        key,
        Uint8Array.fromBase64(signature, { alphabet: "base64url" }),
        new TextEncoder().encode(`${header}.${claims}`),
    );
    if (!isValid) {
        throw new Error(`invalid ${alg} signature`);
    }

    return { alg, claims: Claims.parse(decode(claims)) };
}

/** Decode a base64url JSON part of a compact JWT. */
function decode(part: string): unknown {
    return JSON.parse(
        new TextDecoder().decode(Uint8Array.fromBase64(part, { alphabet: "base64url" })),
    );
}
