import { graphql } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Ref } from "vue";

export function useValidName(name: Ref<string | null>) {
  return { valid: computed(() => (name.value?.length ?? 0) >= 2) };
}

export function isValidSlug(slug: string, minLength = 3): boolean {
  return /^[a-z0-9_-]+$/.test(slug ?? "") && (slug.length ?? 0) >= minLength;
}

export function useValidSlug(slug: Ref<string | null>, me?: Ref<{ id: string } | null>, minLength = 3) {
  const valid = computed(() => isValidSlug(slug.value ?? "", minLength));
  const available = computed(() => owner.value?.ownerBySlug == null || owner.value.ownerBySlug?.id == me?.value?.id);
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
      enabled: valid as any,
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
