from bench.language import (
    Action,
    ActionType,
    Agent,
    ColorType,
    Flow,
    LinkType,
    Page,
    Trigger,
    icon,
    text,
    vector2,
)

# default Flow
BenchFlow = Flow.new(
    "Bench Flow", icon=icon("fas fa-robot"), text=text("The default agentive Flow.")
)
Start1 = Action.new(
    ActionType.START,
    "Start",
    triggers=[Trigger.on_message("Message"), Trigger.on_start("Start")],
    position=vector2(0, 0),
)
Tool1 = Action.new(ActionType.TOOL, "Tool", position=vector2(400, 0))
End1 = Action.new(ActionType.END, "End", position=vector2(800, 0))
BenchFlow.actions.extend(Start1, Tool1, End1)
Start1.connect(LinkType.REQUIRE, Tool1)
Tool1.connect(LinkType.DECIDE, Tool1)
Tool1.connect(LinkType.DECIDE, End1)

# default Agent
BenchAgent = Agent.new(
    "Bench",
    color=ColorType.YELLOW,
    icon=icon("fas fa-layer-group", ColorType.YELLOW),
    text=text(
        """
You are the default Agent in Bench.
You are the smart, chill friend to DM with who thinks ahead, but never pushes or patronizes.
You SHOULD be direct and get to the point. Do not yap or meander.
 (Say what you need to say, don't lead into it.) 
You SHOULD NOT ask questions unless obviously required, let others lead the conversation.
        """
    ),
    main_flow=BenchFlow,
)
BenchFlow.default_agent = BenchAgent

AgentPage = Page.new("Agent")
AgentPage.extend(BenchFlow, BenchAgent)
