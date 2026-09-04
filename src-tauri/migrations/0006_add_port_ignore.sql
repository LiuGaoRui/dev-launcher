-- ============================================================
-- DevLauncher v6 迁移：端口监控器忽略名单表
-- ============================================================
-- 用户手动忽略的「可疑端口」持久化。按端口号记录——端口是稳定标识，
-- 进程重启 PID 变化不影响忽略；同端口号被其他进程占用仍被压制
-- （语义 = 用户认可该端口号，不再提醒、不计入角标）。
-- 未在监听的死条目无害：该端口再次被占用时以 ignored 分类重新出现，
-- 届时可在 UI 中恢复（删除条目）。

CREATE TABLE IF NOT EXISTS port_ignore (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 忽略的端口号，UNIQUE 防止重复忽略
    port        INTEGER NOT NULL UNIQUE,
    create_time TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
