import { graphql } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Ref } from "vue";

export function useValidName(name: Ref<string | null>) {
  return { valid: computed(() => (name.value?.length ?? 0) >= 2) };
}

export function useValidSlug(slug: Ref<string | null>, me: Ref<{ id: string } | null>) {
  const valid = computed(() => /^[a-z0-9_-]{3,}$/.test(slug.value ?? "") && (slug.value?.length ?? 0 >= 4));
  const available = computed(() => owner.value == null || owner.value.ownerBySlug?.id == me.value?.id);
  const { result: owner, loading } = useQuery(
    graphql(/* GraphQL */ `
      query checkOwnerBySlug($slug: String!) {
        ownerBySlug(slug: $slug) {
          ... on Organization {
            id
          }
          ... on User {
            id
          }
        }
      }
    `),
    computed(() => ({
      slug: slug.value || "",
    })),
    {
      enabled: valid,
    }
  );

  return { valid, available, loading, owner };
}
