import { graphql, useFragment } from "@/gql";
import { RunStatus } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { toValueRef, wrapValueRefs } from "@/utils/functools";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

export const RUN_TERMINAL_STATES = [RunStatus.Aborted, RunStatus.Failed, RunStatus.Completed];

export const RunContentType = graphql(/* GraphQL */ `
  fragment RunContent on Run {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    duration
    cachedDuration
    cachedGeneratedAt
    status
    projectVersion {
      id
      tag
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

export function _useModuleSessions(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string | null>;
  },
  options: { live?: boolean }
) {
  // rewrap refs to prevent eager updates
  filter = wrapValueRefs(filter);
}

function _useCurrentSessions() {
  const bench = useBenchState();
  const projectId = computed(() => bench.projectId);
  const projectVersionId = computed(() => bench.projectVersionId);
  return useSessions({ projectId, projectVersionId }, { live: true });
}

export const useCurrentSessions = createSharedComposable(_useCurrentSessions);

export function useSessions(
  filter: {
    runnableIds: Ref<string[] | undefined>;
  },
  options: { live?: boolean } = {}
) {
  // nocheckin
}

export function isMostlyCached(run: { duration?: number; cachedDuration?: number }) {
  return run.duration != null && run.cachedDuration != null && run.cachedDuration > run.duration * 0.8;
}

export function getCachedPercentage(run: { duration?: number | null; cachedDuration?: number | null }) {
  return 100 - ((run.duration ?? 0) * 100) / (run.cachedDuration ?? 0);
}
