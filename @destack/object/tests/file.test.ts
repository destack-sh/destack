import { testSimulation } from "../src/test/index.ts";
import { defineObject, field } from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf, TestBearer } from "../src/test/index.ts";
import { ObjectFile, ObjectFilePart } from "../src/file/index.ts";
import { principal, relation } from "@destack/access";
import { blobTable, defineDatabase, inArray } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Digest, present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The space containing the boards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000c1");

/** A photo's identifier. */
const Photo = schema.identifier("photo");

/** How a browser shows the photo and saves it under its own name. */
const DISPOSITION = "inline; filename*=UTF-8''lisboa%20%C3%A9%20%281%29.jpg";

/** Read an answer's status, headers that describe its body, and a successful body. */
async function shown(answer: Promise<Response>) {
    const response = await answer;
    const { headers } = response;

    return [
        response.status,
        ...["content-type", "content-disposition", "content-range"].flatMap(
            (name) => headers.get(name) ?? [],
        ),
        response.status < 400 ? await response.text() : "",
    ];
}

/** Photos with an image of at most ten bytes. */
const photo = defineObject(
    {
        name: "photo",
        plural: "photos",
        title: t`${plural(argument("count"), { one: "Photo", other: "Photos" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(),
            image: field.file({ accept: ["image/*"], size: 10 }),
        },
        access: { permissions: { read: relation("owner"), write: relation("owner") } },
    },
    (method) => ({
        get: method.get("read"),
        create: method.create("write", { fields: ["name", "image"] }),
        update: method.update("write", { fields: ["name", "image"] }),
    }),
);

/** Serve photos in a space, asking as one of its users. */
async function open() {
    // serve photos to their owner and to someone else
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "photo",
            ...tablesOf({
                packages: [objectPackage, { objects: { photo }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([photo]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: photo.package },
    });
    const served = TestObjectStore.serve(store);
    onTestFinished(() => served[Symbol.asyncDispose]());
    const server = served.server(present(store.packages[0], "the served package").package);

    // ask for a path of the store's files, signed in as a user or not at all
    const ask = (
        path: string,
        options: {
            readonly method?: string;
            readonly body?: BodyInit;
            readonly headers?: Readonly<Record<string, string>>;
        } = {},
        user: string | null = users.frank,
    ) =>
        server.fetch(
            new Request(`https://photos.test/store/file${path}`, {
                ...options,
                headers: {
                    ...(user === null
                        ? {}
                        : {
                              authorization: TestBearer.header(
                                  principal.user.reference(Scope.universe.id, user),
                              ),
                          }),
                    ...options.headers,
                },
            }),
        );

    // send a file's bytes in one part, and make the file
    const send = async (text: string, name: string, type: string) => {
        const bytes = new TextEncoder().encode(text);
        const digest = await Digest.of(bytes, world.crypto);
        const part = ObjectFilePart.parse(
            await (await ask(`/${digest}`, { method: "PUT", body: bytes })).json(),
        );
        const made = await ask("", {
            method: "POST",
            body: JSON.stringify({ parts: [part], name, type }),
        });

        return { part, file: ObjectFile.parse(await made.json()) };
    };
    const context = userContext(users.frank, spaceId, { environment: world }).access();
    const create = async (image: ObjectFile) => {
        const [created] = await store.mutate(context, {
            id: RequestId.create(world),
            calls: [photo.call("create", { spaceId, name: "Lisbon", image })],
        });

        return created;
    };
    const update = async (input: {
        readonly id: string;
        readonly name?: string;
        readonly image?: ObjectFile;
    }) => {
        const [updated] = await store.mutate(context, {
            id: RequestId.create(world),
            calls: [photo.call("update", { ...input, spaceId, id: Photo.parse(input.id) })],
        });

        return updated;
    };

    return { storage, ask, send, create, update };
}

