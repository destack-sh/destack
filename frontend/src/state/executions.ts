import { graphql, useFragment } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Ref } from "vue";

export const ExecutionContentType = graphql(/* GraphQL */ `
  fragment ExecutionContent on Execution {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    status
    # note: do not query for non-id fields on root/parent here since
    # they may not be available when streamed directly from the runtime
    root {
      id
    }
    parent {
      id
    }
    inputs
    outputs
    error
    build {
      id
    }
    task {
      id
    }
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
  buildId: Ref<string | null>,
  taskId: Ref<string | null>,
  codeId: Ref<string | null>,
  options: { root: boolean; live?: boolean }
) {
  const { result: executionsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query executions(
        $projectVersionId: GlobalID!
        $buildId: GlobalID
        $taskId: GlobalID
        $codeId: GlobalID
        $rootIdNull: Boolean
        $first: Int
        $last: Int
      ) {
        executions(
          projectVersionId: $projectVersionId
          buildId: $buildId
          taskId: $taskId
          codeId: $codeId
          rootIdNull: $rootIdNull
          first: $first
          last: $last
        ) {
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
    { projectVersionId, buildId, taskId, codeId, rootIdNull: options.root, first: 25 }
  );

  if (options.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription moduleExecutionChanged(
          $projectVersionId: GlobalID!
          $buildId: GlobalID
          $taskId: GlobalID
          $codeId: GlobalID
          $rootIdNull: Boolean
        ) {
          moduleExecutionChanged(
            projectVersionId: $projectVersionId
            buildId: $buildId
            taskId: $taskId
            codeId: $codeId
            rootIdNull: $rootIdNull
          ) {
            ...ExecutionContent
          }
        }
      `),
      variables: { projectVersionId, codeId, rootIdNull: options.root },
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
          // not sure what to put here, it's a strawberry internal
          // should probably update all other edges' cursors as well
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
