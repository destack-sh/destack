import { graphql, useFragment } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Ref, reactive } from "vue";

export const ExecutionContentType = graphql(/* GraphQL */ `
  fragment ExecutionContent on Execution {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    status
    root {
      id
    }
    parent {
      id
    }
    inputs
    outputs
    error
    code {
      id
    }
    model {
      id
    }
  }
`);

export function useExecutions(
  projectVersionId: Ref<string>,
  codeId: Ref<string | null>,
  options: { root: boolean; live?: boolean }
) {
  const { result: executionsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query executions($projectVersionId: GlobalID!, $codeId: GlobalID, $first: Int, $last: Int) {
        executions(projectVersionId: $projectVersionId, codeId: $codeId, first: $first, last: $last) {
          totalCount
          edges {
            cursor
            node {
              ...ExecutionContent
              descendants {
                ...ExecutionContent
              }
            }
          }
          pageInfo {
            hasNextPage
            hasPreviousPage
            startCursor
            endCursor
          }
        }
      }
    `),
    { projectVersionId, codeId, first: 25 }
  );

  if (options.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription moduleExecutionChanged($projectVersionId: GlobalID!) {
          moduleExecutionChanged(projectVersionId: $projectVersionId) {
            ...ExecutionContent
          }
        }
      `),
      variables: { projectVersionId, codeId },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const execution = useFragment(ExecutionContentType, subscriptionData.data.moduleExecutionChanged);
        const index = prev.executions.edges.findIndex((edge) => edge.node.id === execution.id);
        if (index >= 0) {
          // update is automatic in Apollo cache
          return prev;
        }
        // insert into edges if it's new, update count and page info
        // cursor is base64-encoded ExecutionConnection:{nodeId}
        const newEdge = {
          __typename: "ExecutionEdge",
          cursor: btoa(`arrayconnection:0`),
          node: { ...execution, descendants: [] },
        };
        const pageInfo = {
          ...prev.executions.pageInfo,
          startCursor: newEdge.cursor,
          hasPreviousPage: false,
        };
        return {
          executions: {
            ...prev.executions,
            totalCount: (prev.executions.totalCount ?? 0) + 1,
            edges: [newEdge, ...prev.executions.edges],
            pageInfo,
          },
        };
      },
    });
  }

  return {
    totalCount: computed(() => executionsResult.value?.executions?.totalCount ?? 0),
    executions: computed(
      () =>
        executionsResult.value?.executions?.edges?.map((edge: any) => useFragment(ExecutionContentType, edge.node)) ??
        []
    ),
  };
}
