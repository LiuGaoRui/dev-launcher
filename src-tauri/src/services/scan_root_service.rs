//! 扫描目录顺序服务
//!
//! 管理扫描目录（面板）的排序持久化。scan_root_order 表以 scan_root 为主键，
//! 仅记录用户调整过的顺序；未记录的目录在前端按字母序兜底。
//!
//! 纯业务层，不依赖 Tauri，接收 `&Pool<Sqlite>`。

use std::collections::HashMap;

use sqlx::Sqlite;

use crate::error::AppResult;

pub struct ScanRootService;

impl ScanRootService {
    /// 读取全部已记录的扫描目录顺序：scan_root → sort_order。
    pub async fn list_order(pool: &sqlx::Pool<Sqlite>) -> AppResult<HashMap<String, i64>> {
        let rows = sqlx::query("SELECT scan_root, sort_order FROM scan_root_order")
            .fetch_all(pool)
            .await?;

        let mut map = HashMap::with_capacity(rows.len());
        use sqlx::Row;
        for row in rows {
            let root: String = row.try_get("scan_root")?;
            let order: i64 = row.try_get("sort_order")?;
            map.insert(root, order);
        }
        Ok(map)
    }

    /// 按给定顺序批量重排扫描目录。
    ///
    /// 用事务保证原子性：先清空旧记录，再按 `roots` 顺序批量 INSERT（下标即 sort_order）。
    /// 传空 Vec 视为清空全部排序记录（恢复默认字母序）。
    pub async fn reorder(pool: &sqlx::Pool<Sqlite>, roots: Vec<String>) -> AppResult<()> {
        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM scan_root_order")
            .execute(&mut *tx)
            .await?;

        if !roots.is_empty() {
            let placeholders = vec!["(?, ?)"; roots.len()];
            let sql = format!(
                "INSERT INTO scan_root_order (scan_root, sort_order) VALUES {}",
                placeholders.join(", ")
            );
            let mut query = sqlx::query(&sql);
            for (i, root) in roots.iter().enumerate() {
                query = query.bind(root).bind(i as i64);
            }
            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
