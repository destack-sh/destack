import { graphql, useFragment } from "@/gql";
import { ExecutionStatus } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
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
    runnable {
      id
      name
    }
  }
`);

// TODO @Performance @Architecture: use single module session subscription for mutations, logs, sessions, executions, etc.
//  also load runtime state (last execution, etc.) and stream live state (filtered, somehow) here for the entire module
export function useSessions(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string | null>;
  },
  options: { live?: boolean }
) {
  // rewrap refs to prevent eager updates
  filter = wrapValueRefs(filter);
  const first = options?.first ?? 25;
  const {
    result: executionsResult,
    subscribeToMore,
    loading,
  } = useQuery(
    graphql(/* GraphQL */ `
      query executions(
        $projectId: GlobalID!
        $projectVersionId: GlobalID
        $includeAncestorVersions: Boolean
        $runnableIds: [GlobalID!]
        $rootIdNull: Boolean
        $first: Int
        $last: Int
      ) {
        executions(
          projectId: $projectId
          projectVersionId: $projectVersionId
          includeAncestorVersions: $includeAncestorVersions
          runnableIds: $runnableIds
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
      runnableIds: filter.runnableIds,
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
          $runnableIds: [GlobalID!]
          $rootIdNull: Boolean
        ) {
          executionsChanged(
            projectId: $projectId
            projectVersionId: $projectVersionId
            includeAncestorVersions: $includeAncestorVersions
            runnableIds: $runnableIds
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
        runnableIds: filter.runnableIds,
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
    loading,
  };
}

function _useModuleExecutions() {
  const bench = useBenchState();
  const projectId = computed(() => bench.projectId);
  const projectVersionId = computed(() => bench.projectVersionId);
  return useSessions(
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

export function isMostlyCached(execution: { duration?: number; cachedDuration?: number }) {
  return (
    execution.duration != null &&
    execution.cachedDuration != null &&
    execution.cachedDuration > execution.duration * 0.8
  );
}

export function getCachedPercentage(execution: { duration?: number | null; cachedDuration?: number | null }) {
  return 100 - ((execution.duration ?? 0) * 100) / (execution.cachedDuration ?? 0);
}
