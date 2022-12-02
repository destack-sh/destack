<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";

const CodeContentFragment = graphql(/* GraphQL */ `
  fragment CodeContent on Code {
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
type CodeContent = FragmentType<typeof CodeContentFragment>;

const props = defineProps<{ content: CodeContent }>();
const content = useFragment(CodeContentFragment, props.content);
</script>
<template>
  <div>
    <div v-if="!content.code">builtin {{ content.builtinId }}</div>
    <MonacoEditor v-else :model-value="content.code" language="python" />
  </div>
</template>
