# shrine-debug

An interactive debug shell and testing framework for the Shrine hierarchical data store.

## Overview

`shrine-debug` provides:

- **Interactive REPL** - A command-line shell for exploring and manipulating Shrine data
- **Testing Framework** - Utilities for writing isolated unit tests with mocking and event recording
- **Event Tracing** - Full event recording for debugging and analysis

## Installation

```bash
cargo build -p shrine-debug
```

## Quick Start

### Running the Shell

```bash
cargo run -p shrine-debug
```

You'll see:
```
shrine-debug shell
mode: Shell
type 'help' for commands, 'exit' to quit
shrine>
```

### Basic Commands

```bash
# Create a binding at a path
shrine> make /users/alice [name="Alice", age="30"]
ok: make /users/alice

# Read the binding (%x = file-level read)
shrine> scry %x /users/alice
/users/alice  [v1.0]
  name: /types/text (5 bytes)
    Alice
  age: /types/text (2 bytes)
    30

# List children (%y = folder-level read)
shrine> scry %y /users
lock: v1.0
  /users/alice  [v1.0]

# List all descendants (%z = subtree read)
shrine> scry %z /
lock: v1.0
  /users         [v1.0]
    alice        [v1.0]

# Update content (no shape change)
shrine> poke /users/alice [age="31"]
ok: poke /users/alice

# Delete a binding
shrine> cull /users/alice
ok: cull /users/alice
```

## Command Reference

### Read Operations

| Command | Description |
|---------|-------------|
| `scry %x <path>` | Read a single binding (file) |
| `scry %y <path>` | List immediate children (folder) |
| `scry %z <path>` | List all descendants (subtree) |
| `scry %x <path>@<version>` | Read at specific version |

**Care Levels:**
- `%x` - Returns the full Tale (all slots) at a path
- `%y` - Returns the lock and immediate children with their locks
- `%z` - Returns the lock and all descendants in a tree structure

### Write Operations

| Command | Description |
|---------|-------------|
| `make <path> [slot=value, ...]` | Create or structurally modify a binding |
| `poke <path> [slot=value, ...]` | Update content without shape change |
| `cull <path>` | Delete a binding (write tombstone) |

**Tale Syntax:**
```bash
# String values
make /test [name="Alice", city="Boston"]

# Hex bytes
make /test [data=0xdeadbeef]

# Mixed
make /test [label="example", raw=0x0102030405]
```

### Subscriptions

| Command | Description |
|---------|-------------|
| `subscribe %x <path>` | Watch for changes to a binding |
| `subscribe %y <path>` | Watch for changes to children |
| `subscribe %z <path>` | Watch for changes in subtree |
| `sub %z <path>` | Short form |

### Debug Mode

| Command | Description |
|---------|-------------|
| `debug on` | Enable verbose event tracing |
| `debug off` | Disable tracing |
| `events` | Show all recorded events |
| `events <path>` | Show events for a specific path |
| `clear events` | Clear recorded events |

### Mock System (Testing)

| Command | Description |
|---------|-------------|
| `mock <pattern> ok` | Allow operations matching pattern |
| `mock <pattern> fail` | Fail operations matching pattern |
| `mock <pattern> capture` | Capture operations without executing |
| `inject <path> [tale]` | Inject a synthetic event |
| `clear mocks` | Clear all mocks |
| `clear all` | Clear events and mocks |

**Pattern Wildcards:**
- `/*` - Match single path segment
- `/**` - Match any number of segments (recursive)
- `~*` - Match any ship name (e.g., `~zod`, `~sampel-palnet`)

```bash
# Capture all writes under /test
shrine> mock /test/** capture

# Fail any ship-specific operations
shrine> mock /~*/apps/* fail

# Allow specific path
shrine> mock /allowed/path ok
```

### Utility Commands

| Command | Description |
|---------|-------------|
| `help` | Show all commands |
| `help <command>` | Show detailed help for a command |
| `exit` / `quit` / `q` | Exit the shell |

## Testing Framework

The `testing` module provides utilities for writing Shrine tests without real storage.

### Basic Test Structure

```rust
use shrine_debug::testing::*;

#[test]
fn test_my_feature() {
    // Create a mock namespace
    let mut ns = MockNamespace::new();

    // Inject test data
    ns.inject("/test/data", tale! {
        "name" => "Alice",
        "count" => 42u32,
    });

    // Verify data exists
    assert!(ns.exists("/test/data"));

    // Check children
    let children = ns.children("/test");
    assert_eq!(children.len(), 1);
}
```

### Helper Macros

```rust
// Create a PathSpec
let spec = path!("/foo/bar");
let versioned = path!("/foo/bar@5");

// Create a Tale (slot map)
let tale = tale! {
    "name" => "Alice",
    "count" => "42",
};

// Create a Pail with type
let pail = pail!("/types/text", "hello");
let binary = pail!("/types/binary", &[1u8, 2, 3][..]);

// Create a path pattern
let pattern = pattern!("/foo/**");
```

### MockNamespace

An in-memory namespace for pure unit tests:

