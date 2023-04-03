import { graphql, useFragment } from "@/gql";
import type { EvaluationKind, EvaluationScope } from "@/gql/graphql";
import { getUpdatedConnectionQuery } from "@/utils/connection";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, type Ref } from "vue";

export const EvaluationResultContentType = graphql(/* GraphQL */ `
  fragment EvaluationResultContent on EvaluationResult {
    id
    createdAt
    updatedAt
    kind
    scope
    project {
      id
    }
    projectVersion {
      id
    }
    build {
      id
    }
    statement {
      id
    }
    record {
      id
    }
    typeNode {
      id
    }
    selfMetrics
    aggregatedMetrics
  }
`);

export function useEvaluations(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string>;
    scopeIn?: Ref<EvaluationScope[] | null>;
    kindIn?: Ref<EvaluationKind[] | null>;
    buildIdIn?: Ref<string[] | null>;
    systemIdIn?: Ref<string[] | null>;
  },
  options?: { first?: number; live?: boolean; enabled?: Ref<boolean> }
) {
  const { result: evaluationsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query evaluations(
        $projectId: GlobalID!
        $projectVersionId: GlobalID!
        $scopeIn: [EvaluationScope!]
        $kindIn: [EvaluationKind!]
        $buildIdIn: [GlobalID!]
        $systemIdIn: [GlobalID!]
        $first: Int
      ) {
        evaluations(
          projectId: $projectId
          projectVersionId: $projectVersionId
          scopeIn: $scopeIn
          kindIn: $kindIn
          buildIdIn: $buildIdIn
          systemIdIn: $systemIdIn
          first: $first
        ) {
          totalCount
          edges {
            node {
              ...EvaluationResultContent
            }
          }
        }
      }
    `),
    {
      projectId: filter.projectId,
      projectVersionId: filter.projectVersionId,
      scopeIn: filter.scopeIn ?? ref<EvaluationScope[]>(null),
      kindIn: filter.kindIn ?? ref<EvaluationKind[]>(null),
      buildIdIn: filter.buildIdIn ?? ref<string[]>(null),
      systemIdIn: filter.systemIdIn ?? ref<string[]>(null),
      first: options?.first ?? 25,
    },
    {
      enabled: options?.enabled ?? ref(true),
    }
  );

  if (options?.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription evaluationsChanged(
          $projectId: GlobalID!
          $projectVersionId: GlobalID!
          $scopeIn: [EvaluationScope!]
          $kindIn: [EvaluationKind!]
          $buildIdIn: [GlobalID!]
          $systemIdIn: [GlobalID!]
        ) {
          evaluationsChanged(
            projectId: $projectId
            projectVersionId: $projectVersionId
            scopeIn: $scopeIn
            kindIn: $kindIn
            buildIdIn: $buildIdIn
            systemIdIn: $systemIdIn
          ) {
            ...EvaluationResultContent
          }
        }
      `),
      variables: {
        projectId: filter.projectId,
        projectVersionId: filter.projectVersionId,
        scopeIn: filter.scopeIn ?? ref<EvaluationScope[]>([]),
        kindIn: filter.kindIn ?? ref<EvaluationKind[]>([]),
        buildIdIn: filter.buildIdIn ?? ref<string[]>([]),
        systemIdIn: filter.systemIdIn ?? ref<string[]>([]),
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const evaluation = useFragment(EvaluationResultContentType, subscriptionData.data.evaluationsChanged);
        return {
          evaluations: getUpdatedConnectionQuery(evaluation, prev.evaluations, options.first),
        };
      },
    });
  }

  return {
    totalCount: computed(() => evaluationsResult.value?.evaluations?.totalCount ?? 0),
    evaluations: computed(() =>
      evaluationsResult.value?.evaluations?.edges.map((e) => useFragment(EvaluationResultContentType, e.node))
    ),
  };
}
