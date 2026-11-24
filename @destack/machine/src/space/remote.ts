import { Caller, principal } from "@destack/access";
import { PartitionId } from "@destack/access/token";
import type { Credential } from "@destack/account/client";
import * as accountObject from "@destack/account/object";
import type { ObjectRemote, Push, PushResult, Signer } from "@destack/object";
import objectPackage from "@destack/object/package";
import type { PackageId } from "@destack/package";
import type { JsonValue } from "@destack/schema";
import { Subject } from "@destack/sync";
import type * as sync from "@destack/sync";
import type { Space, SpaceStart } from "./space.ts";

/** A scope's remote at its home, following as the space or an installation, calling as signers. */
export class PartitionRemote implements ObjectRemote {
    /** The space following and calling the scope. */
    readonly #space: Space;
    /** The start whose connections the remote opens. */
    readonly #start: SpaceStart;
    /** The followed and called scope. */
    readonly #scope: string;
    /** The scope's home and partition, located once. */
    #located: Promise<{ readonly home: string; readonly partition: string }> | undefined;
    /** The connections to the home, by package and signer. */
    readonly #connected = new Map<string, ObjectRemote & Disposable>();

    /** Open a scope's remote at its home for a space's start. */
    constructor(space: Space, start: SpaceStart, scope: string) {
        this.#space = space;
        this.#start = start;
        this.#scope = scope;
    }

    /** Follow a subscription as its subject: an installation, else the space. */
    async *stream(subscription: sync.Subscription, signal: AbortSignal): AsyncIterable<sync.Page> {
        yield* (await this.#follower(subscription)).stream(subscription, signal);
    }

    /** Ask the home to decide a subscription again, as its subject. */
    async request(subscription: sync.Subscription): Promise<void> {
        await (await this.#follower(subscription)).request(subscription);
    }

    /** Push mutations to the home as their signer. */
    async push(home: string, push: Push, signal?: AbortSignal): Promise<PushResult> {
        const calls = push.mutations.flatMap((mutation) => mutation.calls);

        return (await this.#signer(calls, push.signer)).push(home, push, signal);
    }

    /** Make a reading call at the home as its signer. */
    async call(scope: string, call: sync.Call, signer?: Signer): Promise<JsonValue> {
        return (await this.#signer([call], signer)).call(scope, call);
    }

    /** Send an event to an object's readers at the home as its signer. */
    async broadcast(
        scope: string,
        object: Omit<sync.ObjectReference, "scope">,
        event: JsonValue,
        signer?: Signer,
    ): Promise<void> {
        await (await this.#signer([], signer)).broadcast(scope, object, event);
    }

    /** Locate the scope: a space at itself, another scope at its account's home partition. */
    #locate(): Promise<{ readonly home: string; readonly partition: string }> {
        this.#located ??= (async () => {
            // locate a space at itself
            if (accountObject.space.identifies(this.#scope)) {
                return { home: this.#scope, partition: this.#scope };
            }

            // locate another scope at its account's home
            const { id } = await accountObject.Account.of(
                this.#space.database.snapshot(),
                this.#scope,
            );

            return { home: accountObject.Account.home(id), partition: id };
        })();
        this.#located.catch(() => (this.#located = undefined));

        return this.#located;
    }

    /** Connect as a subscription's subject: an installation by its keys, else the space. */
    #follower(subscription: sync.Subscription): Promise<ObjectRemote> {
        const { subject } = subscription;
        const space = this.#space;

        return subject !== undefined && principal.installation.is(subject)
            ? this.#connect(Subject.key(subject), space.identityOf(subject))
            : this.#connect("", space.identityOf(space.principal).granted);
    }

    /** Connect as a signer: the space or an installation by its keys, else a person by a grant. */
    async #signer(calls: readonly sync.Call[], signer: Signer | undefined): Promise<ObjectRemote> {
        // sign as the space or an installation by its own keys
        const space = this.#space;
        const subject = signer?.subject ?? space.principal;
        const actors = signer?.actors ?? [];
        const grants = signer?.grants ?? [];
        const isSpace = Subject.same(subject, space.principal);
        const packageId = PartitionRemote.#packageOf(calls);
        if (grants.length === 0 && (isSpace || principal.installation.is(subject))) {
            const credential = isSpace
                ? space.identityOf(subject).granted
                : space.identityOf(subject);

            return this.#connect(Subject.key(subject), credential, packageId);
        }

        // act on behalf of a person or a delegate under the grant the calls need
        const requested = await space.requested(calls);
        const key = JSON.stringify([
            Subject.key(subject),
            actors.map((actor) => Subject.key(actor)),
            grants.map((grant) => grant.id),
            requested,
        ]);

        // connect once per person, grant and request
        const caller = { ...Caller.of(subject), actors, grants };

        return this.#connect(key, space.onBehalfOf(caller, requested), packageId);
    }

    /** Reuse the connection of a package and signer to the home, else open one. */
    async #connect(
        signed: string,
        credential: Credential,
        packageId: PackageId = objectPackage.id,
    ): Promise<ObjectRemote> {
        // reuse the connection of the package and signer
        const key = `${packageId} ${signed}`;
        const known = this.#connected.get(key);
        if (known !== undefined) {
            return known;
        }

        // open one at the home, bound to the partition
        const { home, partition } = await this.#locate();
        const bound = PartitionId.parse(partition);
        const { machine } = this.#space;
        const connection = this.#start.connections.use(
            machine.directory.publisher(
                home,
                (request) => credential.authorize(request, packageId, { partition: bound }),
                machine.environment,
                packageId,
            ),
        );
        this.#connected.set(key, connection);

        return connection;
    }

    /** Read the one package whose mount takes some calls, the object package for none. */
    static #packageOf(calls: readonly sync.Call[]): PackageId {
        // take the one package of the calls
        const packages = new Set(calls.map((call) => call.type.packageId));
        if (packages.size > 1) {
            throw new TypeError(
                `calls of packages '${[...packages].join("', '")}' are invalid: a push takes one mount`,
            );
        }

        return packages.values().next().value ?? objectPackage.id;
    }
}
