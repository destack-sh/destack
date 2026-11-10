import { Authentication, Caller } from "@destack/access";
import { Identifier, type UuidEnvironment, present } from "@destack/schema";
import type { Subject } from "@destack/sync";
import type { ExchangeSubject, ExchangeTarget, TokenExchange } from "../gateway.ts";

/** A machine's token exchange that records each exchange and verifies its own numbered tokens. */
export class RecordingTokenExchange implements TokenExchange {
    /** The exchanges made: the installation, what it asked for and where the token is bound. */
    readonly exchanged: [Subject, ExchangeSubject, ExchangeTarget][] = [];
    /** The environment the grants of verified tokens are drawn from. */
    readonly #environment: UuidEnvironment;

    /** Record exchanges, drawing grants from an environment. */
    constructor(environment: UuidEnvironment) {
        this.#environment = environment;
    }

    /** Record an exchange and answer a numbered token. */
    async exchange(
        installation: Subject,
        asked: ExchangeSubject,
        target: ExchangeTarget,
    ): Promise<string> {
        this.exchanged.push([installation, asked, target]);

        return `exchanged-${this.exchanged.length}`;
    }

    /** Verify a numbered token as the subject acting through the installation under its grant. */
    async verify(token: string, target: ExchangeTarget): Promise<Authentication> {
        const [installation, asked] = present(
            this.exchanged[Number(token.slice("exchanged-".length)) - 1],
            `the exchange of ${token}`,
        );

        return {
            caller: {
                ...Caller.of(asked.subject),
                actors: [installation],
                grants:
                    asked.grants.length > 0
                        ? asked.grants
                        : [
                              {
                                  id: Identifier.create("grant", this.#environment),
                                  scope: asked.subject.id,
                                  permissions: [],
                              },
                          ],
            },
            credential: {
                kind: "session",
                id: token,
                audience: target.audience,
                ...(target.scope === undefined ? {} : { scope: target.scope }),
                verifiedAt: 0,
                expiresAt: Number.MAX_SAFE_INTEGER,
            },
        };
    }
}
