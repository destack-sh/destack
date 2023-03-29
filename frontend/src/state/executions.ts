import { graphql, useFragment } from "@/gql";
import { getUpdatedConnectionQuery } from "@/utils/connection";
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
  options: { root: boolean; first?: number; live?: boolean }
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
      first: options?.first ?? 25,
    }
  );

  if (options.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription executionsChanged(
          $projectId: GlobalID!
          $projectVersionId: GlobalID
          $includeAncestorVersions: Boolean
          $buildIds: [GlobalID!]
          $taskIds: [GlobalID!]
          $codeIds: [GlobalID!]
          $rootIdNull: Boolean
        ) {
          executionsChanged(
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
        const execution = useFragment(ExecutionContentType, subscriptionData.data.executionsChanged);
        return {
          executions: getUpdatedConnectionQuery({ ...execution, descendants: [] }, prev.executions, options.first),
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
