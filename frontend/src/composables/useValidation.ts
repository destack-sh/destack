import { graphql } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Ref } from "vue";

export function useValidName(name: Ref<string | null>) {
  return { valid: computed(() => (name.value?.length ?? 0) >= 2) };
}

export function isValidSlug(slug: string): boolean {
  return /^[a-z0-9_-]{3,}$/.test(slug ?? "") && (slug.length ?? 0) >= 4;
}

export function useValidSlug(slug: Ref<string | null>, me?: Ref<{ id: string } | null>) {
  const valid = computed(() => isValidSlug(slug.value));
  const available = computed(() => owner.value == null || owner.value.ownerBySlug?.id == me?.value?.id);
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
    })) as any,
    {
      enabled: valid,
      // we don't want to cache this to (almost) guarantee that the slug is valid,
      // and to definitely re-fetch ownerBySlug when a slug is created/changed
      fetchPolicy: "no-cache",
    }
  );

  return { valid, available, loading, owner };
}

export function isValidEmail(email: string): boolean {
  // This is not an RFC compliant email address, we just want to filter obvious junk.
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}
