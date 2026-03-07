# Workspace Symbol

## Member Search

### Search by member name

Workspace symbol search should return the expected member matches for the query string.

```ds:main.ds
[|class /*logger*/Logger {
    [|/*log*/log(message: string): void {}|]
    [|level: int32|]
}|]
```

```lsp workspace_symbol_query
log
```

```lsp workspace_symbol
log|method|Logger
logger|class
```

## Single-File Churn

### Drop one matching method from workspace search
Workspace symbol search should shrink after one matching method is removed.

```ds:main.ds
[|class /*logger*/Logger {
   [|/*log*/log(message: string): void {}|]
   [|/*login*/login(user: string): void {}|]
}|]
```

```ds:main.ds[1]
[|class /*logger*/Logger {
   [|/*log*/log(message: string): void {}|]
}|]
```

```lsp workspace_symbols log [0]
log|method|Logger
login|method|Logger
logger|class
```

```lsp workspace_symbols log [1]
log|method|Logger
logger|class
```

```lsp workspace_symbols log [2]
log|method|Logger
logger|class
```

### Rename the containing class and keep search results exact
Workspace symbol search should update the container name when the class itself is renamed.

```ds:main.ds
[|class /*logger*/Logger {
   [|/*log*/log(message: string): void {}|]
}|]
```

```ds:main.ds[1]
[|class /*audit_logger*/AuditLogger {
   [|/*log*/log(message: string): void {}|]
}|]
```

```lsp workspace_symbols log [0]
log|method|Logger
logger|class
```

```lsp workspace_symbols log [1]
log|method|AuditLogger
audit_logger|class
```

### Add a second matching class in the same file
Workspace symbol search should include new matching declarations after the overlay grows the file.

```ds:main.ds
[|class /*logger*/Logger {
   [|/*log*/log(message: string): void {}|]
}|]
```

```ds:main.ds[1]
[|class /*logger*/Logger {
   [|/*log*/log(message: string): void {}|]
}|]

[|class /*backup_logger*/BackupLogger {
   [|/*backup_log*/logBackup(message: string): void {}|]
}|]
```

```lsp workspace_symbols log [0]
log|method|Logger
logger|class
```

```lsp workspace_symbols log [1]
log|method|Logger
backup_log|method|BackupLogger
logger|class
backup_logger|class
```

## Multi-File Growth

### Grow workspace search results across three files
Workspace search should add matching symbols as more files introduce them.

```ds:a.ds
[|class /*logger_a*/LoggerA {
   [|/*log_a*/log(message: string): void {}|]
}|]
```

```ds:b.ds
[|class /*other*/Other {
   [|value: int|]
}|]
```

```ds:c.ds
[|class /*helper*/Helper {
   [|value: int|]
}|]
```

```ds:b.ds[1]
[|class /*logger_b*/LoggerB {
   [|/*log_b*/log(message: string): void {}|]
}|]
```

```ds:b.ds[2]
[|class /*logger_b*/LoggerB {
   [|/*log_b*/log(message: string): void {}|]
}|]
```

```ds:c.ds[1]
[|class /*helper*/Helper {
   [|value: int|]
}|]
```

```ds:c.ds[2]
[|class /*logger_c*/LoggerC {
   [|/*log_c*/log(message: string): void {}|]
}|]
```

```lsp workspace_symbols log [0]
log_a|method|LoggerA
logger_a|class
```

```lsp workspace_symbols log [1]
log_a|method|LoggerA
log_b|method|LoggerB
logger_a|class
logger_b|class
```

```lsp workspace_symbols log [2]
log_a|method|LoggerA
log_b|method|LoggerB
log_c|method|LoggerC
logger_a|class
logger_b|class
logger_c|class
```

### Shrink workspace search after two files rename away from the query
Workspace search should drop renamed matches as files move away from the search term.

```ds:a.ds
[|class /*logger_a*/LoggerA {
   [|/*log_a*/log(message: string): void {}|]
}|]
```

```ds:b.ds
[|class /*logger_b*/LoggerB {
   [|/*log_b*/log(message: string): void {}|]
}|]
```

```ds:c.ds
[|class /*logger_c*/LoggerC {
   [|/*log_c*/log(message: string): void {}|]
}|]
```

```ds:b.ds[1]
[|class Backup {
   [|write(message: string): void {}|]
}|]
```

```ds:b.ds[2]
[|class Backup {
   [|write(message: string): void {}|]
}|]
```

```ds:c.ds[1]
[|class /*logger_c*/LoggerC {
   [|/*log_c*/log(message: string): void {}|]
}|]
```

```ds:c.ds[2]
[|class Archive {
   [|write(message: string): void {}|]
}|]
```

