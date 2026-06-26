-- ============================================================
-- DevLauncher v4 迁移：排序持久化
-- ============================================================
-- 两类排序：
--   1. project.sort_order：同一扫描目录面板内卡片的顺序
--   2. scan_root_order 表：扫描目录（面板）之间的顺序

-- project 表加排序列：默认 0，list 时按 sort_order ASC, id ASC 排序。
-- 新建项目时 service 层取当前最大值 +1，使新项目排到末尾。
ALTER TABLE project ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

-- 扫描目录顺序表：scan_root 为主键，sort_order 为面板排序值。
-- 仅记录用户调整过的顺序；未记录的目录在前端按字母序兜底排到已记录目录之后。
CREATE TABLE IF NOT EXISTS scan_root_order (
    scan_root  TEXT PRIMARY KEY,
    sort_order INTEGER NOT NULL DEFAULT 0
);
