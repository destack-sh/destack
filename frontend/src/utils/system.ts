import { graphql } from "@/gql";
import type { SystemInfo } from "@/gql/graphql";
import { VERSION } from "@/utils/globals";
import { isSemVerNewer, parseSemVer, type SemVer } from "@/utils/semver";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import { computed, ref, watch, type Ref } from "vue";

function _useSystemVersioning(checkIntervalMs = 60 * 1000) {
  const { result: systemInfo, refetch } = useQuery(
    graphql(/* GraphQL */ `
      query systemInfo {
        systemInfo {
          version
          gitCommit
        }
      }
    `)
  );
  // auto-check system info every `checkIntervalMs`
  setInterval(refetch, checkIntervalMs);

  const lastSystemInfo: Ref<SystemInfo | null> = ref(null);
  watch(systemInfo, (newSystemInfo) => {
    if (lastSystemInfo.value?.gitCommit != newSystemInfo?.systemInfo.gitCommit) {
      // log if anything changed
      console.group("%cBench Backend", "color:orangered");
      console.info("%cVersion: " + newSystemInfo?.systemInfo.version, "color:orangered");
      console.info("%cGit Commit: " + newSystemInfo?.systemInfo.gitCommit, "color:orangered");
      console.groupEnd();
    }
    lastSystemInfo.value = newSystemInfo?.systemInfo ?? null;
  });

  const outOfDate = computed(() => {
    if (lastSystemInfo.value == null) {
      return undefined;
    }
    // check if backend version is actually newer or just different
    const backendSemVer = parseSemVer(lastSystemInfo.value.version) as SemVer;
    const mySemVer = parseSemVer(VERSION) as SemVer;
    return isSemVerNewer(backendSemVer, mySemVer);
  });

  return { systemInfo: lastSystemInfo, outOfDate };
}

export const useSystemVersioning = createSharedComposable(_useSystemVersioning);
