import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { CLIENT_HEAD_HEADER, ClientHead } from "@destack/object/workload";
import { ROUTE_HEADER, SPACE_HEADER, SpaceRequest } from "./route.ts";

/** A space of the scenarios. */
const SPACE = "space-01996ab0-0000-7000-8000-000000000002";

/** An account of the scenarios. */
const ACCOUNT = "account-01996ab0-0000-7000-8000-000000000001";

/** The stack service's mount path. */
const SERVICE = "/package-01a0c80b-614e-739c-9b50-70e5119010b1";

/** The stack service's mount on the machine's origin. */
const MOUNT = `https://eu.destack.cloud${SERVICE}`;

/** The installation origins below the space domain, where only work--ada lives. */
const ORIGINS = {
    domain: "destack.space",
    find: async (address: string) => (address === "work--ada" ? SPACE : undefined),
};

test("route a request to the space of its S3 host name, space prefix, scoped route, creation or replica scope, and none for an account or otherwise", async () => {
    // read the space, its header and the path of each request at the space
    const requests = [
        new Request(`https://${SPACE}.s3.eu.destack.cloud/bucket-1/notes.txt`),
        new Request(
            `https://eu.destack.cloud/spaces/${SPACE}/.destack/egress/@destack/vault/ingest`,
        ),
        new Request(`${MOUNT}/spaces/${SPACE}/installations`),
        new Request(`${MOUNT}/accounts/${ACCOUNT}/spaces`, {
            method: "POST",
            body: JSON.stringify({ id: SPACE, requestId: "r", name: "work" }),
        }),
        new Request(`${MOUNT}/accounts/${ACCOUNT}/spaces`, {
            method: "POST",
            body: JSON.stringify({ requestId: "r", name: "work" }),
        }),
        new Request(`${MOUNT}/store/stream`, {
            method: "POST",
            body: JSON.stringify({
                name: "replica",
                publication: "space",
                scope: SPACE,
                target: SPACE,
            }),
        }),
        new Request(`${MOUNT}/store/stream`, {
            method: "POST",
            body: JSON.stringify({
                name: "replica",
                publication: "user",
                scope: ACCOUNT,
                target: ACCOUNT,
            }),
        }),
        new Request(`${MOUNT}/addresses/locale`, { method: "POST", body: "{}" }),
    ];
    const routed = await Promise.all(
        requests.map(async (request) => {
            const found = await SpaceRequest.of(request, ORIGINS);

            return found === undefined
                ? undefined
                : [
                      found.space,
                      found.request.headers.get(SPACE_HEADER),
                      new URL(found.request.url).pathname,
                  ];
        }),
    );

    // add the space to its header and strip only an explicit space prefix
    expect(routed).toEqual([
        [SPACE, SPACE, "/bucket-1/notes.txt"],
        [SPACE, SPACE, "/.destack/egress/@destack/vault/ingest"],
        [SPACE, SPACE, `${SERVICE}/spaces/${SPACE}/installations`],
        [SPACE, SPACE, `${SERVICE}/accounts/${ACCOUNT}/spaces`],
        undefined,
        [SPACE, SPACE, `${SERVICE}/store/stream`],
        undefined,
        undefined,
    ]);
});

test("route a request to the space of its installation origin, as a relay forwards it or at its own host", async () => {
    // address work--ada's notes through the relay and at their own host
    const requests = [
        new Request("https://eu.destack.cloud/.destack/package/notes", {
            headers: { forwarded: 'host="notes--work--ada.destack.space";proto=https' },
        }),
        new Request("https://notes--work--ada.destack.space/.destack/package/notes"),
        new Request("https://notes--home--grace.destack.space/.destack/package/notes"),
    ];
    const routed = await Promise.all(
        requests.map(async (request) => (await SpaceRequest.of(request, ORIGINS))?.space),
    );

    // find work--ada's space and none for a space nowhere
    expect(routed).toEqual([SPACE, SPACE, undefined]);
});

