PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;
PRAGMA foreign_keys = ON;

-- %x locks: version history for file-level bindings
CREATE TABLE IF NOT EXISTS x_locks (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    data_version INTEGER NOT NULL,
    shape_version INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    first_y INTEGER NOT NULL DEFAULT 0,
    first_y_shape INTEGER NOT NULL DEFAULT 0,
    first_z INTEGER NOT NULL DEFAULT 0,
    first_z_shape INTEGER NOT NULL DEFAULT 0,
    time BLOB NOT NULL,
    sig BLOB NOT NULL,
    hash BLOB NOT NULL,


    UNIQUE(path, data_version)
);

-- Slot content: immutable snapshots keyed by (path, x_data_version)
-- Empty slot set at a version = tombstone
-- Large blobs (>100KB) are stored externally with value=NULL
CREATE TABLE IF NOT EXISTS slots (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    x_data_version INTEGER NOT NULL,
    slot TEXT NOT NULL,
    type TEXT NOT NULL,
    hash BLOB NOT NULL,
    value BLOB,

    UNIQUE(path, x_data_version, slot)
);

-- %y locks: version history for folder-level views
CREATE TABLE IF NOT EXISTS y_locks (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    data_version INTEGER NOT NULL,
    shape_version INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),

    UNIQUE(path, data_version)
);

-- %y child snapshots: which children existed at each y version
CREATE TABLE IF NOT EXISTS y_children (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    y_data_version INTEGER NOT NULL,
    child_path TEXT NOT NULL,
    child_x_data INTEGER NOT NULL,
    child_x_shape INTEGER NOT NULL,

    UNIQUE(path, y_data_version, child_path)
);

-- %z locks: version history for subtree-level views
CREATE TABLE IF NOT EXISTS z_locks (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    data_version INTEGER NOT NULL,
    shape_version INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),

    UNIQUE(path, data_version)
);

-- %z descendant snapshots: which descendants existed at each z version
CREATE TABLE IF NOT EXISTS z_descendants (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    z_data_version INTEGER NOT NULL,
    descendant_path TEXT NOT NULL,
    descendant_x_data INTEGER NOT NULL,
    descendant_x_shape INTEGER NOT NULL,

    UNIQUE(path, z_data_version, descendant_path)
);

-- Current version pointers (only mutable table)
CREATE TABLE IF NOT EXISTS current (
    path TEXT PRIMARY KEY,
    parent TEXT,
    x_data INTEGER NOT NULL DEFAULT 0,
    x_shape INTEGER NOT NULL DEFAULT 0,
    y_data INTEGER NOT NULL DEFAULT 0,
    y_shape INTEGER NOT NULL DEFAULT 0,
    z_data INTEGER NOT NULL DEFAULT 0,
    z_shape INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS parents (
    path TEXT PRIMARY KEY,
    parent_path TEXT,
);


-- Indexes for read performance
CREATE INDEX IF NOT EXISTS idx_x_locks_lookup ON x_locks(path, data_version DESC);
CREATE INDEX IF NOT EXISTS idx_slots_lookup ON slots(path, x_data_version);
CREATE INDEX IF NOT EXISTS idx_y_locks_lookup ON y_locks(path, data_version DESC);
CREATE INDEX IF NOT EXISTS idx_y_children_lookup ON y_children(path, y_data_version);
CREATE INDEX IF NOT EXISTS idx_z_locks_lookup ON z_locks(path, data_version DESC);
CREATE INDEX IF NOT EXISTS idx_z_descendants_lookup ON z_descendants(path, z_data_version);
CREATE INDEX IF NOT EXISTS idx_current_prefix ON current(path);
