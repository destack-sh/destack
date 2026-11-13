import { Account, account, space } from "@destack/account/object";
import { defineObject, field, Package } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { testBuild, type TestObjectStore } from "@destack/object/test";
import { testHost } from "@destack/test/host";
import { TestPackage } from "./package.ts";
import { PackageVersion } from "@destack/package";
import { present, schema, type Identifier } from "@destack/schema";
import { ids, TestSpace, type HostedSpace } from "./space.ts";
import { propagated } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The package of the cards and memos the spaces serve in addition to the stack. */
const BOARD = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-000000000006",
    name: "@example/board",
    version: "2026.10.0",
});

/** Cards of a space or of an account, written by whoever writes their scope. */
export const card = defineObject(
    {
        name: "card",
        plural: "cards",
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: [space, account],
        fields: { title: field.string() },
        access: {
            permissions: ["read", "write"],
        },
    },
    undefined,
    { package: BOARD },
);

/** Notes an account propagates to its spaces, written at the account's home. */
export const memo = defineObject(
    {
        name: "memo",
        plural: "memos",
        title: t`${plural(argument("count"), { one: "Memo", other: "Memos" })}`,
        scope: [space, account],
        fields: { title: field.string() },
        access: {
            permissions: ["read", "write"],
        },
        traits: [propagated({})],
    },
    undefined,
    { package: BOARD },
);

/** The package of the cards and memos. */
export const board = Package.of(BOARD, { objects: [card, memo] });

/** The board as a test space installs it: linked and served in process. */
export const BOARD_PACKAGE: TestPackage = {
    handle: board,
    build: await testBuild(board, testHost().crypto),
    server: implementPackage(board, { objects: [] }),
    connectors: [],
    providers: [],
};

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

/** An account's home and another space of the account, each serving the board. */
export class TestPartition {
    /** The account's home. */
    readonly home: TestSpace;
    /** The other space of the account. */
    readonly other: TestSpace;
    /** The home's served store, with the board installed. */
    readonly homed: TestObjectStore;
    /** The other space's served store, with the board installed. */
    readonly served: TestObjectStore;

    /** Keep the two spaces and their served stores. */
    private constructor(fields: Pick<TestPartition, "home" | "other" | "homed" | "served">) {
        // keep the spaces and their stores
        this.home = fields.home;
        this.other = fields.other;
        this.homed = fields.homed;
        this.served = fields.served;
    }

    /** Open the account's home and the other space from their template, each serving the board. */
    static async open(): Promise<TestPartition> {
        // copy both spaces, then install the board in each as its machine serves it
        const { spaces } = await TestSpace.copy(
            PARTITION_TEMPLATE,
            TestPackage.options([BOARD_PACKAGE]),
        );
        const copied = (id: Identifier<"space">) => present(spaces.get(id), `space ${id}`);
        const home = copied(Account.home(ids.account));
        const other = copied(OTHER_SPACE);

        return new TestPartition({
            home,
            other,
            homed: await TestPartition.#installed(home),
            served: await TestPartition.#installed(other),
        });
    }

    /** Serve a space with the board installed in process, served again once installed. */
    static async #installed(hosted: TestSpace): Promise<TestObjectStore> {
        // install the board as the space's owner, then serve its installed set
        await hosted.install(await hosted.serve(), BOARD_PACKAGE.build, { isolation: "host" });

        return hosted.serve();
    }
}
