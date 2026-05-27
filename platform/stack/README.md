# Stack

The single package that defines and deploys the operated Destack platform.

## Hosting

The first production target is `destack.sh`.
SST deploys the public site to AWS S3 and CloudFront, while Cloudflare owns DNS and redirect records for the Destack domains.

## GitHub Environment

Production deploys run through the `production` GitHub environment.
The workflow uses GitHub OIDC to assume an AWS role, so no long-lived AWS keys are stored in GitHub.

Required environment variables:

```sh
AWS_ROLE_ARN=arn:aws:iam::<account>:role/destack-platform-github
```

Optional environment variables:

```sh
AWS_REGION=eu-central-2
```

`AWS_REGION` defaults to `eu-central-2`, the AWS Zurich region.

Required environment secrets:

```sh
CLOUDFLARE_API_TOKEN=...
CLOUDFLARE_DEFAULT_ACCOUNT_ID=...
```

## AWS Role

Create an IAM OIDC provider for `https://token.actions.githubusercontent.com`.
Then create a deploy role that trusts only this repository and the `production` environment.

The trust policy should have this shape:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Principal": {
                "Federated": "arn:aws:iam::<account>:oidc-provider/token.actions.githubusercontent.com"
            },
            "Action": "sts:AssumeRoleWithWebIdentity",
            "Condition": {
                "StringEquals": {
                    "token.actions.githubusercontent.com:aud": "sts.amazonaws.com",
                    "token.actions.githubusercontent.com:sub": "repo:destack-sh/destack:environment:production"
                }
            }
        }
    ]
}
```

Use a dedicated AWS account for the operated Destack platform.
Until the generated SST resource policy is stabilized, attach administrator permissions to this deploy role inside that dedicated account.

## Cloudflare Token

Create one Cloudflare API token for the deploy workflow.
The token needs zone read and DNS edit access for every `destack.*` zone managed by the stack.

## Local Commands

Run these from `platform/stack`.

```sh
bun run diff
bun run deploy
```
