//! 端口监控器忽略名单服务
//!
//! 管理用户手动忽略的「可疑端口」持久化（port_ignore 表）。按端口号记录——
//! 端口是稳定标识，进程重启 PID 变化不影响忽略；同端口号被其他进程占用
//! 仍被压制（语义 = 用户认可该端口号，不再提醒、不计入角标）。
//!
//! 条目独立于端口是否在监听：未监听时条目也保留（无害），仅用户在 UI
//! 「已忽略」折叠区点「恢复」时删除；端口再次被占用时会以 ignored 分类
//! 重新出现，届时可恢复。
//!
//! 纯业务层，不依赖 Tauri，接收 `&Pool<Sqlite>`。

use std::collections::HashSet;

use sqlx::{Row, Sqlite};

use crate::error::AppResult;

pub struct PortIgnoreService;

impl PortIgnoreService {
    /// 读取忽略端口号集合。
    ///
    /// `list_listening_ports` 命令用它对扫描结果应用忽略覆盖
    /// （命中 → Ignored 分类，score 0）。
    pub async fn list_ignored(pool: &sqlx::Pool<Sqlite>) -> AppResult<HashSet<u16>> {
        let rows = sqlx::query("SELECT port FROM port_ignore")
            .fetch_all(pool)
            .await?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                let port: i64 = row.try_get("port").ok()?;
                u16::try_from(port).ok()
            })
            .collect())
    }

    /// 忽略一个端口（按端口号去重，重复忽略为幂等操作）。
    pub async fn add(pool: &sqlx::Pool<Sqlite>, port: u16) -> AppResult<()> {
        sqlx::query("INSERT OR IGNORE INTO port_ignore (port) VALUES (?)")
            .bind(port as i64)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 恢复一个端口（「已忽略」折叠区的「恢复」按钮）。
    pub async fn remove(pool: &sqlx::Pool<Sqlite>, port: u16) -> AppResult<()> {
        sqlx::query("DELETE FROM port_ignore WHERE port = ?")
            .bind(port as i64)
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup() -> sqlx::Pool<Sqlite> {
        crate::services::setup_pool().await
    }

    #[tokio::test]
    async fn add_is_idempotent_by_port() {
        let pool = setup().await;
        PortIgnoreService::add(&pool, 52389).await.unwrap();
        PortIgnoreService::add(&pool, 52389).await.unwrap(); // 重复忽略
        let ignored = PortIgnoreService::list_ignored(&pool).await.unwrap();
        assert_eq!(ignored, HashSet::from([52389]));
    }

    #[tokio::test]
    async fn remove_deletes_only_target_port() {
        let pool = setup().await;
        PortIgnoreService::add(&pool, 3001).await.unwrap();
        PortIgnoreService::add(&pool, 3002).await.unwrap();

        PortIgnoreService::remove(&pool, 3001).await.unwrap();
        let ignored = PortIgnoreService::list_ignored(&pool).await.unwrap();
        assert_eq!(ignored, HashSet::from([3002]));
    }
}
