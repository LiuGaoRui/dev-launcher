-- ============================================================
-- DevLauncher v3 迁移：project 表新增 scan_root 列
-- ============================================================
-- scan_root：扫描根目录（可选）。
--   扫描添加时记录扫描时的根目录路径，用于前端按「扫描目录」分面板展示。
--   手动添加的项目可为空，前端归入「其他」面板。
--
-- 与 path / workdir 的区别：
--   path      = 项目目录（找 pom.xml / package.json，构建在此执行）
--   workdir   = 运行时工作目录（进程 current_dir）
--   scan_root = 扫描时选定的根目录（界面分组依据，与 workdir 常相同但语义独立）

ALTER TABLE project ADD COLUMN scan_root TEXT;
