<script lang="tsx" setup>
import { NodeType } from "@/proto/wire";
import { getChildrenRef } from "@/system/graph";
import { ref } from "vue";

const { children: subviews } = getChildrenRef(ref(null), NodeType.VIEW);
</script>
<template>
  <div>
    hey
  </div>
</template>