test("store a file in an object's field, refusing one the store did not make or the field does not take", async () => {
    // create a photo, rename it, and write other files to it
    const { storage, send, create, update } = await open();
    const { file: image } = await send("a photo", "lisbon.jpg", "image/jpeg");
    const created = await create(image);
    const { id } = created;
    const renamed = await update({ id, name: "Tram 28" });
    const { file: text } = await send("text", "notes.txt", "text/plain");
    const { file: large } = await send("eleven byte", "large.png", "image/png");
    const { file: gone } = await send("gone", "gone.png", "image/png");
    await storage.database.delete(blobTable).where(inArray(blobTable.digest, gone.parts));
    const replace = (file: ObjectFile) => refusal(update({ id, image: file }));

    expect({
        created: created.image,
        renamed: [renamed.name, renamed.image],
        renamedFile: await replace({ ...image, name: "stolen.jpg" }),
        otherParts: await replace({ ...image, parts: text.parts }),
        text: await replace(text),
        large: await replace(large),
        gone: await replace(gone),
    }).toEqual({
        created: image,
        renamed: ["Tram 28", image],
        renamedFile: [
            "BAD_REQUEST",
            "FILE_INVALID",
            "file 'stolen.jpg' is invalid: the store did not make it",
        ],
        otherParts: [
            "BAD_REQUEST",
            "FILE_INVALID",
            "file 'lisbon.jpg' is invalid: the store did not make it",
        ],
        text: [
            "BAD_REQUEST",
            "FILE_INVALID",
            "file 'notes.txt' is invalid: 'image' takes no file of type text/plain",
        ],
        large: [
            "BAD_REQUEST",
            "FILE_INVALID",
            "file 'large.png' is invalid: 'image' takes at most 10 bytes",
        ],
        gone: [
            "BAD_REQUEST",
            "FILE_INVALID",
            "file 'gone.png' is invalid: its bytes are gone, so send it again",
        ],
    });
});

test("send, make, write and answer a file over HTTP, to callers who may read its object", async () => {
    // send its bytes, make the file, and write it
    const { ask, send, create } = await open();
    const { part, file: image } = await send("a photo", "lisboa é (1).jpg", "image/jpeg");
    const created = await create(image);

    // read it whole, by range and again, and as others
    const path = `/${spaceId}/photo/${created.id}/image`;
    const etag = `"${image.signature}"`;
    const bytes = new TextEncoder().encode("a photo");
    const put = { method: "PUT", body: bytes };
    const make = async (name: string, type: string) => {
        const body = JSON.stringify({ parts: [part], name, type });
        const answer = await ask("", { method: "POST", body });

        return answer.status;
    };

    expect({
        whole: await shown(ask(path)),
        range: await shown(ask(path, { headers: { range: "bytes=2-4" } })),
        end: await shown(ask(path, { headers: { range: "bytes=-3" } })),
        outside: await shown(ask(path, { headers: { range: "bytes=9-" } })),
        again: await shown(ask(path, { headers: { "if-none-match": `"other", W/${etag}` } })),
        names: await Promise.all(
            ["a/b.png", "..", "a\nb.png", "\uD800.png"].map((name) => make(name, "image/png")),
        ),
        types: await Promise.all(
            ["", "png", "image/png\r\nx: y"].map((type) => make("a.png", type)),
        ),
        head: await shown(ask(path, { method: "HEAD" })),
        other: await shown(ask(path, {}, users.alice)),
        field: await shown(ask(`/${spaceId}/photo/${created.id}/name`)),
        beyond: await shown(ask(`${path}/0`)),
        anonymous: await shown(ask(`/${part.digest}`, put, null)),
        wrong: await shown(ask(`/${"0".repeat(64)}`, put)),
        digest: await shown(ask("/photo", put)),
        encoded: await shown(ask("/%E0%A4%A", put)),
        json: await shown(ask("", { method: "POST", body: "no json" })),
    }).toEqual({
        whole: [200, "image/jpeg", DISPOSITION, "a photo"],
        range: [206, "image/jpeg", DISPOSITION, "bytes 2-4/7", "pho"],
        end: [206, "image/jpeg", DISPOSITION, "bytes 4-6/7", "oto"],
        outside: [416, "image/jpeg", DISPOSITION, "bytes */7", ""],
        again: [304, "image/jpeg", DISPOSITION, ""],
        names: [400, 400, 400, 400],
        types: [400, 400, 400],
        head: [200, "image/jpeg", DISPOSITION, ""],
        other: [404, "application/json;charset=utf-8", ""],
        field: [404, "application/json;charset=utf-8", ""],
        beyond: [404, "application/json;charset=utf-8", ""],
        anonymous: [401, "application/json;charset=utf-8", ""],
        wrong: [400, "application/json;charset=utf-8", ""],
        digest: [400, "application/json;charset=utf-8", ""],
        encoded: [400, "application/json;charset=utf-8", ""],
        json: [400, "application/json;charset=utf-8", ""],
    });
});
