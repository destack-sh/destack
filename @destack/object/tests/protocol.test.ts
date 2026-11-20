import { argument, plural, t } from "@destack/locale";
import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { defineObject, definePackage, field, shareable } from "../src/index.ts";
import { implementPackage } from "../src/server/index.ts";
import { TestObjectStore, testIdentity } from "../src/test/index.ts";
import { space } from "../src/test/space.ts";
import { users } from "./fixture/user.ts";

/** The ledger protocol: entries any package may serve. */
const LEDGER = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-000000000310",
    name: "@example/ledger",
    version: "2026.9.0",
});

/** The books package, which serves the ledger's entries beside its own audits. */
const BOOKS = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-000000000311",
    name: "@example/books",
    version: "2026.9.0",
});

/** An entry of the ledger protocol. */
const entry = defineObject(
    {
        name: "entry",
        plural: "entries",
        title: t`${plural(argument("count"), { one: "Entry", other: "Entries" })}`,
        scope: space,
        fields: { amount: field.number() },
        traits: [shareable({})],
    },
    (method) => ({ create: method.create("write") }),
    { package: LEDGER },
);

/** An audit the books package keeps of its own. */
const audit = defineObject(
    {
        name: "audit",
        plural: "audits",
        title: t`${plural(argument("count"), { one: "Audit", other: "Audits" })}`,
        scope: space,
        fields: { note: field.string(schema.string()) },
        traits: [shareable({})],
    },
    (method) => ({ create: method.create("write") }),
    { package: BOOKS },
);

/** The ledger protocol, whose entries other packages serve. */
const ledgerPackage = definePackage({ ...testIdentity, objects: [entry] }, { package: LEDGER });

/** The books package, serving the ledger's entries and its own audits. */
const booksPackage = definePackage(
    { ...testIdentity, objects: [audit], protocols: [ledgerPackage] },
    { package: BOOKS },
);

test("reach a protocol's types alone through a connection made for the protocol, refusing the implementer's own", async () => {
    // serve the books with an entry and an audit of alice's
    await using objects = await TestObjectStore.open({
        packages: [implementPackage(booksPackage, { objects: [entry, audit] })],
        tables: entry.tables,
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000312"),
    });
    const alice = objects.as(users.alice);
    const requested = () => RequestId.create(objects.environment);
    await alice
        .client(ledgerPackage, booksPackage)
        .entry.create({ spaceId: objects.scope, requestId: requested(), amount: 12 });
    await alice
        .client(booksPackage)
        .audit.create({ spaceId: objects.scope, requestId: requested(), note: "checked" });

    // connect for the ledger, then list its entries, the books' audits, and describe the store
    const ledger = alice.connect(booksPackage, { scope: objects.scope }, ledgerPackage);
    const entries = await ledger.list(entry, { spaceId: objects.scope });
    const audits = await refusal(ledger.list(audit, { spaceId: objects.scope }));
    const described = await ledger.describe();

    // expect the entry, the audit refused as missing, and the entry described with the scope it lives in
    expect({
        amounts: entries.map((row) => row["amount"]),
        audits,
        described: described.map((type) => type.name),
    }).toEqual({
        amounts: [12],
        audits: ["NOT_FOUND", "TYPE_NOT_FOUND", `type '${BOOKS.id}/audit' not found`],
        described: ["entry", "space"],
    });
});
