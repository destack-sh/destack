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
    triggerType
    projectVersion {
      id
      tag
      name
    }
    deployment {
      id
    }
    user {
      id
      slug
    }
    accessToken {
      id
      name
    }
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
      name
    }
    task {
      id
      name
    }
    code {
      id
      name
    }
  }
`);

export function useExecutions(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string>;
    includeAncestorVersions: Ref<boolean>;
    buildIds: Ref<string[] | null>;
    taskIds: Ref<string[] | null>;
    codeIds: Ref<string[] | null>;
  },
  options: { root: boolean; live?: boolean }
) {
  const { result: executionsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query executions(
        $projectId: GlobalID!
        $projectVersionId: GlobalID
        $includeAncestorVersions: Boolean
        $buildIds: [GlobalID!]
        $taskIds: [GlobalID!]
        $codeIds: [GlobalID!]
        $rootIdNull: Boolean
        $first: Int
        $last: Int
      ) {
        executions(
          projectId: $projectId
          projectVersionId: $projectVersionId
          includeAncestorVersions: $includeAncestorVersions
          buildIds: $buildIds
          taskIds: $taskIds
          codeIds: $codeIds
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
    {
      projectId: filter.projectId,
      projectVersionId: filter.projectVersionId,
      includeAncestorVersions: filter.includeAncestorVersions,
      buildIds: filter.buildIds,
      taskIds: filter.taskIds,
      codeIds: filter.codeIds,
      rootIdNull: options.root,
      first: 25,
    }
  );

  if (options.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription moduleExecutionChanged(
          $projectId: GlobalID!
          $projectVersionId: GlobalID
          $includeAncestorVersions: Boolean
          $buildIds: [GlobalID!]
          $taskIds: [GlobalID!]
          $codeIds: [GlobalID!]
          $rootIdNull: Boolean
        ) {
          moduleExecutionChanged(
            projectId: $projectId
            projectVersionId: $projectVersionId
            includeAncestorVersions: $includeAncestorVersions
            buildIds: $buildIds
            taskIds: $taskIds
            codeIds: $codeIds
            rootIdNull: $rootIdNull
          ) {
            ...ExecutionContent
          }
        }
      `),
      variables: {
        projectId: filter.projectId,
        projectVersionId: filter.projectVersionId,
        includeAncestorVersions: filter.includeAncestorVersions,
        buildIds: filter.buildIds,
        taskIds: filter.taskIds,
        codeIds: filter.codeIds,
        rootIdNull: options.root,
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const execution = useFragment(ExecutionContentType, subscriptionData.data.moduleExecutionChanged);
        // cursor is base64-encoded ExecutionConnection:{nodeId}
        const newEdge = {
          __typename: "ExecutionEdge",
          // not sure what to put here, it's a strawberry internal
          // should probably update all other edges' cursors as well
          cursor: btoa(`arrayconnection:0`),
          node: { ...execution, descendants: [] },
        };

        if (prev?.executions == null) {
          // first execution, return directly
          return {
            executions: {
              totalCount: 1,
              edges: [newEdge],
              pageInfo: {
                hasNextPage: false,
                hasPreviousPage: false,
                startCursor: newEdge.cursor,
                endCursor: newEdge.cursor,
              },
            },
          };
        }

        const index = prev.executions.edges.findIndex((edge) => edge.node.id === execution.id);
        if (index >= 0) {
          // update is automatic in Apollo cache
          return prev;
        }
        // insert into edges if it's new, update count and page info

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
