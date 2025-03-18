from bench.language import Action, ActionType, Flow, LinkType, Page, icon, text

AgentFlow = Flow.new(
    "Agent", icon=icon("fas fa-robot"), text=text("The minimal, default Agent Flow.")
)
Start1 = Action.new(ActionType.START, "Start")
Tool1 = Action.new(ActionType.TOOL, "Tool")
Complete1 = Action.new(ActionType.COMPLETE, "Complete")
AgentFlow.actions.extend(Start1, Tool1, Complete1)
Start1.connect(LinkType.REQUIRE, Tool1)
Tool1.connect(LinkType.DECIDE, Tool1)
Tool1.connect(LinkType.DECIDE, Complete1)

FlowPage = Page.new("Flow")
FlowPage.extend(AgentFlow)
