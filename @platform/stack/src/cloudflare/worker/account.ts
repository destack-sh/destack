import { createAuthenticator } from "@destack/account/better-auth";
import { accountService } from "@destack/account/service";
import { accountDatabase } from "@destack/account/stack";
import { accountConfiguration, accountWorkload } from "@destack/account/workload";
import { AuditHistory } from "@destack/audit/history";
import { ResourceContext } from "@destack/resource/context";
import type { Alarm } from "@destack/service/control";
import { type DurableState, DurableWorkload } from "@destack/service/cloudflare";
import { PlatformAuthentication } from "../../authentication/index.ts";
import { PlatformAccount } from "../../placement/index.ts";
import { signInMail } from "../../mail/index.ts";
import { type ProcessEnvironment, WorkerProcess } from "./process.ts";

/** The account process's bindings: the universe's sign-in secret and the mail delivering its links and codes. */
export interface AccountEnvironment extends ProcessEnvironment {
    /** The secret protecting the universe's cookies and tokens. */
    readonly DESTACK_SECRET: string;
    /** The email of the operator, invited as the platform organisation's owner. */
    readonly DESTACK_OPERATOR: string;
    /** The SES region sending sign-in mail, empty to print it. */
    readonly DESTACK_MAIL_REGION: string;
    /** The sender of sign-in mail, empty to print it. */
    readonly DESTACK_MAIL_FROM: string;
    /** The AWS access key sending through SES. */
    readonly AWS_ACCESS_KEY_ID?: string;
    /** The AWS secret key sending through SES. */
    readonly AWS_SECRET_ACCESS_KEY?: string;
}

/** The account process's Durable Object, running the account service over its database. */
export class DurableAccount extends DurableWorkload {
    /** Start the account service before the object takes any event. */
    constructor(state: DurableState, environment: AccountEnvironment) {
        super(state, (alarm) => start(environment, alarm));
    }
}

/** Start the account service: sign-in at the issuer's origin over the account database, mailing links and codes through SES. */
async function start(environment: AccountEnvironment, alarm: Alarm): Promise<WorkerProcess> {
    // open the account database, and sign in at the issuer's origin
    const callKey = WorkerProcess.callKey(environment);
    const database = await WorkerProcess.database(environment, accountDatabase);
    const origin = environment.DESTACK_ISSUER;
    const mail = signInMail({
        DESTACK_MAIL_REGION: environment.DESTACK_MAIL_REGION,
        DESTACK_MAIL_FROM: environment.DESTACK_MAIL_FROM,
        AWS_ACCESS_KEY_ID: environment.AWS_ACCESS_KEY_ID,
        AWS_SECRET_ACCESS_KEY: environment.AWS_SECRET_ACCESS_KEY,
    });
    const signIn = createAuthenticator({
        callKey,
        origin,
        trustedOrigins: [origin],
        ipAddress: { ipAddressHeaders: ["cf-connecting-ip"] },
        secret: environment.DESTACK_SECRET,
        database,
        providers: {},
        ...PlatformAccount.pages(origin),
        service: (request) => served.fetch(request),
        sendMagicLink: mail.sendMagicLink,
        sendCode: mail.sendCode,
    });

    // serve the account service at the issuer's paths and below its mount, verifying callers against its own records
    const authentication = new PlatformAuthentication(origin, (request) => served.fetch(request));
    const served: WorkerProcess = await WorkerProcess.start(
        accountWorkload,
        {
            resources: new ResourceContext().bind(
                accountConfiguration,
                PlatformAccount.configuration(signIn, environment.DESTACK_OPERATOR),
            ),
            history: new AuditHistory(database),
            callKey,
            alarm,
            authenticate: (request, audience) =>
                authentication.account(request, audience, signIn, database),
        },
        accountService,
    );

    return served;
}

/** The account process's Worker, handing every request to its Durable Object. */
export default {
    /** Forward a request to the process's Durable Object. */
    fetch(request: Request, environment: AccountEnvironment): Promise<Response> {
        return WorkerProcess.forward(environment, request);
    },
};
