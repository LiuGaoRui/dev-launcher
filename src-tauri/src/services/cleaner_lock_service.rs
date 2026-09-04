//! 内存清理器进程锁定服务
//!
//! 管理手动锁定进程的**指纹**持久化（cleaner_lock 表）。锁定不按 PID 记录——
//! 进程重启后 PID 必变且可能被系统复用，故用稳定指纹（完整命令行优先，
//! 启动目录+进程名兜底）匹配。指纹由 [`dev_scan::fingerprint`] 基于未截断的
//! 完整命令行在扫描时算好，前端锁定时原样回传（`CleanerLockInput.fingerprint`），
//! 写入与匹配两侧同源，同一进程无论以何种 PID 重启，指纹恒定。
//!
//! 锁定条目独立于进程是否存在：进程未运行时条目也保留（永不丢失），
//! 仅用户「解锁全部」或逐条解锁时删除。
//!
//! 纯业务层，不依赖 Tauri，接收 `&Pool<Sqlite>`。

use std::collections::HashMap;

use sqlx::Sqlite;

use crate::error::AppResult;
use crate::models::{CleanerLock, CleanerLockInput};

/// 所有列的 SELECT 片段（list / 按 id 查共用）。
const SELECT_ALL: &str = "SELECT id, fingerprint, name, cmdline, cwd, exe, display_title, cmdline_summary, locked_pid, create_time FROM cleaner_lock";

pub struct CleanerLockService;

impl CleanerLockService {
    /// 读取全部锁定条目的匹配表：fingerprint → 锁定条目 id。
    ///
    /// `scan_dev_processes` 命令用它为每个扫描到的进程标注锁定态。
    pub async fn locked_map(pool: &sqlx::Pool<Sqlite>) -> AppResult<HashMap<String, i64>> {
        let rows = sqlx::query("SELECT fingerprint, id FROM cleaner_lock")
            .fetch_all(pool)
            .await?;

        let mut map = HashMap::with_capacity(rows.len());
        use sqlx::Row;
        for row in rows {
            let fp: String = row.try_get("fingerprint")?;
            let id: i64 = row.try_get("id")?;
            map.insert(fp, id);
        }
        Ok(map)
    }

    /// 列出全部锁定条目（按创建时间倒序，新锁定的在前）。
    pub async fn list(pool: &sqlx::Pool<Sqlite>) -> AppResult<Vec<CleanerLock>> {
        let locks = sqlx::query_as::<_, CleanerLock>(&format!(
            "{SELECT_ALL} ORDER BY create_time DESC, id DESC"
        ))
        .fetch_all(pool)
        .await?;
        Ok(locks)
    }

    /// 锁定一个进程（按指纹去重，重复锁定同一指纹为幂等操作）。
    ///
    /// `input.fingerprint` 由前端从扫描结果 `DevProcInfo.fingerprint`
    /// 原样回传（后端不重算），返回该指纹对应的锁定条目（新建或已存在的）。
    pub async fn add(pool: &sqlx::Pool<Sqlite>, input: CleanerLockInput) -> AppResult<CleanerLock> {
        let fp = input.fingerprint;
        sqlx::query(
            "INSERT OR IGNORE INTO cleaner_lock \
             (fingerprint, name, cmdline, cwd, exe, display_title, cmdline_summary, locked_pid) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&fp)
        .bind(&input.name)
        .bind(&input.cmdline)
        .bind(&input.cwd)
        .bind(&input.exe)
        .bind(&input.display_title)
        .bind(&input.cmdline_summary)
        .bind(input.pid as i64)
        .execute(pool)
        .await?;

        // 回读：INSERT OR IGNORE 命中已存在条目时也能拿到 id
        let lock = sqlx::query_as::<_, CleanerLock>(&format!(
            "{SELECT_ALL} WHERE fingerprint = ?"
        ))
        .bind(&fp)
        .fetch_one(pool)
        .await?;
        Ok(lock)
    }

    /// 解除单条锁定（按条目 id）。
    pub async fn remove(pool: &sqlx::Pool<Sqlite>, id: i64) -> AppResult<()> {
        sqlx::query("DELETE FROM cleaner_lock WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 清空全部锁定条目（「解锁全部」）。
    pub async fn clear(pool: &sqlx::Pool<Sqlite>) -> AppResult<()> {
        sqlx::query("DELETE FROM cleaner_lock")
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::dev_scan::fingerprint;

    async fn setup() -> sqlx::Pool<Sqlite> {
        crate::services::setup_pool().await
    }

    fn sample_input(pid: u32) -> CleanerLockInput {
        CleanerLockInput {
            name: "java.exe".into(),
            cmdline: "C:\\JDK\\bin\\java.exe -jar app.jar --server.port=8080".into(),
            cwd: Some("D:\\work\\demo".into()),
            exe: "C:\\JDK\\bin\\java.exe".into(),
            display_title: "demo.jar".into(),
            cmdline_summary: "demo.jar".into(),
            fingerprint: fingerprint(
                "java.exe",
                "C:\\JDK\\bin\\java.exe -jar app.jar --server.port=8080",
                Some("D:\\work\\demo"),
            ),
            pid,
        }
    }

    #[tokio::test]
    async fn add_is_idempotent_by_fingerprint() {
        let pool = setup().await;
        let first = CleanerLockService::add(&pool, sample_input(100)).await.unwrap();
        // 同指纹不同 PID（进程重启场景）：仍是同一条锁定
        let second = CleanerLockService::add(&pool, sample_input(999)).await.unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(CleanerLockService::list(&pool).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn locked_map_matches_restarted_process() {
        let pool = setup().await;
        let lock = CleanerLockService::add(&pool, sample_input(100)).await.unwrap();

        // 进程重启：PID 变了，指纹不变（大小写/空白差异也不影响）
        let map = CleanerLockService::locked_map(&pool).await.unwrap();
        let fp = fingerprint(
            "JAVA.EXE",
            "C:\\jdk\\bin\\java.exe -jar app.jar --server.port=8080",
            Some("d:\\work\\demo"),
        );
        assert_eq!(map.get(&fp), Some(&lock.id));
    }

    #[tokio::test]
    async fn remove_and_clear() {
        let pool = setup().await;
        let a = CleanerLockService::add(&pool, sample_input(1)).await.unwrap();
        let mut b = sample_input(2);
        b.name = "node.exe".into();
        b.cmdline = "node server.js".into();
        b.cwd = None;
        b.fingerprint = fingerprint("node.exe", "node server.js", None);
        let b = CleanerLockService::add(&pool, b).await.unwrap();
        assert_ne!(a.id, b.id);

        CleanerLockService::remove(&pool, a.id).await.unwrap();
        assert_eq!(CleanerLockService::list(&pool).await.unwrap().len(), 1);

        CleanerLockService::clear(&pool).await.unwrap();
        assert_eq!(CleanerLockService::list(&pool).await.unwrap().len(), 0);
    }
}
