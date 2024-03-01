import { NodeDataGraph } from "@/system/graph";
import { NodeType } from "@/proto/wire";
import authState from "@/system/auth";
import { watch } from "vue";

const graph = new NodeDataGraph();
const user = graph.findRootRef(NodeType.USER);
const clients = graph.getChildrenRef(user, NodeType.CLIENT);

const userState = {
  auth: authState,
  graph: graph,
  user,
  clients,


};

watch(
  () => authState.clientAccess.value?.token,
  () => {
    // fetch user data
  },
);

export default userState;
