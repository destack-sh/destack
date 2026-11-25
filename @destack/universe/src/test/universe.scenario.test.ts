import { BasicCredential } from "@destack/environment";
import { PackageId } from "@destack/package";
import { present, schema } from "@destack/schema";
import registryPackage from "@destack/registry/package";
import { Account, group, Placement } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { expect, onTestFinished, test } from "@destack/test";
import { TestMachine } from "./machine.ts";
import { TEST_UNIVERSE, TestUniverse } from "./universe.ts";

test("sign a person up through the mailed code a native client redeems, make an example's calls in their account, and refuse an example it cannot make", async () => {
    // start the universe and sign ada up, as a native client signs a new person in
    await using started = await TestUniverse.open();
    const ada = await started.signIn("ada@example.test");

    // make a group in her personal account through the example's calls, then settle
    await started.given(
        { objects: { account: [{ object: group, method: "create", input: { name: "Friends" } }] } },
        ada,
    );
    await started.simulation.idle();

    // read the group as ada, and refuse a scope or a type the universe does not serve
    const listed = await ada.client(accountPackage).group.list({ scope: ada.account });
    expect(listed.items.map((item) => item.name)).toEqual(["Friends"]);
    await expect(started.given({ objects: { space: [] } }, ada)).rejects.toThrow(
        new TypeError("the test universe serves accounts alone, not the example's 'space'"),
    );
    await expect(
        started.given(
            { objects: { account: [{ object: { name: "ledger" }, method: "create", input: {} }] } },
            ada,
        ),
    ).rejects.toThrow(new TypeError("the test universe serves no object type ledger"));
});

test("place a person's home on their region's managed machine, as the machine's directory replica reads it", async () => {
    // sign ada up and follow the directory as the region's machine
    await using started = await TestUniverse.open();
    const ada = await started.signIn("ada@example.test");
    const machine = await started.machine();
    await started.simulation.idle();

    // read the placement of her home in the replica
    const home = Account.home(ada.account);
    const [placed] = await Placement.find(machine.directory.snapshot, [home]);
    expect(placed).toEqual({ space: home, scope: ada.account, machine: machine.id, epoch: 1 });
});

test("forward Git for a repository link to its account's registry as the caller, challenging a refusal for Basic credentials", async () => {
    // sign ada up, place her home, and answer as its machine, recording what arrives
    await using started = await TestUniverse.open();
    const ada = await started.signIn("ada@example.test");
    const machine = await started.machine();
    await started.simulation.idle();
    const home = Account.home(ada.account);
    const url = new URL(await machine.directory.answering(machine.id, started.clock.now()));
    const received: { readonly url: string; readonly authorization: string | null }[] = [];
    const listener = started.simulation.network.server("registry-machine").http({
        hostname: url.hostname,
        port: Number(url.port || "443"),
        fetch: (request) => {
            received.push({
                url: request.url,
                authorization: request.headers.get("authorization"),
            });

            // refuse a private repository's anonymous caller, as the registry does
            const isRefused =
                request.url.includes("/private.git/") && !request.headers.has("authorization");

            return isRefused ? new Response("sign in", { status: 401 }) : new Response("refs");
        },
    });
    onTestFinished(() => listener.stop(true));

    // ask as git does: anonymously, with ada's account token as a bearer and as a Basic password, with a token the issuer refuses, for a private repository, and for an unknown handle
    const ask = async (path: string, authorization?: string) => {
        const response = await started.universe.fetch(
            new Request(`${TEST_UNIVERSE.origin}${path}`, {
                headers: authorization === undefined ? {} : { authorization },
            }),
        );

        return [response.status, await response.text(), response.headers.get("www-authenticate")];
    };
    const references = `/@${ada.handle}/site.git/info/refs?service=git-upload-pack`;
    const account = (await ada.token()).token;
    const answers = [
        await ask(references),
        await ask(references, `Bearer ${account}`),
        await ask(references, BasicCredential.write("x-access-token", account)),
        await ask(references, "Bearer not-a-token"),
        await ask(`/@${ada.handle}/private.git/info/refs?service=git-upload-pack`),
        await ask("/@nobody/site.git/info/refs?service=git-upload-pack"),
    ];
    const claims = (index: number) => {
        // read a forwarded token's audience, binding and subject
        const token = received[index]?.authorization?.split(" ")[1]?.split(".")[1];
        const payload = schema
            .looseObject({ aud: schema.string(), partition: schema.string(), sub: schema.string() })
            .parse(
                JSON.parse(
                    atob(
                        present(token, "the forwarded token")
                            .replaceAll("-", "+")
                            .replaceAll("_", "/"),
                    ),
                ),
            );

        return { aud: payload.aud, partition: payload.partition, sub: payload.sub };
    };

    // answer through the registry of her home: anonymous as anonymous, her token exchanged for the registry's there in either form, each refusal with the challenge git answers
    const registry = `${url.origin}/spaces/${home}/${registryPackage.id}/git/@${ada.handle}/site.git/info/refs?service=git-upload-pack`;
    const exchanged = { aud: PackageId.resource(registryPackage.id), partition: home, sub: ada.id };
    const challenge = 'Basic realm="destack"';
    expect({
        answers,
        urls: received.map((each) => each.url),
        anonymous: received[0]?.authorization,
        claims: [claims(1), claims(2)],
    }).toEqual({
        answers: [
            [200, "refs", null],
            [200, "refs", null],
            [200, "refs", null],
            [
                401,
                "the token endpoint refused the request: invalid_grant: invalid access token",
                challenge,
            ],
            [401, "sign in", challenge],
            [404, "", null],
        ],
        urls: [registry, registry, registry, registry.replace("/site.git/", "/private.git/")],
        anonymous: null,
        claims: [exchanged, exchanged],
    });
});

test("enroll a person's machine of their account, published at its name through its region's relay, as the directory reads it", async () => {
    // enroll ada's machine in her personal account behind a relay, and follow the directory as it
    await using started = await TestUniverse.open();
    const ada = await started.signIn("ada@example.test");
    const machine = await started.enroll(ada, ada.account, "https://relay-eu.machine.test");
    await started.simulation.idle();

    // locate the machine at its name below her handle
    const name = { machine: TestMachine.nameOf(machine.id), handle: ada.handle };
    expect(await machine.directory.answering(machine.id, started.clock.now())).toBe(
        TEST_UNIVERSE.machine(name),
    );
});

test("redirect an app link to its installation, and refuse a bare handle", async () => {
    // sign ada up
    await using started = await TestUniverse.open();
    const ada = await started.signIn("ada@example.test");
    const open = async (path: string) => {
        const response = await started.universe.fetch(
            new Request(`${TEST_UNIVERSE.origin}${path}`),
        );

        return [response.status, response.headers.get("location"), await response.text()];
    };

    // open her app, and her bare handle, which names no link
    expect([await open(`/@${ada.handle}/home/notes`), await open(`/@${ada.handle}`)]).toEqual([
        [303, `https://notes--home--${ada.handle}.${TEST_UNIVERSE.domains.space}/`, ""],
        [404, null, ""],
    ]);
});
