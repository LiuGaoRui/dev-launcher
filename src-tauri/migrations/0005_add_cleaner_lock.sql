-- ============================================================
-- DevLauncher v5 迁移：内存清理器进程锁定表
-- ============================================================
-- 手动锁定的进程指纹持久化。不存 PID——进程重启后 PID 必变，
-- 改存稳定指纹（完整命令行优先，cwd+name 兜底，见 dev_scan::fingerprint）。
-- 锁定与匹配用同一后端函数计算指纹（基于未截断命令行），PID 变化不影响锁定；
-- 进程未运行时条目也保留，仅「解锁全部」可清空。
-- 注意：指纹算法基于完整命令行，早期基于截断命令行的开发期数据不兼容、
-- 不迁移（重新锁定即可）。

CREATE TABLE IF NOT EXISTS cleaner_lock (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 规范化匹配键（小写），UNIQUE 防止重复锁定同一指纹
    fingerprint     TEXT NOT NULL UNIQUE,
    -- 以下为锁定时的进程原始指纹（展示用）
    name            TEXT NOT NULL,
    cmdline         TEXT NOT NULL DEFAULT '',
    cwd             TEXT,
    exe             TEXT NOT NULL DEFAULT '',
    display_title   TEXT NOT NULL DEFAULT '',
    cmdline_summary TEXT NOT NULL DEFAULT '',
    -- 锁定时的 PID（仅参考展示，不参与匹配）
    locked_pid      INTEGER NOT NULL,
    create_time     TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
