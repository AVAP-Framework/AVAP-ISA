# AVAP Language Specification

**Version 1.0.0**
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

---

## Overview

AVAP (API Value Access Protocol) is a declarative, command-based language for defining API endpoint logic. Programs are sequences of commands that read HTTP request parameters, manipulate variables, call external services, and produce structured results.

AVAP programs are:
- **Compiled** — source code is compiled to AVBC bytecode by the Definition Server
- **Sandboxed** — executed by the Platon VM kernel with timeout and instruction limits
- **Stateless** — each execution starts with a clean variable namespace
- **Language-agnostic** — the underlying ISA supports any language that compiles to AVBC

---

## Program Structure

An AVAP program is a sequence of command invocations, one per line:

```
command(arg1, arg2, ...)
```

### Comments

```
// This is a comment
```

### Variable assignment (inline)

```
result = someCommand(arg1, arg2)
```

---

## Type System

AVAP is dynamically typed. All values belong to one of the following runtime types:

| Type | Description | Example |
|---|---|---|
| `null` | Absence of value | `None` |
| `bool` | Boolean | `true`, `false` |
| `int` | 64-bit signed integer | `42`, `-7` |
| `float` | 64-bit IEEE 754 | `3.14` |
| `string` | UTF-8 text | `"hello"` |
| `list` | Ordered sequence | `[1, 2, 3]` |
| `dict` | Key-value mapping | `{"a": 1}` |

### Parameter type annotations

Command parameters carry type hints that describe how the argument is resolved:

| Annotation | Meaning |
|---|---|
| `variable` | Name of an existing runtime variable |
| `value` | Literal value or variable reference, resolved at runtime |
| `var` | Alias for `variable` |

---

## Variable Scope

All variables live in a single flat namespace (`conector.variables`) for the duration of a request execution. Variables are:

- Created by `addVar`, `addParam`, or any command with a `TargetVariable` parameter
- Read by any subsequent command that references the variable name
- Exported to the response by `addResult`

There is no lexical scoping. Variables set inside `if` blocks or loops are visible outside them.

---

## Command Reference

### Variables

#### `addVar(targetVarName, varValue)`

Declares or updates a runtime variable with intelligent type resolution:

1. If `varValue` is the name of an existing variable → uses that variable's value
2. If `varValue` is a numeric string → converts to `int`
3. If `varValue` is an `int` or `float` → stores as-is
4. Otherwise → stores as string literal

```
addVar(total, 0)
addVar(name, "Alice")
addVar(copy, existingVar)
```

#### `addResult(sourceVariable)`

Exports a variable to the API response. The variable is copied from the runtime namespace to the response results object under the same key.

```
addVar(score, 98)
addResult(score)
// Response: { "score": 98 }
```

#### `addParam(param, variable)`

Reads an HTTP request parameter by name and stores it in a variable. Searches in order: query string → JSON body → form data. Performs type coercion (numeric strings → int/float).

```
addParam(userId, uid)
addParam(page, pageNum)
```

---

### Control Flow

#### `if(variable, variableValue, comparator) ... else() ... end()`

Conditional branching. Both `variable` and `variableValue` are resolved from the runtime namespace if they exist as variable names, otherwise used as literals.

**Comparators:** `=`, `==`, `!=`, `<`, `>`, `<=`, `>=`

```
if(status, active, =)
  addResult(data)
else()
  addVar(error, account_inactive)
  addResult(error)
end()
```

`else()` is optional. `end()` is required.

#### `startLoop(varName, from, to) ... endLoop()`

Counted loop from `from` to `to` inclusive, incrementing by 1. Both bounds can be variable references or literals.

```
startLoop(i, 1, 5)
  addVar(squared, i)
  addResult(squared)
endLoop()
```

---

### Error Handling

#### `try() ... exception(error) ... end()`

Wraps commands in an error handler. If any command in the `try` block raises an error, execution jumps to the `exception` handler. The error message is stored in the given variable and in `__last_error__`.

```
try()
  RequestGet(riskyUrl, , , result)
exception(errMsg)
  addVar(result, request_failed)
end()
addResult(result)
```

