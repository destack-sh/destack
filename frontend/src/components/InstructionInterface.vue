<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";

const InstructionContentFragment = graphql(/* GraphQL */ `
  fragment InstructionContent on Instruction {
    id
    builtinId
    code
    parameters {
      name
      type
      schema
    }
    arguments {
      name
      type
      value
      reference {
        id
        nameDotType
      }
    }
  }
`);
type InstructionContent = FragmentType<typeof InstructionContentFragment>;

const props = defineProps<{ content: InstructionContent }>();
const content = useFragment(InstructionContentFragment, props.content);
</script>
<template>
  <div>
    <div v-if="!content.code">builtin {{ content.builtinId }}</div>
    <MonacoEditor v-else :model-value="content.code" language="python" />
  </div>
</template>
