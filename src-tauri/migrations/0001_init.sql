-- ============================================================
-- DevLauncher 初始化迁移
-- DDL 来源：docs/02-数据库设计.md
-- ============================================================

-- ============================================================
-- group: 项目分组
-- ============================================================
CREATE TABLE IF NOT EXISTS `group` (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    `order`     INTEGER NOT NULL DEFAULT 0,
    create_time TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- project: 被管理的项目
-- ============================================================
CREATE TABLE IF NOT EXISTS `project` (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    group_id        INTEGER REFERENCES `group`(id) ON DELETE SET NULL,
    type            TEXT NOT NULL CHECK (type IN ('springboot','java_jar','node','docker_compose','custom')),
    path            TEXT NOT NULL,                    -- 工作目录（绝对路径）
    start_cmd       TEXT NOT NULL,                    -- 启动命令完整串
    build_cmd       TEXT,                             -- 构建命令，可空
    expected_ports  TEXT NOT NULL DEFAULT '[]',       -- JSON 数组 ["8080","5173"]
    enabled         INTEGER NOT NULL DEFAULT 1,       -- 0=禁用 1=启用
    last_pid        INTEGER,                          -- 运行时缓存（权威态以实时探测为准）
    last_start_time TEXT,
    last_stop_time  TEXT,
    create_time     TEXT NOT NULL DEFAULT (datetime('now')),
    update_time     TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_project_group ON project(group_id);

-- ============================================================
-- log_ref: 日志文件索引（便于历史日志快速定位）
-- ============================================================
CREATE TABLE IF NOT EXISTS `log_ref` (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    log_date    TEXT NOT NULL,                        -- YYYYMMDD
    file_path   TEXT NOT NULL,                        -- 相对 logs_root
    size_bytes  INTEGER NOT NULL DEFAULT 0,
    create_time TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, log_date)
);
CREATE INDEX IF NOT EXISTS idx_log_ref_project ON log_ref(project_id);

-- ============================================================
-- dependency: 项目间依赖关系（P2 用，MVP 建表不用）
-- ============================================================
CREATE TABLE IF NOT EXISTS `dependency` (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id              INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    depends_on_project_id   INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    UNIQUE(project_id, depends_on_project_id)
);