```lsp workspace_symbols log [0]
log_a|method|LoggerA
log_b|method|LoggerB
log_c|method|LoggerC
logger_a|class
logger_b|class
logger_c|class
```

```lsp workspace_symbols log [1]
log_a|method|LoggerA
log_c|method|LoggerC
logger_a|class
logger_c|class
```

```lsp workspace_symbols log [2]
log_a|method|LoggerA
logger_a|class
```

### Preserve workspace search while an unrelated file churns
Workspace search should stay exact when unrelated files change repeatedly.

```ds:logger.ds
[|class /*logger*/Logger {
   [|/*log*/log(message: string): void {}|]
}|]
```

```ds:helper.ds
[|class Helper {
   [|value: int|]
}|]
```

```ds:helper.ds[1]
[|class Helper {
   [|value: int|]
   [|extra: int|]
}|]
```

```ds:helper.ds[2]
[|class Helper {
   [|value: int|]
   [|extra: int|]
   [|final: int|]
}|]
```

```lsp workspace_symbols log [0]
log|method|Logger
logger|class
```

```lsp workspace_symbols log [1]
log|method|Logger
logger|class
```

### Grow and prune workspace search across four states
Workspace search should reflect each matching file as it appears and disappears across overlay states.

```ds:a.ds
[|class /*logger_a*/LoggerA {
   [|/*log_a*/log(message: string): void {}|]
}|]
```

```ds:b.ds
[|class Helper {
   [|value: int|]
}|]
```

```ds:c.ds
[|class Utility {
   [|value: int|]
}|]
```

```ds:b.ds[1]
[|class /*logger_b*/LoggerB {
   [|/*log_b*/log(message: string): void {}|]
}|]
```

```ds:a.ds[2]
[|class Archive {
   [|write(message: string): void {}|]
}|]
```

```ds:b.ds[2]
[|class /*logger_b*/LoggerB {
   [|/*log_b*/log(message: string): void {}|]
}|]
```

```ds:c.ds[3]
[|class /*logger_c*/LoggerC {
   [|/*log_c*/log(message: string): void {}|]
}|]
```

```lsp workspace_symbols log [0]
log_a|method|LoggerA
logger_a|class
```

```lsp workspace_symbols log [1]
log_a|method|LoggerA
log_b|method|LoggerB
logger_a|class
logger_b|class
```

```lsp workspace_symbols log [2]
log_b|method|LoggerB
logger_b|class
```

```lsp workspace_symbols log [3]
log_b|method|LoggerB
log_c|method|LoggerC
logger_b|class
logger_c|class
```

## Multi-File Renames

### Grow workspace search across two renames and one new match
Workspace search should track all matching class and method symbols as files rename into the query.

```ds:a.ds
[|class /*logger_a*/LoggerA {
   [|/*log_a*/log(message: string): void {}|]
}|]
```

```ds:b.ds
[|class Backup {
   [|write(message: string): void {}|]
}|]
```

```ds:c.ds
[|class Helper {
   [|value: int|]
}|]
```

```ds:b.ds[1]
[|class /*logger_b*/LoggerB {
   [|/*log_b*/log(message: string): void {}|]
}|]
```

```ds:c.ds[2]
[|class /*logger_c*/LoggerC {
   [|/*log_c*/log(message: string): void {}|]
}|]
```

```ds:b.ds[2]
[|class /*logger_b*/LoggerB {
   [|/*log_b*/log(message: string): void {}|]
}|]
```

```ds:c.ds[1]
[|class Helper {
   [|value: int|]
}|]
```

```lsp workspace_symbols log [0]
log_a|method|LoggerA
logger_a|class
```

```lsp workspace_symbols log [1]
log_a|method|LoggerA
log_b|method|LoggerB
logger_a|class
logger_b|class
```

```lsp workspace_symbols log [2]
log_a|method|LoggerA
log_b|method|LoggerB
log_c|method|LoggerC
logger_a|class
logger_b|class
logger_c|class
```

## File Creation And Removal

### Create one matching file and then remove it again
Workspace search should grow when a new matching file appears and shrink when that file is deleted.

```ds:base.ds
[|class /*logger_base*/LoggerBase {
   [|/*log_base*/log(message: string): void {}|]
}|]
```

```ds:extra.ds[1]
[|class /*logger_extra*/LoggerExtra {
   [|/*log_extra*/log(message: string): void {}|]
}|]
```

```lsp workspace_symbols log [0]
log_base|method|LoggerBase
logger_base|class
```

```lsp workspace_symbols log [1]
log_base|method|LoggerBase
log_extra|method|LoggerExtra
logger_base|class
logger_extra|class
```

```lsp delete_file extra.ds [2]
```

```lsp workspace_symbols log [2]
log_base|method|LoggerBase
logger_base|class
```
