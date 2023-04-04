import { graphql, useFragment } from "@/gql";
import { EvaluationKind, EvaluationScope, SymbolType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { buildsOf, useCurrentInterpModule } from "@/state/runtime";
import { getUpdatedConnectionQuery } from "@/utils/connection";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import { computed, ref, type Ref } from "vue";

export const EvaluationResultContentType = graphql(/* GraphQL */ `
  fragment EvaluationResultContent on EvaluationResult {
    id
    createdAt
    updatedAt
    kind
    scope
    project {
      id
    }
    projectVersion {
      id
    }
    build {
      id
    }
    statement {
      id
    }
    record {
      id
    }
    typeNode {
      id
    }
    selfMetrics
    aggregatedMetrics
  }
`);

export function useEvaluations(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string>;
    scopeIn?: Ref<EvaluationScope[] | null>;
    kindIn?: Ref<EvaluationKind[] | null>;
    buildIdIn?: Ref<string[] | null>;
    systemIdIn?: Ref<string[] | null>;
  },
  options?: { first?: number; live?: boolean; enabled?: Ref<boolean> }
) {
  const { result: evaluationsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query evaluations(
        $projectId: GlobalID!
        $projectVersionId: GlobalID!
        $scopeIn: [EvaluationScope!]
        $kindIn: [EvaluationKind!]
        $buildIdIn: [GlobalID!]
        $systemIdIn: [GlobalID!]
        $first: Int
      ) {
        evaluations(
          projectId: $projectId
          projectVersionId: $projectVersionId
          scopeIn: $scopeIn
          kindIn: $kindIn
          buildIdIn: $buildIdIn
          systemIdIn: $systemIdIn
          first: $first
        ) {
          totalCount
          edges {
            node {
              ...EvaluationResultContent
            }
          }
        }
      }
    `),
    {
      projectId: filter.projectId,
      projectVersionId: filter.projectVersionId,
      scopeIn: filter.scopeIn ?? ref(null),
      kindIn: filter.kindIn ?? ref(null),
      buildIdIn: filter.buildIdIn ?? ref(null),
      systemIdIn: filter.systemIdIn ?? ref(null),
      first: options?.first ?? 25,
    } as any,
    {
      enabled: options?.enabled ?? ref(true),
    }
  );

  if (options?.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription evaluationsChanged(
          $projectId: GlobalID!
          $projectVersionId: GlobalID!
          $scopeIn: [EvaluationScope!]
          $kindIn: [EvaluationKind!]
          $buildIdIn: [GlobalID!]
          $systemIdIn: [GlobalID!]
        ) {
          evaluationsChanged(
            projectId: $projectId
            projectVersionId: $projectVersionId
            scopeIn: $scopeIn
            kindIn: $kindIn
            buildIdIn: $buildIdIn
            systemIdIn: $systemIdIn
          ) {
            ...EvaluationResultContent
          }
        }
      `),
      variables: {
        projectId: filter.projectId,
        projectVersionId: filter.projectVersionId,
        scopeIn: filter.scopeIn ?? ref(null),
        kindIn: filter.kindIn ?? ref(null),
        buildIdIn: filter.buildIdIn ?? ref<string[]>([]),
        systemIdIn: filter.systemIdIn ?? ref<string[]>([]),
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const evaluation = useFragment(EvaluationResultContentType, subscriptionData.data.evaluationsChanged);
        return {
          evaluations: getUpdatedConnectionQuery(evaluation, prev.evaluations, options.first),
        };
      },
    });
  }

  return {
    totalCount: computed(() => evaluationsResult.value?.evaluations?.totalCount ?? 0),
    evaluations: computed(() =>
      evaluationsResult.value?.evaluations?.edges.map((e) => useFragment(EvaluationResultContentType, e.node))
    ),
  };
}

function _useCurrentEvaluations() {
  const editor = useEditorState();
  const runtime = useCurrentInterpModule();
  const mainSymbol = computed(() => runtime.moduleIndex.value?.symbolsById[editor.mainSymbolId ?? ""]);
  const mainBuilds = buildsOf(mainSymbol as Ref<{ id: string; parentId: string } | undefined>);
  const projectId = computed(() => editor.currentProjectId as string);
  const projectVersionId = computed(() => editor.currentProjectVersionId as string);

  const globalEvaluations = useEvaluations(
    {
      projectId,
      projectVersionId,
      scopeIn: ref([EvaluationScope.Instruction]),
      kindIn: ref([EvaluationKind.Lint]),
      // no specific system id
      buildIdIn: ref([]), // no specific build
    },
    { live: false, enabled: runtime.connected }
  );

  function getGlobalEvaluation(symbolId?: string) {
    if (globalEvaluations.evaluations.value == null || (globalEvaluations.evaluations.value?.length ?? 0) == 0) {
      return null;
    } else if (symbolId == null) {
      return globalEvaluations.evaluations.value[0];
    } else {
      return globalEvaluations.evaluations.value.find((e) => e.statement?.id == symbolId);
    }
  }

  // TODO @Broken: 25 limit is wrong for evaluations
  // local to a build for scope
  const buildEvaluations = useEvaluations(
    {
      projectId,
      projectVersionId,
      scopeIn: computed(() =>
        mainSymbol.value == null || mainSymbol.value?.symbolType == SymbolType.Build
          ? []
          : [EvaluationScope.Instruction]
      ),
      buildIdIn: computed(() => mainBuilds.value.map((b) => b.id)),
      // no specific system id
    },
    { live: false, enabled: runtime.connected }
  );

  function getBuildEvaluation(buildId: string, symbolId?: string) {
    return buildEvaluations.evaluations.value?.find(
      (e) => e.build?.id == buildId && (symbolId == null || e.statement?.id == symbolId)
    );
  }

  function getBuildEvaluations(symbolId?: string) {
    const buildIds = mainBuilds.value.map((b) => b.id);
    return buildIds.map((buildId) => getBuildEvaluation(buildId, symbolId)).filter((e) => e != undefined);
  }

  return {
    mainSymbol,
    mainBuilds,
    getGlobalEvaluation,
    getBuildEvaluation,
    getBuildEvaluations,
  };
}

export const useCurrentEvaluations = createSharedComposable(_useCurrentEvaluations);
