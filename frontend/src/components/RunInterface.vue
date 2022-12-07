<script lang="ts" setup>
import { graphql, useFragment } from "@/gql";
import { SymbolType, type RunCodeOutput } from "@/gql/graphql";
import type { RunConfiguration } from "@/utils/editor";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { useMutation, useQuery } from "@vue/apollo-composable";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{ config: RunConfiguration }>();
const symbol = computed(() => props.config.symbol);

const CodeContentRunType = graphql(/* GraphQL */ `
  fragment CodeContentToRun on Code {
    inputSchema {
      ...SchemaElementContentDeep
    }
    outputSchema {
      ...SchemaElementContentDeep
    }
    parameters {
      name
      type
      schema {
        ...SchemaElementContentDeep
      }
    }
    arguments {
      name
      type
      value
      reference {
        id
        typeNameDeclaration
      }
    }
  }
`);

const { result: resolvedSymbol } = useQuery(
  graphql(/* GraphQL */ `
    query codeToRun($id: GlobalID!) {
      symbol(id: $id) {
        id
        type
        name
        typeNameDeclaration
        content {
          ... on Code {
            ...CodeContentToRun
          }
        }
      }
    }
  `),
  {
    id: symbol.value?.id,
  }
);
const content = computed(() => {
  if (resolvedSymbol.value?.symbol?.type != SymbolType.Code) {
    return null;
  }
  const content = resolvedSymbol.value?.symbol?.content;
  return useFragment(CodeContentRunType, content);
});
const inputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.inputSchema));
const outputSchema = computed(() => useFragment(SchemaElementContentDeepType, content.value?.outputSchema));
const parameters = computed(() => content.value?.parameters ?? []);
const arguments_ = computed(() => content.value?.arguments ?? []);

function getArgument(name: string) {
  return arguments_.value?.find((a) => a.name === name);
}
function isFree(name: string) {
  return !arguments_.value?.find((a) => a.name === name);
}

const freeParameters = computed(() => {
  // parameter is free if there is no argument for it
  return parameters.value?.filter((p) => !arguments_.value?.find((a) => a.name === p.name));
});
const boundParameters = computed(() => {
  // parameter is bound if there is an argument for it
  return parameters.value?.filter((p) => arguments_.value?.find((a) => a.name === p.name));
});

const includeBoundParameters = ref(false);
const visibleParameters = computed(() => {
  if (includeBoundParameters.value) {
    return [...freeParameters.value, ...boundParameters.value];
  }
  return freeParameters.value;
});

const sessionArguments: Ref<Record<string, string>> = ref({});

const lastOutputs: Ref<RunCodeOutput[]> = [];

const { mutate: run, loading: running } = useMutation(
  graphql(/* GraphQL */ `
    mutation run($input: RunCodeInput!) {
      run(input: $input) {
        outputs {
          name
          value
        }
      }
    }
  `)
);

async function runCode() {
  const result = await run({
    input: {
      codeId: symbol.value?.id,
      arguments: Object.keys(sessionArguments.value).map((name) => ({
        name,
        value: sessionArguments.value[name],
      })),
    },
  });
  lastOutputs.value = result.data?.run?.outputs ?? [];
}
</script>
<template>
  <div class="mx-8 my-3 flex flex-col gap-6">
    <div class="relative mx-auto w-full max-w-[1000px] transition-all">
      <!-- Run header -->
      <div class="mx-1 my-1 flex flex-row items-center justify-between">
        <div>
          <span class="text-sm text-gray-900">Run: </span>
          <span class="text-sm tracking-wide text-gray-900">{{ symbol.name }}</span>
        </div>
      </div>
      <!-- Run parameters/arguments -->
      <div class="rounded-sm border bg-white py-2 px-2" v-if="content">
        <!-- Parameters (with argument if available) -->
        <div class="m-2 flex flex-col gap-2">
          <div class="grid grid-cols-4 gap-2" v-for="parameter in visibleParameters" :key="parameter.name">
            <div class="flex flex-row items-baseline gap-1 text-sm text-gray-900">
              <span>{{ parameter.name }}</span>
              <span class="text-gray-500">{{ parameter.type.toLowerCase() }}</span>
            </div>
            <div class="col-span-3 text-sm text-gray-900">
              <!-- Show argument if it's bound -->
              <template v-if="getArgument(parameter.name)">
                <span v-if="getArgument(parameter.name)?.value != null">
                  {{ getArgument(parameter.name)?.value }}
                </span>
                <span class="tracking-wide" v-else-if="getArgument(parameter.name)?.reference != null">
                  {{ getArgument(parameter.name)?.reference?.typeNameDeclaration }}
                </span>
              </template>
              <!-- Show argument editor otherwise -->
              <template v-else>
                <input
                  v-bind="sessionArguments[parameter.name]"
                  :name="parameter.name"
                  :id="parameter.name"
                  class="block w-full rounded-sm border border-transparent text-gray-900 placeholder-gray-400 outline-none focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
                  :placeholder="parameter.name"
                  @keypress.enter="runCode"
                />
              </template>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div class="relative mx-auto w-full max-w-[1000px] transition-all" v-show="!running">
      <!-- Output header -->
      <div class="mx-1 my-1.5 flex flex-row items-center justify-between">
        <div>
          <span class="text-sm text-gray-900">Output: </span>
          <span class="text-sm tracking-wide text-gray-900">{{ outputSchema?.name }}</span>
        </div>
      </div>
      <!-- Output content -->
      <div class="rounded-sm border bg-white py-2 px-2" v-if="content">
        <div class="m-2 flex flex-col gap-2">
          <div class="grid grid-cols-4 gap-2" v-for="output in lastOutputs" :key="output.name">
            <div class="flex flex-row items-baseline gap-1 text-sm text-gray-900">
              <span>{{ output.name }}</span>
              <!-- <span class="text-gray-500">{{ output.type.toLowerCase() }}</span> -->
            </div>
            <div class="col-span-3 text-sm text-gray-900">
              <span>{{ output.value }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
