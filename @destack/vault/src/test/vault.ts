import { SECRET_DECLARATION } from "@destack/stack/declare";
import type { Crypto, Environment } from "@destack/environment";
import { AccessContext, Caller, type PermissionReference, principal } from "@destack/access";
import * as account from "@destack/account/object";
import type { TestPerson } from "@destack/account/test";
import { and, eq } from "@destack/db";
import { SimulatedClock } from "@destack/environment/simulation";
import type { TestCaller, TestObjectStore } from "@destack/object/test";
import { type BuildReader, Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { RESOURCE_DECLARATION, RESOURCE_KIND_PACKAGE_ID } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import { binding } from "@destack/stack/object";
import { fixtureOutputs, ids, TestSpace, workload } from "@destack/stack/test";
import { type Call, RequestId } from "@destack/sync";
import { single } from "@destack/test";
import { defineVault, type SecretDeclaration, type VaultDeclaration } from "../declare/index.ts";
import { describeSecret, describeVault } from "../inspect/index.ts";
import { secret, secretVersion, vault } from "../object/index.ts";
import vaultPackage from "../package.ts";
import vaultServer from "../server.ts";

/** The application the test vault's space installs, declaring its vaults. */
export const APPLICATION = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c1",
    name: "@example/application",
    version: "2026.10.0",
});

/** A second application, whose installation binds a vault or a secret beside the first's. */
export const CONSUMER = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c2",
    name: "@example/consumer",
    version: "2026.10.0",
});

/** The vault the application declares unless a test gives others. */
export const CREDENTIALS = "credentials";

/** The text value the kit writes as a secret's first version. */
export const CREDENTIAL = { encoding: "text", value: "credential" } as const;

/** The email the member of the space signs up with. */
const MEMBER_EMAIL = "member@example.test";

/** Every vault permission a role may grant, with the read of the space the vaults live in. */
export const VAULT_PERMISSIONS: readonly PermissionReference[] = [
    ...[vault, secret, secretVersion].flatMap((object) =>
        object.grantable.map((name) => ({ packageId: vaultPackage.id, type: object.name, name })),
    ),
    {
        packageId: account.space.policy.definition.packageId,
        type: account.space.name,
        name: "read",
    },
];

/** A secret as calls select it: its scope and identifier. */
export interface SecretKey {
    /** The scope the secret lives in. */
    readonly scope: string;
    /** The secret. */
    readonly id: string;
}

/** A space serving vaults, with an installed application's vaults and a member's role. */
export class TestVault {
    /** The space serving the vault package. */
    readonly space: TestSpace;
    /** The space's object store as its machine serves it. */
    readonly served: TestObjectStore;
    /** The clock the space's servers read, which tests pause. */
    readonly clock: SimulatedClock;
    /** The application's installation. */
    readonly installation: Identifier<"installation">;
    /** The space's owner calling with their client's token, without a fresh second factor. */
    readonly owner: TestCaller;
    /** The callers of people who stepped up in this test, by person. */
    readonly #elevated = new Map<TestPerson, Promise<TestCaller>>();

    /** The world the space's servers run in, its store's. */
    get environment(): Environment {
        return this.served.environment;
    }

    /** Keep a served space with its installed application. */
    private constructor(fields: {
        readonly space: TestSpace;
        readonly served: TestObjectStore;
        readonly clock: SimulatedClock;
        readonly installation: Identifier<"installation">;
    }) {
        // keep the space and how it is served, and call as its owner
        this.space = fields.space;
        this.served = fields.served;
        this.clock = fields.clock;
        this.installation = fields.installation;
        this.owner = fields.space.person(fields.served, fields.space.owner);
    }

    /** Serve the template's space with the application installed. */
    static async open(
        options: {
            /** The vaults the application declares, the credentials vault when absent. */
            readonly vaults?: readonly VaultDeclaration[];
            /** The secrets the application declares. */
            readonly secrets?: readonly SecretDeclaration[];
        } = {},
    ): Promise<TestVault> {
        // serve the vault package in the template's space on its world's clock
        const space = await TestSpace.open({ packages: [vaultPackage] });
        const clock = space.universe.clock;
        if (!(clock instanceof SimulatedClock)) {
            throw new TypeError("the test space runs on the system clock");
        }
        const served = await space.serve({ servers: [vaultServer] });

        // install the application declaring the vaults and secrets
        const { vaults = [TestVault.declare(CREDENTIALS)], secrets = [] } = options;
        const build = await TestVault.build(
            APPLICATION,
            vaults,
            secrets,
            served.environment.crypto,
        );
        const installation = await space.install(served, build);

        return new TestVault({ space, served, clock, installation });
    }

