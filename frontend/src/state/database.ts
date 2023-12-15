import { readValue } from "@/components/inputs";
import { graphql } from "@/gql";
import { ConditionalOp, TypeHint, TypeTag, type Conditional, StatementType, QueryEngine, SortOp } from "@/gql/graphql";
import { useCurrentModule, type Field } from "@/state/module";
import type { useFields } from "@/state/statement";
import { TypeStorageFormat, getStorageFormat } from "@/state/type";
import { useDebounceFn } from "@vueuse/core";
import { computed, ref, watch, type Ref } from "vue";

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
      engine
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

export function useDatabaseInlineSearch(fields: ReturnType<typeof useFields>, textQuery: Ref<string | undefined>) {
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
        (f.tag == TypeTag.TypeReference && module.statementOf(f.referenceCk)?.type == StatementType.Choice)
    )
  );
  // TODO @UX: apply inline search to local records immediately/optmistically
  // update search query on inline query change
  const inlineQuery: Ref<Conditional | undefined> = ref(undefined);
  function getInlineQuery() {
    if ((textQuery.value ?? "").trim().length == 0) return undefined;
    const subclauses = [
      ...stringFields.value.map(
        (f) =>
          ({
            op: ConditionalOp.Matches,
            field: "value." + module.getTypedKey(f),
            value: textQuery.value,
          } as Conditional)
      ),
      ...nameFields.value.map(
        (f) =>
          ({
            op: ConditionalOp.StartsWith,
            field: "value." + module.getTypedKey(f),
            value: textQuery.value,
          } as Conditional)
      ),
    ];
    // filter for enum fields members that match the query
    for (const enumField of enumFields.value) {
      const matchingMembers = module
        .statementOf(enumField.referenceCk)
        ?.fields.filter((m) => m.name?.toLowerCase().startsWith(textQuery.value?.toLowerCase() ?? ""));
      if (matchingMembers == null || matchingMembers.length == 0) continue;
      subclauses.push({
        field: "value." + module.getTypedKey(enumField),
        op: ConditionalOp.Equals,
        value: matchingMembers.map((m) => m.key),
      } as Conditional);
    }

    if (subclauses.length == 0) return undefined; // TODO @UX: indicate inline search is not possible if no plausible subclauses
    return { op: ConditionalOp.Or, clauses: subclauses } as Conditional;
  }
  const queryEngine = computed(() => {
    // :QueryEngineSelection
    if ((textQuery.value ?? "").length > 0) return QueryEngine.Opensearch;
    else return QueryEngine.Postgres;
  });

  // update inline query on query change
  function updateInlineQuery() {
    inlineQuery.value = getInlineQuery();
  }
  const updateInlineQueryDebounced = useDebounceFn(updateInlineQuery, 100);
  watch(() => [textQuery.value, stringFields.value, nameFields.value, enumFields.value], updateInlineQueryDebounced, {
    immediate: true,
  });

  return { inlineQuery, queryEngine };
}

export function useDatabaseCombinedSearch(
  fields: ReturnType<typeof useFields>,
  textQuery: Ref<string | undefined>,
  maybeFilters: Ref<Conditional[]>
) {
  const { inlineQuery, queryEngine } = useDatabaseInlineSearch(fields, textQuery);
  const module = useCurrentModule();
  const combinedQuery: Ref<Conditional | undefined> = computed(() => {
    const clauses = [
      ...(maybeFilters.value.filter((c) => {
        const field = module.fieldOf(c?.field as string);
        if (field == null) return false;
        return isConditionalFullySpecified(field, c as Conditional);
      }) ?? []),
      inlineQuery.value,
    ]
      .filter((c) => c != null)
      .map((c) => c as Conditional);
    return combineConditionals(ConditionalOp.And, clauses);
  });
  return { inlineQuery, combinedQuery, queryEngine };
}

export function getDefaultConditional(field: Field): Conditional {
  return { field: field.ck, op: ConditionalOp.Exists };
}

export const CONDITIONAL_OP_NAME: Partial<Record<ConditionalOp, string>> = {
  [ConditionalOp.Exists]: "exists",
  [ConditionalOp.NotExists]: "not exists",
  [ConditionalOp.Equals]: "=",
  [ConditionalOp.NotEquals]: "!=",
  [ConditionalOp.GreaterThan]: ">",
  [ConditionalOp.GreaterThanOrEquals]: ">=",
  [ConditionalOp.LessThan]: "<",
  [ConditionalOp.LessThanOrEquals]: "<=",
  [ConditionalOp.Matches]: "matches",
  [ConditionalOp.StartsWith]: "starts with",
  [ConditionalOp.Contains]: "contains",
  [ConditionalOp.NotContains]: "not contains",
  [ConditionalOp.In]: "in",
  [ConditionalOp.NotIn]: "not in",
  [ConditionalOp.And]: "and",
  [ConditionalOp.Or]: "or",
  [ConditionalOp.Not]: "not",
};

