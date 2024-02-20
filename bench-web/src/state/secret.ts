import { graphql } from "@/gql";
import type { Secret } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { useSecretOps } from "@/state/operations/secret";
import { SECRET_TYPENAME } from "@/state/type";
import { useApolloClient } from "@vue/apollo-composable";

export type SecretRecord = {
  _type: typeof SECRET_TYPENAME;
  id: string;
  name?: string | null;
  sha512: string;
};

export function toSecretDataId(id: string) {
  /* From btoa encoded Secret:uuid to uuid */
  return atob(id).split(":")[1];
}

export function toSecretId(id: string) {
  /* From uuid to btoa encoded Secret:uuid */
  return btoa(`Secret:${id}`);
}

function makeSecretRecord(secret: Secret): SecretRecord {
  const record = {
    _type: SECRET_TYPENAME,
    id: toSecretDataId(secret.id),
    name: secret.name ?? null,
    sha512: secret.sha512,
  } as SecretRecord;
  if (!isValidSecretRecord(record)) {
    throw new Error("invalid secret record");
  }
  return record;
}

export function isValidSecretRecord(obj: any): boolean {
  if (obj?._type != SECRET_TYPENAME) {
    return false;
  }
  // check required field types
  for (const field of ["id", "sha512"]) {
    if (typeof obj[field] != "string") {
      return false;
    }
  }
  return true;
}

export function useSecrets() {
  const ops = useSecretOps();
  const bench = useBenchState();
  const client = useApolloClient();

  async function upsert(
    existing: SecretRecord | null,
    updated: { name: string | null; value: any }
  ): Promise<SecretRecord> {
    if (existing == null) {
      const ret = await ops.create(bench.projectId as string, updated.name, updated.value);
      if (ret?.data?.createSecret.__typename != "Secret") {
        throw new Error("secret.create returned non-secret");
      }
      return makeSecretRecord(ret.data.createSecret as Secret);
    } else {
      const ret = await ops.update(toSecretId(existing.id), updated.name, updated.value);
      if (ret?.data?.updateSecret.__typename != "Secret") {
        throw new Error("secret.update returned non-secret");
      }
      return makeSecretRecord(ret.data.updateSecret as Secret);
    }
  }

  async function reveal(secretId: string) {
    const ret = await client.client.query({
      query: graphql(/* GraphQL */ `
        query revealSecret($secretId: GlobalID!) {
          secret(id: $secretId) {
            ... on Secret {
              id
              sha512
              valueRevealed
            }
          }
        }
      `),
      variables: { secretId: toSecretId(secretId) },
      fetchPolicy: "no-cache",
    });
    if (ret.data.secret?.__typename != "Secret") {
      throw new Error("could not get secret");
    }
    return ret.data.secret.valueRevealed;
  }

  return {
    ops,
    upsert,
    reveal,
  };
}
