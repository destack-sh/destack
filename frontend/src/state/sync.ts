import { graphql } from "@/gql";
import { useClient } from "@/state/client";
import { useSubscription } from "@vue/apollo-composable";
import { watch } from "vue";

export function useModuleSync(projectVersionId: Ref<string | null>) {
  // TODO @Incomplete: implement basic sync

  const client = useClient();
  const {
    onResult: onModuleChanged,
    start,
    stop,
  } = useSubscription(
    graphql(/* GraphQL */ `
      subscription moduleChanged($projectVersionId: GlobalID!) {
        moduleChanged(projectVersionId: $projectVersionId) {
          id
          clientId
          mutations {
            fileId
            input
          }
        }
      }
    `),
    {
      projectVersionId,
    }
  );
  // enable/disable subscription when projectVersionId changes
  watch(
    projectVersionId,
    () => {
      if (projectVersionId.value != null) {
        start();
      } else {
        stop();
      }
    },
    { immediate: true }
  );

  onModuleChanged((result) => {
    console.log("moduleChanged", result.data?.moduleChanged);
  });
}