```rust
let mut ns = MockNamespace::new();

// Inject bindings
ns.inject("/parent", tale!());
ns.inject("/parent/child1", tale! { "data" => "one" });
ns.inject("/parent/child2", tale! { "data" => "two" });

// Query structure
let children = ns.children("/parent");  // ["/parent/child1", "/parent/child2"]
let descendants = ns.descendants("/parent");  // All 3 paths

// Set up mocks
ns.mock("/captured/**", MockResponse::Capture);

// Record events for assertions
ns.record_read("/test", Care::X);
ns.recorder().assert_read_at("/test");
```

### Event Recording & Assertions

```rust
let recorder = EventRecorder::new();
recorder.start();

// ... perform operations ...

// Assert on events
recorder.assert_read_at("/test");
recorder.assert_write_at("/test");
recorder.assert_event_count(2);
recorder.assert_no_errors();
recorder.assert_read_count(1);
recorder.assert_write_count(1);
recorder.assert_subscribed("/path", Care::Z);
```

### Output Assertions

```rust
let output = TestOutput::new();

// ... perform operations that write output ...

output.assert_success();
output.assert_error();
output.assert_contains("expected text");
output.assert_not_contains("unexpected");
output.assert_output_count(3);
```

### TestDriverBuilder

Fluent API for setting up test environments:

```rust
let ns = TestDriverBuilder::new()
    .capture("/captured/**")
    .fail("/should-fail/*")
    .inject("/preset", tale! { "data" => "value" })
    .debug()
    .build_namespace();

assert!(ns.exists("/preset"));
assert_eq!(
    ns.mocks().matches("/captured/foo"),
    Some(MockResponse::Capture)
);
```

## Architecture

```
shrine-debug/
├── src/
│   ├── main.rs        # Entry point
│   ├── cli.rs         # ShellDriver - main REPL coordinator
│   ├── parser.rs      # Command parsing
│   ├── output.rs      # Output formatting (terminal, JSON, test)
│   ├── mock.rs        # Mock registry and path patterns
│   ├── recorder.rs    # Event recording for debugging
│   ├── interceptor.rs # Effect interception (shell/debug/test modes)
│   ├── testing.rs     # Test utilities and macros
│   ├── integration.rs # Integration tests
│   └── types.rs       # Re-exports from shrine-core
```

### Operating Modes

| Mode | Description |
|------|-------------|
| `Shell` | Normal interactive mode |
| `Debug` | Verbose tracing enabled, events recorded |
| `Test` | Full mocking, output capture, event recording |

## Data Model

### Paths

Shrine uses hierarchical paths similar to a filesystem:
- `/users/alice`
- `/apps/chat/messages`
- `/~zod/apps/landscape`

### Locks (Versions)

Every binding has a lock with two components:
- `data` - Content version (increments on poke)
- `shape` - Structure version (increments on make)

Displayed as `v{data}.{shape}`, e.g., `v5.2`

### Tales

A Tale is a map of slot names to Pails:
```
{
  "name": Pail { typ: "/types/text", data: "Alice" },
  "age": Pail { typ: "/types/uint", data: [30] }
}
```

### Pails

A Pail contains typed data:
- `typ` - Type path (e.g., `/types/text`, `/types/binary`)
- `data` - Raw bytes

## Event Types

The recorder captures these events:

| Event | Description |
|-------|-------------|
| `Read` | A scry operation |
| `Write` | A poke/make operation |
| `CardEmitted` | An effect was produced |
| `CardExecuted` | An effect was executed |
| `Subscribed` | A subscription was created |
| `SubscriptionUpdate` | Subscription received an update |
| `CommandParsed` | A command was parsed |
| `Error` | An error occurred |
| `Debug` | A debug message |

Export events to JSON for analysis:
```rust
let json = recorder.to_json()?;
```

## Examples

### Interactive Session

```bash
$ cargo run -p shrine-debug

shrine> make /config/app [name="MyApp", version="1.0"]
ok: make /config/app

shrine> make /config/app/settings [theme="dark", lang="en"]
ok: make /config/app/settings

shrine> scry %z /config
lock: v2.0
  /config/app           [v1.0]
    settings            [v1.0]

shrine> debug on
ok: debug mode enabled

shrine> scry %x /config/app
/config/app  [v1.0]
  name: /types/text (5 bytes)
    MyApp
  version: /types/text (3 bytes)
    1.0

shrine> events
2 events:
  [0ms] CMD scry (scry %x /config/app)
  [1ms] READ X /config/app

shrine> exit
goodbye
```

### Unit Test Example

```rust
use shrine_debug::testing::*;

#[test]
fn test_user_creation() {
    let mut ns = MockNamespace::new();

    // Set up initial state
    ns.inject("/users", tale!());

    // Simulate creating a user
    ns.inject("/users/bob", tale! {
        "name" => "Bob",
        "email" => "bob@example.com",
    });

    // Verify
    assert!(ns.exists("/users/bob"));
    let tale = ns.get("/users/bob").unwrap();
    assert_eq!(tale.get("name").unwrap().data, b"Bob");

    // Check hierarchy
    let users = ns.children("/users");
    assert_eq!(users.len(), 1);
    assert!(users.contains(&&"/users/bob".to_string()));
}

#[test]
fn test_mock_capture() {
    let ns = TestDriverBuilder::new()
        .capture("/external/**")
        .build_namespace();

    // Operations to /external/** are captured, not executed
    assert_eq!(
        ns.mocks().matches("/external/api/call"),
        Some(MockResponse::Capture)
    );
}
```

## License

See the root workspace LICENSE file.
