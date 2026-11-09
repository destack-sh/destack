import { PackageHandle } from "@destack/object";
import { defineWorkload } from "@destack/object/workload";
import { hostWorkload } from "@destack/stack/cloudflare";
import { ECHO_PACKAGE } from "./package.ts";

/** The echo package, mounting the route a client opens its WebSocket at. */
export const echo = PackageHandle.of(ECHO_PACKAGE, {});

/** The workload of the echo package. */
export const workload = defineWorkload(
    {
        name: "echo",
        start: async (context) => {
            // label answers with the partition the instance serves, else its installation
            const { server, hibernation } = context;
            if (server === undefined || hibernation === undefined) {
                throw new TypeError("the echo workload runs in a host that leaves sockets open");
            }
            const partition = server.partition ?? server.installation;

            return {
                packages: [
                    {
                        package: echo,
                        router: {},
                        environment: context.environment,
                        // accept a WebSocket under the identifier the client asks for
                        fetch: async (request: Request) => {
                            const id = new URL(request.url).searchParams.get("id") ?? "socket";

                            return hibernation.accept({
                                id,
                                attachment: { partition },
                                protocol: "echo",
                                respond: { request: "ping", response: "pong" },
                            });
                        },
                    },
                ],
                sockets: {
                    // answer each message prefixed with the partition, and list the open sockets on "list"
                    message: async (socket, message) => {
                        const text = new TextDecoder().decode(message);
                        const answer =
                            text === "list"
                                ? JSON.stringify(
                                      (await hibernation.list()).map(
                                          (description) => description.id,
                                      ),
                                  )
                                : `${partition} ${text}`;
                        socket.send(new TextEncoder().encode(answer));
                    },
                    close: async () => {},
                },
            };
        },
    },
    { package: ECHO_PACKAGE },
);

/** The class of the echo workload's partition facets. */
export const Workload = hostWorkload(workload, {});
