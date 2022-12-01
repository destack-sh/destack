<script lang="ts" setup>
import DatasetInterface from "@/components/DatasetInterface.vue";
import InstructionInterface from "@/components/InstructionInterface.vue";
import { graphql, type FragmentType } from "@/gql";
import { SymbolType } from "@/gql/graphql";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const FileHeader = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    name
    createdAt
    updatedAt
  }
`);

const props = defineProps<{ file: FragmentType<typeof FileHeader> }>();

const { result: file } = useQuery(
  graphql(/* GraphQL */ `
    query getFileById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
        name
        definitions {
          id
          name
          type
          nameDotType
          createdAt
          updatedAt
          content {
            ...InstructionContent
            ...DatasetContent
          }
        }
      }
    }
  `),
  () => ({
    fileId: props.file.id,
  })
);

const definitions = computed(() => {
  return file.value?.file?.definitions || [];
});

type DefinitionInterface = {
  component: any;
};

const interfaces: Record<SymbolType, DefinitionInterface> = {
  [SymbolType.Dataset]: {
    component: DatasetInterface,
  },
  [SymbolType.Instruction]: {
    component: InstructionInterface,
  },
};
</script>

<template>
  <div class="m-6 flex flex-col gap-6 overflow-y-scroll">
    <div
      v-for="definition in definitions"
      :key="definition.id"
      class="mx-auto w-full rounded-sm border border-orange-600 bg-white shadow-md shadow-orange-200"
    >
      <span class="m-1 text-sm text-gray-900">
        {{ definition.nameDotType }}
      </span>
      <div class="relative min-h-fit overflow-clip">
        <component
          v-if="interfaces[definition.type]"
          :is="interfaces[definition.type].component"
          :definition="definition"
          :content="definition.content"
        />
        <span v-else> cannot render {{ definition.type }} </span>
      </div>
    </div>
  </div>
</template>
