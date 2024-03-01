import { NodeDataGraph } from "@/language/graph";
import { NodeType } from "@/proto/wire";
import authState from "@/system/auth";

const graph = new NodeDataGraph();
const user = graph.findRootRef(NodeType.USER);
const clients = graph.getChildrenRef(user, NodeType.CLIENT);

const userState = {
  auth: authState,
  graph: graph,
  user,
  clients,
};

export default userState;