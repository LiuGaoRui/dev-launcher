//! 分组服务（CRUD）
//!
//! 纯业务层，不依赖 Tauri，接收 `&Pool<Sqlite>`。
//! 错误：UNIQUE 冲突转 `AppError::GroupNameExists`。

use sqlx::Sqlite;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{Group, GroupInput, GroupUpdate};

pub struct GroupService;

impl GroupService {
    /// 列出全部分组，按 `order` 升序
    pub async fn list(pool: &sqlx::Pool<Sqlite>) -> AppResult<Vec<Group>> {
        let groups = sqlx::query_as::<_, Group>(
            r#"SELECT id, name, `order` AS "order", create_time
               FROM `group` ORDER BY `order` ASC, id ASC"#,
        )
        .fetch_all(pool)
        .await?;
        Ok(groups)
    }

    /// 按 id 取分组
    pub async fn get(pool: &sqlx::Pool<Sqlite>, id: i64) -> AppResult<Group> {
        let group = sqlx::query_as::<_, Group>(
            r#"SELECT id, name, `order` AS "order", create_time
               FROM `group` WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        group.ok_or(AppError::GroupNotFound(id))
    }

    /// 新建分组（name UNIQUE）
    pub async fn create(pool: &sqlx::Pool<Sqlite>, input: GroupInput) -> AppResult<Group> {
        let next_order: i32 =
            sqlx::query_scalar("SELECT COALESCE(MAX(`order`), -1) + 1 FROM `group`")
                .fetch_one(pool)
                .await?;

        let result = sqlx::query(
            r#"INSERT INTO `group` (name, `order`) VALUES (?, ?)"#,
        )
        .bind(&input.name)
        .bind(next_order)
        .execute(pool)
        .await
        .map_err(|e| db::map_unique_err(e, |_| AppError::GroupNameExists(input.name.clone())))?;

        let id = result.last_insert_rowid();
        Self::get(pool, id).await
    }

    /// 更新分组（仅更新非 None 字段）
    pub async fn update(
        pool: &sqlx::Pool<Sqlite>,
        id: i64,
        input: GroupUpdate,
    ) -> AppResult<Group> {
        // 无字段更新，直接返回当前
        if input.name.is_none() && input.order.is_none() {
            return Self::get(pool, id).await;
        }

        // 动态构建一条 UPDATE
        let mut sql = String::from("UPDATE `group` SET ");
        let mut params: Vec<String> = Vec::new();

        if input.name.is_some() {
            sql.push_str("name = ?");
            params.push(input.name.clone().unwrap());
        }
        if input.order.is_some() {
            if !params.is_empty() {
                sql.push_str(", ");
            }
            sql.push_str("`order` = ?");
            params.push(input.order.map(|o| o.to_string()).unwrap());
        }
        sql.push_str(" WHERE id = ?");

        let mut query = sqlx::query(&sql);
        for p in &params {
            query = query.bind(p);
        }
        query = query.bind(id);

        let rows = match query.execute(pool).await {
            Ok(r) => r.rows_affected(),
            Err(e) => {
                if input.name.is_some() {
                    return Err(db::map_unique_err(e, |_| {
                        AppError::GroupNameExists(input.name.clone().unwrap())
                    }));
                }
                return Err(e.into());
            }
        };
        if rows == 0 {
            return Err(AppError::GroupNotFound(id));
        }

        Self::get(pool, id).await
    }

    /// 删除分组
    pub async fn delete(pool: &sqlx::Pool<Sqlite>, id: i64) -> AppResult<()> {
        let affected = sqlx::query(r#"DELETE FROM `group` WHERE id = ?"#)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::GroupNotFound(id));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> sqlx::Pool<Sqlite> {
        crate::services::setup_pool().await
    }

    #[tokio::test]
    async fn create_get_list() {
        let pool = pool().await;

        let g1 = GroupService::create(&pool, GroupInput { name: "HR系统".into() })
            .await
            .unwrap();
        let g2 = GroupService::create(&pool, GroupInput { name: "中间件".into() })
            .await
            .unwrap();

        assert_eq!(g1.name, "HR系统");
        assert_eq!(g1.order, 0);
        assert_eq!(g2.order, 1);

        let got = GroupService::get(&pool, g1.id).await.unwrap();
        assert_eq!(got.name, "HR系统");

        let list = GroupService::list(&pool).await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, g1.id);
        assert_eq!(list[1].id, g2.id);
    }

    #[tokio::test]
    async fn create_duplicate_name_errors() {
        let pool = pool().await;
        GroupService::create(&pool, GroupInput { name: "HR".into() })
            .await
            .unwrap();
        let err = GroupService::create(&pool, GroupInput { name: "HR".into() })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::GroupNameExists(_)));
    }

    #[tokio::test]
    async fn update_fields() {
        let pool = pool().await;
        let g = GroupService::create(&pool, GroupInput { name: "A".into() })
            .await
            .unwrap();

        let updated = GroupService::update(
            &pool,
            g.id,
            GroupUpdate {
                name: Some("B".into()),
                order: Some(5),
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.name, "B");
        assert_eq!(updated.order, 5);
    }

    #[tokio::test]
    async fn delete_sets_project_group_null() {
        let pool = pool().await;
        let g = GroupService::create(&pool, GroupInput { name: "G".into() })
            .await
            .unwrap();

        sqlx::query(
            r#"INSERT INTO project (name, group_id, type, path, start_cmd)
               VALUES ('P1', ?, 'node', '/tmp', 'npm run dev')"#,
        )
        .bind(g.id)
        .execute(&pool)
        .await
        .unwrap();

        GroupService::delete(&pool, g.id).await.unwrap();

        let group_id: Option<i64> =
            sqlx::query_scalar("SELECT group_id FROM project WHERE name = 'P1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(group_id.is_none());

        let err = GroupService::delete(&pool, g.id).await.unwrap_err();
        assert!(matches!(err, AppError::GroupNotFound(_)));
    }
}
