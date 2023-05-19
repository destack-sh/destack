import { graphql } from "@/gql";
import type { Secret } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { useSecretOps } from "@/state/operations/secret";
import { useApolloClient } from "@vue/apollo-composable";

export type SecretRecord = Pick<Secret, "id" | "createdAt" | "updatedAt" | "sha512" | "name">;

export function useSecrets() {
  const ops = useSecretOps();
  const editor = useEditorState();
  const client = useApolloClient();

  async function upsert(
    existing: SecretRecord | null,
    updated: { name: string | null; value: any }
  ): Promise<SecretRecord> {
    if (existing == null) {
      const ret = await ops.create(editor.currentProjectId as string, updated.name, updated.value);
      if (ret?.data?.createSecret.__typename != "Secret") {
        throw new Error("secret.create returned non-secret");
      }
      return ret?.data?.createSecret;
    } else {
      const ret = await ops.update(existing.id, updated.name, updated.value);
      if (ret?.data?.updateSecret.__typename != "Secret") {
        throw new Error("secret.update returned non-secret");
      }
      return ret?.data?.updateSecret;
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
      variables: { secretId },
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