    /** Declare a vault of the application. */
    static declare(name: string): VaultDeclaration {
        return defineVault({ name, spec: {} }, { package: APPLICATION });
    }

    /** Build a package's release declaring some vaults and secrets, which its workload binds. */
    static build(
        released: Package,
        vaults: readonly VaultDeclaration[],
        secrets: readonly SecretDeclaration[],
        crypto: Crypto,
    ): Promise<BuildReader> {
        // run one workload binding every declaration, so its deployment captures them
        const bound = (declared: { readonly name: string }) => ({
            packageId: released.id,
            name: declared.name,
        });
        const outputs = fixtureOutputs({
            server: {
                runtime: "bun",
                emit: true,
                workloads: {
                    main: {
                        ...workload,
                        resources: vaults.map(bound),
                        secrets: secrets.map(bound),
                    },
                },
                views: {},
            },
        });

        // declare the vaults and secrets
        return MemoryBuild.of(released, TestVault.declarations(vaults, secrets), crypto, {
            outputs,
        });
    }

    /** Describe some vaults and secrets as a build declares them, through vault's describers. */
    static declarations(
        vaults: readonly VaultDeclaration[],
        secrets: readonly SecretDeclaration[],
    ) {
        return [
            ...vaults.map((declared) => ({
                kind: RESOURCE_DECLARATION,
                kindPackageId: RESOURCE_KIND_PACKAGE_ID,
                name: declared.name,
                description: describeVault(declared),
            })),
            ...secrets.map((declared) => ({
                kind: SECRET_DECLARATION,
                kindPackageId: vaultPackage.id,
                name: declared.name,
                description: describeSecret(declared),
            })),
        ];
    }

    /** Call the space as a person just stepped up with a second factor, the owner by default. */
    elevated(person = this.space.owner): Promise<TestCaller> {
        // step a person up once, their fresh second factor lasting the test
        const known = this.#elevated.get(person);
        if (known !== undefined) {
            return known;
        }
        const stepped = (async () =>
            this.space.person(this.served, await this.space.universe.elevate(person)))();
        this.#elevated.set(person, stepped);

        return stepped;
    }

    /** Sign the member in, allowed exactly some permissions through the space's stack. */
    async member(permissions: readonly PermissionReference[]): Promise<TestPerson> {
        const person = await this.space.universe.signIn(MEMBER_EMAIL);
        await this.allow(person, permissions);

        return person;
    }

    /** Call the space as an installation of it. */
    installed(installation: Identifier<"installation">): TestCaller {
        return this.served.as(principal.installation.reference(ids.space, installation));
    }

    /** Mutate as an installation in process, as its server calls the vault package. */
    async mutate(installation: Identifier<"installation">, called: Call): Promise<void> {
        const caller = Caller.of(principal.installation.reference(ids.space, installation));
        await this.served.store.mutate(AccessContext.of(caller, this.clock.now()), {
            id: RequestId.create(this.environment),
            calls: [called],
        });
    }

    /** Find the vault the application's binding of a declared vault targets. */
    async vault(name = CREDENTIALS): Promise<Identifier<"vault">> {
        const found = await this.served.database
            .select({ target: binding.table.target })
            .from(binding.table)
            .where(
                and(
                    eq(binding.table.installationId, this.installation),
                    eq(binding.table.name, name),
                ),
            );

        return vault.identifier(single(found).target);
    }

    /** Grant a person exactly some permissions through a role the space's stack declares. */
    async allow(person: TestPerson, permissions: readonly PermissionReference[]): Promise<void> {
        await this.space.apply(this.served, [], {
            roles: { member: { description: "Use the vaults", permissions: [...permissions] } },
            relationships: { member: { role: "member", subject: { id: person.id } } },
        });
    }

    /** Create a secret in the credentials vault with a first version of the kit's value. */
    async createSecret(caller: TestCaller) {
        // create the secret with its first version
        const client = caller.client(vaultPackage);
        const scope = ids.space;
        const created = await client.secret.create({
            scope,
            parentId: await this.vault(),
            name: "first",
            requestId: RequestId.create(this.environment),
        });
        const request = {
            scope,
            parentId: created.id,
            requestId: RequestId.create(this.environment),
            value: CREDENTIAL,
        };
        const version = await client.version.create(request);
        const key = { scope, id: created.id };
        await this.point(caller, key, version.number);

        return { secret: created, key, request, version };
    }

    /** Point a secret's current alias at one of its versions. */
    point(caller: TestCaller, key: SecretKey, version: number) {
        return caller.client(vaultPackage).secret.update({
            ...key,
            requestId: RequestId.create(this.environment),
            currentVersion: version,
        });
    }
}
