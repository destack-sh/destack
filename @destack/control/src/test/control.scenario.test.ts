import { Snapshot } from "@destack/db";
import { PackageId } from "@destack/package";
import { present, schema } from "@destack/schema";
import registryPackage from "@destack/registry/package";
import { Account, group, Placement } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { DOMAINS, MachineAddress } from "@destack/account/address";
import { expect, onTestFinished, test } from "@destack/test";
import { TestMachine } from "./machine.ts";
import { TestControl } from "./control.ts";

test("sign a person up through the mailed code a native client redeems, make an example's calls in their account, and refuse an example it cannot make", async () => {
    // start the universe and sign ada up, as a native client signs a new person in
    await using universe = await TestControl.open();
    const ada = await universe.signIn("ada@example.test");

    // make a group in her personal account through the example's calls, then settle
    await universe.given(
        { objects: { account: [{ object: group, method: "create", input: { name: "Friends" } }] } },
        ada,
    );
    await universe.simulation.idle();

    // read the group as ada, and refuse a scope or a type the universe does not serve
    const listed = await ada.client(accountPackage).group.list({ scope: ada.account });
    expect(listed.items.map((item) => item.name)).toEqual(["Friends"]);
    await expect(universe.given({ objects: { space: [] } }, ada)).rejects.toThrow(
        new TypeError("the test universe serves accounts alone, not the example's 'space'"),
    );
    await expect(
        universe.given(
            { objects: { account: [{ object: { name: "ledger" }, method: "create", input: {} }] } },
            ada,
        ),
    ).rejects.toThrow(new TypeError("the test universe serves no object type ledger"));
});

test("place a person's home on their residency's cloud machine, as the machine's directory replica reads it", async () => {
    // sign ada up and follow the directory as the residency's machine
    await using universe = await TestControl.open();
    const ada = await universe.signIn("ada@example.test");
    const machine = await universe.machine();
    await universe.simulation.idle();

    // read the placement of her home in the replica
    const home = Account.home(ada.account);
    const [placed] = await Placement.find(machine.directory.snapshot, [home]);
    expect(placed).toEqual({ space: home, scope: ada.account, machine: machine.id, epoch: 1 });
});

test("forward Git for a repository link to its account's registry as the caller, refusing a token the issuer refuses and an unknown handle", async () => {
    // sign ada up, place the residency's home, and answer as its machine, recording what arrives
    await using universe = await TestControl.open();
    const ada = await universe.signIn("ada@example.test");
    const machine = await universe.machine();
    await universe.simulation.idle();
    const home = await Account.locate(Snapshot.live(universe.control.store.database), ada.account);
    const url = new URL(await machine.directory.answering(machine.id, universe.clock.now()));
    const received: { readonly url: string; readonly authorization: string | null }[] = [];
    const listener = universe.simulation.network.server("registry-machine").http({
        hostname: url.hostname,
        port: Number(url.port || "443"),
        fetch: (request) => {
            received.push({
                url: request.url,
                authorization: request.headers.get("authorization"),
            });

            return new Response("refs");
        },
    });
    onTestFinished(() => listener.stop(true));

    // ask as git does: anonymously, with ada's account token, with a token the issuer refuses, and for an unknown handle
    const ask = async (path: string, token?: string) => {
        const response = await universe.control.fetch(
            new Request(`https://${DOMAINS.universe}${path}`, {
                headers: token === undefined ? {} : { authorization: `Bearer ${token}` },
            }),
        );

        return [response.status, await response.text()];
    };
    const references = `/@${ada.handle}/site.git/info/refs?service=git-upload-pack`;
    const answers = [
        await ask(references),
        await ask(references, (await ada.token()).token),
        await ask(references, "not-a-token"),
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

    // answer through the registry of her home: anonymous as anonymous, her token exchanged for the registry's there
    const registry = `${url.origin}/spaces/${home}/${registryPackage.id}/git/@${ada.handle}/site.git/info/refs?service=git-upload-pack`;
    expect({
        answers,
        urls: received.map((each) => each.url),
        anonymous: received[0]?.authorization,
        claims: claims(1),
    }).toEqual({
        answers: [
            [200, "refs"],
            [200, "refs"],
            [401, "the token endpoint refused the request: invalid_grant: invalid access token"],
            [404, ""],
        ],
        urls: [registry, registry],
        anonymous: null,
        claims: { aud: PackageId.resource(registryPackage.id), partition: home, sub: ada.id },
    });
});

test("enroll a person's machine of their account, published at its name through its residency's relay, as the directory reads it", async () => {
    // enroll ada's machine in her personal account behind a relay, and follow the directory as it
    await using universe = await TestControl.open();
    const ada = await universe.signIn("ada@example.test");
    const machine = await universe.enroll(ada, ada.account, "https://eu.relay.test");
    await universe.simulation.idle();

    // locate the machine at its name below her handle
    const name = { machine: TestMachine.nameOf(machine.id), handle: ada.handle };
    expect(await machine.directory.answering(machine.id, universe.clock.now())).toBe(
        `https://${MachineAddress.format(name, DOMAINS.machine)}`,
    );
});

test("redirect an app link to its installation, and refuse a bare handle", async () => {
    // sign ada up
    await using universe = await TestControl.open();
    const ada = await universe.signIn("ada@example.test");
    const open = async (path: string) => {
        const response = await universe.control.fetch(
            new Request(`https://${DOMAINS.universe}${path}`),
        );

        return [response.status, response.headers.get("location"), await response.text()];
    };

    // open her app, and her bare handle, which names no link
    expect([await open(`/@${ada.handle}/home/notes`), await open(`/@${ada.handle}`)]).toEqual([
        [303, `https://notes--home--${ada.handle}.${DOMAINS.space}/`, ""],
        [404, null, ""],
    ]);
});