// :ExpressionOps
export const COND_EXACT = [ConditionalOp.Equals, ConditionalOp.NotEquals, ConditionalOp.In, ConditionalOp.NotIn];
export const COND_RANGE = [
  ConditionalOp.GreaterThan,
  ConditionalOp.GreaterThanOrEquals,
  ConditionalOp.LessThan,
  ConditionalOp.LessThanOrEquals,
];
export const COND_VECTOR = [ConditionalOp.Near];
export const COND_STRING = [ConditionalOp.StartsWith, ConditionalOp.Matches];
export const EXPRESSION_OPS = {
  // Conditionals
  COND_STATIC: [ConditionalOp.True, ConditionalOp.False],
  COND_LOGICAL: [ConditionalOp.Not, ConditionalOp.And, ConditionalOp.Or],
  COND_EXACT,
  COND_RANGE,
  COND_COMPARISON: [...COND_EXACT, ...COND_RANGE],
  COND_SET: [ConditionalOp.Contains, ConditionalOp.NotContains],
  COND_EXISTENCE: [ConditionalOp.Exists, ConditionalOp.NotExists],
  COND_VECTOR,
  COND_STRING,
  COND_SCORED: [...COND_VECTOR, ...COND_STRING],
  // Sorts
  SORT: [SortOp.Ascending, SortOp.Descending],
};

// :ExpressionSupport
export const SUPPORTED_OPS_BY_TYPE: Partial<Record<TypeTag | TypeHint | TypeStorageFormat, ConditionalOp[]>> = {
  [TypeStorageFormat.LONG]: [...EXPRESSION_OPS.COND_EXACT, ...EXPRESSION_OPS.COND_RANGE],
  [TypeStorageFormat.DOUBLE]: [...EXPRESSION_OPS.COND_EXACT, ...EXPRESSION_OPS.COND_RANGE],
  [TypeStorageFormat.BOOLEAN]: EXPRESSION_OPS.COND_EXACT,
  [TypeStorageFormat.DATE]: [...EXPRESSION_OPS.COND_EXACT, ...EXPRESSION_OPS.COND_RANGE],
  [TypeStorageFormat.KEYWORD]: EXPRESSION_OPS.COND_EXACT,
  [TypeStorageFormat.VECTOR]: EXPRESSION_OPS.COND_VECTOR,
  [TypeStorageFormat.RELATION]: EXPRESSION_OPS.COND_EXACT,
  [TypeTag.String]: EXPRESSION_OPS.COND_STRING,
  [TypeHint.Name]: [ConditionalOp.StartsWith],
};

export function canSort(field: Field): boolean {
  return [TypeStorageFormat.DATE, TypeStorageFormat.DOUBLE, TypeStorageFormat.LONG, TypeStorageFormat.KEYWORD].includes(
    getStorageFormat(field.tag, field.hint, field.flags) as TypeStorageFormat
  );
}

export function getSupportedConditionalOps(field: Field): ConditionalOp[] {
  const ops: ConditionalOp[] = [];
  for (const op of [
    ...EXPRESSION_OPS.COND_EXISTENCE,
    ...(SUPPORTED_OPS_BY_TYPE[field.tag] ?? []),
    ...(SUPPORTED_OPS_BY_TYPE[field.hint as TypeHint] ?? []),
    ...(SUPPORTED_OPS_BY_TYPE[getStorageFormat(field.tag, field.hint, field.flags) as TypeStorageFormat] ?? []),
  ]) {
    if (!ops.includes(op)) ops.push(op);
  }
  return ops;
}

export function combineConditionals(op: ConditionalOp, clauses: Conditional[]): Conditional | undefined {
  if (clauses.length == 0) return undefined;
  if (clauses.length == 1) return clauses[0];
  return { op, clauses };
}

export function isConditionalFullySpecified(field: Field, conditional: Conditional): boolean {
  /** Whether the conditional has a value set if needed. Used to filter conditionals that were just created but not set yet.  */
  if (EXPRESSION_OPS.COND_COMPARISON.includes(conditional.op) || EXPRESSION_OPS.COND_STRING.includes(conditional.op)) {
    // this feels a bit too hacky
    const value = readValue(field, conditional.value);
    if (typeof value == "string" && (value ?? "").trim().length == 0) return false;
    if (value == null) return false;
    if (Array.isArray(value) && value.length == 0) return false;
  }
  return true;
}
