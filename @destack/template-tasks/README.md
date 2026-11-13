# @destack/template-tasks

Start a team's issue tracker with projects, cycles, labels and a board.

## Overview

`@destack/template-tasks` is a task tracker to fork and grow, after Linear: the space is the team, and everyone who may write in it works its tasks.

It has:

- `task`: a piece of work with a state, a priority, an assignee, a project, a cycle, an estimate and a due time, ordered within its state.
- `project`: a body of work tasks belong to, with a lead, a target and its own state.
- `cycle`: a time box tasks are planned into.
- `label` and `task-label`: tags that sort tasks across projects.
- `tasks` and `board`: the open tasks with a search and a quick create, and a board with a column per state.

Install it, then create a task and move it along:

```sh
destack install @destack/template-tasks --name tasks
destack task create --title "Ship the beta"
destack task <id> update --state started
destack tasks --where "state = 'started'"
```

## Reference

### task

A task moves through `backlog`, `todo`, `started`, `done` and `canceled`, and keeps its place on the board by `position`.

| Field                     | What it is                                         |
| ------------------------- | -------------------------------------------------- |
| `title`, `description`    | what the task is                                   |
| `state`, `priority`       | where it stands and how urgent it is               |
| `assignee`                | the person working on it                           |
| `project`, `cycle`        | the project it belongs to and the cycle it is in   |
| `estimate`, `dueAt`       | its size in points and when it is due              |
