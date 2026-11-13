# @destack/template-tasks

Track a team's tasks in projects and cycles, on a board.

## Install

Install the template into a space as `tasks`.

```sh
destack install @destack/template-tasks --name tasks
```

## Work tasks

Create a task, move it along, and list the started ones.

```sh
destack task create --title "Ship the beta"
destack task <id> update --state started
destack tasks --where "state = 'started'"
```

## Prioritise

`priority` is 0 for none, then 1 urgent to 4 low.

```sh
destack task <id> update --priority 1 # urgent
destack tasks --sort priority
```