test("route a nested scoped route to its space, a stream below a space to the space, a call to an account nowhere, a space's URL to its space, and an installation's mount to its space", async () => {
    // address the space in each form a request names it
    const requests = [
        new Request(`${MOUNT}/accounts/${ACCOUNT}/spaces/${SPACE}/relationships`, {
            method: "POST",
            body: JSON.stringify({ requestId: "r" }),
        }),
        new Request(`${MOUNT}/store/stream`, {
            method: "POST",
            body: JSON.stringify({
                name: "chain",
                publication: "chain",
                scope: ACCOUNT,
                target: SPACE,
            }),
        }),
        new Request(`${MOUNT}/store/call`, {
            method: "POST",
            body: JSON.stringify({ scope: ACCOUNT, target: SPACE }),
        }),
        new Request(`https://eu.destack.cloud/spaces/${SPACE}${SERVICE}/store/stream`, {
            method: "POST",
            body: JSON.stringify({ name: "build", publication: "build", scope: "universe" }),
        }),
        new Request(
            `https://eu.destack.cloud/spaces/${SPACE}/installations/installation-01996ab0-0000-7000-8000-000000000004/.destack/journal`,
        ),
    ];
    const routed = await Promise.all(
        requests.map(async (request) => {
            const found = await SpaceRequest.of(request, ORIGINS);

            return found === undefined
                ? undefined
                : [found.space, new URL(found.request.url).pathname];
        }),
    );

    // find the space of each addressed request
    expect(routed).toEqual([
        [SPACE, `${SERVICE}/accounts/${ACCOUNT}/spaces/${SPACE}/relationships`],
        [SPACE, `${SERVICE}/store/stream`],
        undefined,
        [SPACE, `${SERVICE}/store/stream`],
        [
            SPACE,
            `/spaces/${SPACE}/installations/installation-01996ab0-0000-7000-8000-000000000004/.destack/journal`,
        ],
    ]);
});

test("route a request to a name to the space of its installation origin served here, else to its route's space with the route and the URL asked for, and none for a name nothing serves", async () => {
    // serve work--ada here and route notes.acme.com/blog to a route of another space
    const ROUTED = "space-01996ab0-0000-7000-8000-000000000003";
    const ROUTE = "route-01996ab0-0000-7000-8000-000000000004";
    const names = {
        ...ORIGINS,
        route: async (host: string, path: string) =>
            host === "notes.acme.com" && path.startsWith("/blog")
                ? { scope: ROUTED, id: schema.identifier("route").parse(ROUTE) }
                : undefined,
    };

    // read the space, the route and the URL of each name
    const named = await Promise.all(
        [
            "https://notes--work--ada.destack.space/",
            "https://notes.acme.com/blog/2026?page=2",
            "https://notes--other--ada.destack.space/",
            "https://notes.acme.com/shop",
        ].map(async (url) => {
            const found = await SpaceRequest.named(new Request(url), names);

            return found === undefined
                ? undefined
                : [
                      found.space,
                      found.request.headers.get(ROUTE_HEADER),
                      ClientHead.restore(found.request)?.url ?? null,
                  ];
        }),
    );

    expect(named).toEqual([
        [SPACE, null, null],
        [ROUTED, ROUTE, "https://notes.acme.com/blog/2026?page=2"],
        undefined,
        undefined,
    ]);
});

test("drop the route and the URL a client sends to the machine's own host, which only the machine hands on", async () => {
    const found = await SpaceRequest.of(
        new Request(`https://eu.destack.cloud/spaces/${SPACE}/notes`, {
            headers: {
                [ROUTE_HEADER]: "route-01996ab0-0000-7000-8000-000000000004",
                [CLIENT_HEAD_HEADER]: ClientHead.write(new Request("https://notes.acme.com/")),
            },
        }),
        ORIGINS,
    );

    expect([
        found?.space,
        found?.request.headers.get(ROUTE_HEADER),
        found?.request.headers.get(CLIENT_HEAD_HEADER),
    ]).toEqual([SPACE, null, null]);
});
