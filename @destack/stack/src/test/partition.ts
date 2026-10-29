import { Account, account, space } from "@destack/account/object";
import { defineObject, field, PackageHandle } from "@destack/object";
import { Package } from "@destack/package";
import { present, schema, type Identifier } from "@destack/schema";
import { ids, TestSpace, type HostedSpace } from "./fixture.ts";

/** The package of the cards and memos the spaces serve in addition to the stack. */
const BOARD = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000006",
    name: "@example/board",
    version: "2026.10.0",
});

/** Cards of a space or of an account, written by whoever writes their scope. */
export const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: [space, account],
        fields: { title: field.string() },
        permissions: ["read", "write"],
    },
    undefined,
    { package: BOARD },
);

/** Notes an account hands down to its spaces, written at the account's home. */
export const memo = defineObject(
    {
        name: "memo",
        plural: "memos",
        scope: [space, account],
        inherited: {},
        fields: { title: field.string() },
        permissions: ["read", "write"],
    },
    undefined,
    { package: BOARD },
);

/** The package of the cards and memos. */
export const board = PackageHandle.of(BOARD, { objects: { card, memo } });

/** Another space of the fixture's account, beside the account's home. */
const OTHER_SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d1");

/** The spaces the partition world hosts: the account's home, then another space of the account. */
const SPACES: readonly HostedSpace[] = [
    { id: Account.home(ids.account), account: ids.account, name: "home" },
    { id: OTHER_SPACE, account: ids.account, name: "other" },
];

/** The template of an account's home and another space of the account, serving the board. */
export const PARTITION_TEMPLATE = TestSpace.template(
    { packages: [board] },
    { name: "partition", value: schema.null(), spaces: SPACES, write: async () => null },
);

/** An account's home and another space of the account, copied from their template. */
export class PartitionFixture {
    /** The account's home. */
    readonly home: TestSpace;
    /** The other space of the account. */
    readonly other: TestSpace;

    /** Store the two spaces. */
    private constructor(home: TestSpace, other: TestSpace) {
        this.home = home;
        this.other = other;
    }

    /** Open the account's home and the other space from their template. */
    static async open(): Promise<PartitionFixture> {
        const { spaces } = await TestSpace.copy(PARTITION_TEMPLATE, { packages: [board] });
        const copied = (id: Identifier<"space">) => present(spaces.get(id), `space ${id}`);

        return new PartitionFixture(copied(Account.home(ids.account)), copied(OTHER_SPACE));
    }
}
