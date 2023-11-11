import { graphql } from "@/gql";
import { ConditionalOp, TypeHint, TypeTag, type Conditional } from "@/gql/graphql";
import { useCurrentModule } from "@/state/module";
import type { useFields } from "@/state/statement";
import { SubfieldType, TypeStorageFormat, getStorageFormat } from "@/state/type";
import { useDebounceFn } from "@vueuse/core";
import { computed, ref, watch, type Ref } from "vue";

// :QueryFieldPolicies
export const RECORD_SEARCH_QUERY = graphql(/* GraphQL */ `
  query searchRecords(
    $statementId: GlobalID!
    $query: Conditional
    $sort: [Sort!]
    $after: String
    $limit: Int
    $count: Boolean
  ) {
    searchRecords(statementId: $statementId, query: $query, sort: $sort, after: $after, limit: $limit, count: $count) {
      totalCount
      pageInfo {
        hasNextPage
        hasPreviousPage
        startCursor
        endCursor
      }
      edges {
        cursor
        node {
          id
          revision
          createdAt
          updatedAt
          deletedAt
          value
        }
      }
    }
  }
`);

export function useDatabaseInlineSearch(
  fields: ReturnType<typeof useFields>,
  query: Ref<string | undefined>,
  options?: { debounceMs?: number }
) {
  const module = useCurrentModule();

  // find every string-stored field for search
  const stringFields = computed(() =>
    fields.allFields.value.filter((f) => getStorageFormat(f.tag, f.hint, f.flags) == TypeStorageFormat.STRING)
  );
  const nameFields = computed(() => stringFields.value.filter((f) => f.hint == TypeHint.Name));
  const enumFields = computed(() =>
    fields.allFields.value.filter(
      (f) =>
        f.tag == TypeTag.Enum ||
        (f.tag == TypeTag.TypeReference && module.statementOf(f.referenceCk)?.tag == TypeTag.Enum)
    )
  );
  // TODO @UX: apply inline search to local records immediately/optmistically
  // update search query on inline query change
  const inlineQuery: Ref<Conditional | undefined> = ref(undefined);
  function getInlineQuery() {
    if ((query.value ?? "").trim().length == 0) return undefined;
    const subclauses = [
      ...stringFields.value.map(
        (f) =>
          ({
            op: ConditionalOp.Matches,
            key: "value." + module.getTypedKey(f),
            value: query.value,
          } as Conditional)
      ),
      ...nameFields.value.map(
        (f) =>
          ({
            op: ConditionalOp.StartsWith,
            key: "value." + module.getTypedKey(f) + "." + SubfieldType.starts_with,
            value: query.value?.toLowerCase(), // :StartsWithHack
          } as Conditional)
      ),
    ];
    // filter for enum fields members that match the query
    for (const enumField of enumFields.value) {
      const matchingMembers = module
        .statementOf(enumField.referenceCk)
        ?.fields.filter((m) => m.name?.toLowerCase().startsWith(query.value?.toLowerCase() ?? ""));
      if (matchingMembers == null || matchingMembers.length == 0) continue;
      subclauses.push({
        key: "value." + module.getTypedKey(enumField),
        op: ConditionalOp.Equals,
        value: matchingMembers.map((m) => m.key),
      } as Conditional);
    }

    if (subclauses.length == 0) return undefined; // TODO @UX: indicate inline search is not possible if no plausible subclauses
    return { op: ConditionalOp.Or, clauses: subclauses } as Conditional;
  }

  // update inline query on query change
  function updateInlineQuery() {
    inlineQuery.value = getInlineQuery();
  }
  const updateInlineQueryDebounced = useDebounceFn(updateInlineQuery, options?.debounceMs ?? 100);
  watch(() => [query.value, stringFields.value, nameFields.value, enumFields.value], updateInlineQueryDebounced, {
    immediate: true,
  });

  return { inlineQuery };
}
