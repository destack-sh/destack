import { graphql, useFragment } from "@/gql";
import { ExecutionStatus } from "@/gql/graphql";
import { useBenchState } from "@/state/editor";
import { getUpdatedConnectionQuery } from "@/utils/connection";
import { wrapValueRefs } from "@/utils/functools";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

export const EXECUTION_TERMINAL_STATES = [ExecutionStatus.Aborted, ExecutionStatus.Failed, ExecutionStatus.Completed];

export const ExecutionContentType = graphql(/* GraphQL */ `
  fragment ExecutionContent on Execution {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    duration
    cachedDuration
    cachedGeneratedAt
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
    errorNice {
      type
      message
      traceback {
        line
        filename
        lineno
        name
        locals
      }
    }
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
    taskIds: Ref<string[] | null>;
    codeIds: Ref<string[] | null>;
  },
  options: { root: boolean; first?: number; live?: boolean }
) {
  // rewrap refs to prevent eager updates
  filter = wrapValueRefs(filter);
  const first = options?.first ?? 25;
  const { result: executionsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query executions(
        $projectId: GlobalID!
        $projectVersionId: GlobalID
        $includeAncestorVersions: Boolean
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
      first,
    }
  );

  if (options.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription executionsChanged(
          $projectId: GlobalID!
          $projectVersionId: GlobalID
          $includeAncestorVersions: Boolean
          $taskIds: [GlobalID!]
          $codeIds: [GlobalID!]
          $rootIdNull: Boolean
        ) {
          executionsChanged(
            projectId: $projectId
            projectVersionId: $projectVersionId
            includeAncestorVersions: $includeAncestorVersions
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
        taskIds: filter.taskIds,
        codeIds: filter.codeIds,
        rootIdNull: options.root,
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const execution = useFragment(ExecutionContentType, subscriptionData.data.executionsChanged);
        return {
          executions: getUpdatedConnectionQuery({ ...execution, descendants: [] }, prev.executions, first),
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

function _useModuleExecutions() {
  const bench = useBenchState();
  const projectId = computed(() => bench.currentProjectId);
  const projectVersionId = computed(() => bench.currentProjectVersionId);
  return useExecutions(
    {
      projectId,
      projectVersionId,
      includeAncestorVersions: ref(false),
      taskIds: ref(null),
      codeIds: ref(null),
    },
    { root: true, live: true }
  );
}

export const useModuleExecutions = createSharedComposable(_useModuleExecutions);
