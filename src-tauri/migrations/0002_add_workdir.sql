-- ============================================================
-- DevLauncher v2 迁移：project 表新增 workdir 列
-- ============================================================
-- workdir：运行时工作目录（可选）。
--   - 为空：spawn 用 path 作工作目录（向后兼容旧数据）
--   - 有值：spawn 用 workdir 作工作目录（license 等运行时资源在扫描根目录时用）
--
-- 与 path 的区别：
--   path   = 项目目录（找 pom.xml / package.json，构建在此执行）
--   workdir= 运行时工作目录（进程 current_dir，程序相对路径加载资源基于此）
-- 对多模块 Maven 项目：path=启动模块目录，workdir=扫描根目录（含 license 等）。

ALTER TABLE project ADD COLUMN workdir TEXT;