---

### HTTP

#### `RequestGet(url, querystring, headers, o_result)`

HTTP GET. Raises on 4xx/5xx. Auto-parses JSON responses.

```
addVar(endpoint, https://api.example.com/users)
RequestGet(endpoint, , , users)
addResult(users)
```

#### `RequestPost(url, querystring, headers, body, o_result)`

HTTP POST. Auto-detects JSON vs form body. Auto-parses JSON responses.

```
AddvariableToJSON(name, Alice, body)
AddvariableToJSON(role, admin, body)
RequestPost(endpoint, , , body, response)
addResult(response)
```

---

### Database

#### `ormDirect(prompt, TargetVariable)`

Executes raw SQL. Supports backtick template interpolation:

```
addParam(userId, uid)
addVar(sql, `SELECT * FROM users WHERE id = ${uid}`)
ormDirect(sql, user)
addResult(user)
```

SELECT → list of row dicts. DML → `"Success: N"` string.

---

### Cryptography

#### `encodeSHA256(SourceVariable, TargetVariable)`
#### `encodeMD5(SourceVariable, TargetVariable)`
#### `randomString(Pattern, Length, TargetVariable)`

```
addVar(password, secret)
encodeSHA256(password, hash)
addResult(hash)

randomString([A-Za-z0-9], 16, token)
addResult(token)
```

---

### String

#### `replace(SourceVariable, rePattern, newValue, TargetVariable)`

Regex substitution. Pattern spaces are treated as `\s`.

#### `getRegex(SourceVariable, rePattern, TargetVariable)`

Extracts all matches, joins them. Returns `null` if no match.

---

### DateTime

#### `getDateTime(Format, TimeDelta, TimeZone, TargetVariable)`

Current datetime. Empty `Format` → Unix timestamp (float).

```
getDateTime(%Y-%m-%d, 0, UTC, today)
getDateTime(, 86400, , tomorrowTs)
```

#### `getTimeStamp(DateString, Format, TimeDelta, TargetVariable)`

Parse date string → Unix timestamp.

#### `stampToDatetime(timestamp, Format, TimeDelta, TargetVariable)`

Unix timestamp → formatted string.

---

### Collections

#### `getListLen(SourceVariable, TargetVariable)`
#### `itemFromList(SourceVariable, index, TargetVariable)`
#### `variableToList(element, TargetVariable)`

```
variableToList(item1, myList)
variableToList(item2, myList)
getListLen(myList, count)
itemFromList(myList, 0, first)
addResult(first)
```

---

### JSON

#### `AddvariableToJSON(Key, Value, TargetVariable)`

Creates or updates a key in a dict variable. Supports backtick templates:

```
AddvariableToJSON(id, `user_${userId}`, payload)
```

#### `variableFromJSON(SourceVariable, key, TargetVariable)`

Reads a key from a dict variable.

---

## Request

#### `getQueryParamList(TargetVariable)`

Returns all query parameters as `[{key: value}, ...]`.

---

## Reserved Variables

| Variable | Description |
|---|---|
| `__last_error__` | Set by `try/exception` — contains the last caught error message |

---

## Template Strings

Several commands support backtick template interpolation for constructing dynamic values:

```
addVar(userId, 42)
addVar(query, `SELECT * FROM orders WHERE user_id = ${userId}`)
ormDirect(query, orders)
```

Syntax: `` `literal text ${variableName} more text` ``

---

## Full Example

```
// Read and validate input
addParam(userId, uid)
addParam(format, fmt)
addVar(fmt, %Y-%m-%d)

// Query database
addVar(sql, `SELECT * FROM users WHERE id = ${uid}`)
try()
  ormDirect(sql, user)
exception(err)
  addVar(error, user_not_found)
  addResult(error)
end()

// Process results
if(user, , !=)
  variableFromJSON(user, name, userName)
  getDateTime(fmt, 0, UTC, today)
  AddvariableToJSON(name, userName, response)
  AddvariableToJSON(date, today, response)
  addResult(response)
end()
```
