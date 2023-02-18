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

export function useExecutions(projectVersionId: Ref<string>, codeId: Ref<string | null>, live?: boolean) {
  if (live) {
    throw new Error("TODO @Incomplete: implement live executions");
  }
  const { result: executionsResult } = useQuery(
    graphql(/* GraphQL */ `
      query executions($projectVersionId: GlobalID!, $codeId: GlobalID) {
        executions(projectVersionId: $projectVersionId, codeId: $codeId) {
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
    { projectVersionId, codeId }
  );

  return {
    executions: computed(
      () =>
        executionsResult.value?.executions?.edges?.map((edge: any) => useFragment(ExecutionContentType, edge.node)) ??
        []
    ),
  };
}
