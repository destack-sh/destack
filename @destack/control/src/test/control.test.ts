import { Account, group, Placement } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { DOMAINS, MachineAddress } from "@destack/account/address";
import { expect, test } from "@destack/test";
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

test("show a profile at its handle below the universe's origin, redirect an app link to its installation, and refuse an unknown handle", async () => {
    // sign ada up
    await using universe = await TestControl.open();
    const ada = await universe.signIn("ada@example.test");
    const { name } = await ada
        .client(accountPackage)
        .account.get({ scope: ada.id, id: ada.account });
    const open = async (path: string) => {
        const response = await universe.control.fetch(
            new Request(`https://${DOMAINS.universe}${path}`),
        );

        return [response.status, response.headers.get("location"), await response.text()];
    };

    // open her profile
    expect([
        await open(`/@${ada.handle}`),
        await open(`/@${ada.handle}/home/notes`),
        await open("/@nobody"),
    ]).toEqual([
        [200, null, `<!doctype html><title>${name}</title><h1>${name}</h1><p>@${ada.handle}</p>`],
        [303, `https://notes--home--${ada.handle}.${DOMAINS.space}/`, ""],
        [404, null, ""],
    ]);
});
