//! 项目服务（CRUD）
//!
//! 纯业务层，不依赖 Tauri，接收 `&Pool<Sqlite>`。
//! `expected_ports` 以 JSON 字符串存入 TEXT 列，读取时由 sqlx `Json` 解包。
//! delete：阶段 2 实现进程 registry 前，先不校验运行态（仅按 enabled 软判断不阻断）。

use sqlx::Sqlite;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{Project, ProjectInput};
use crate::services::group_service::GroupService;

pub struct ProjectService;

impl ProjectService {
    /// 列出项目；`group_id = None` 列全部，`Some(gid)` 仅该分组。
    /// 用 `WHERE (? IS NULL OR group_id = ?)` 一条 SQL 覆盖两种场景。
    pub async fn list(
        pool: &sqlx::Pool<Sqlite>,
        group_id: Option<i64>,
    ) -> AppResult<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            "SELECT id, name, group_id, type, path, workdir, start_cmd, build_cmd,
                    expected_ports, enabled, last_pid, last_start_time, last_stop_time,
                    create_time, update_time
             FROM project
             WHERE (?1 IS NULL OR group_id = ?1)
             ORDER BY id ASC",
        )
        .bind(group_id)
        .fetch_all(pool)
        .await?;
        Ok(projects)
    }

    /// 按 id 取项目
    pub async fn get(pool: &sqlx::Pool<Sqlite>, id: i64) -> AppResult<Project> {
        let project = sqlx::query_as::<_, Project>(
            "SELECT id, name, group_id, type, path, workdir, start_cmd, build_cmd,
                    expected_ports, enabled, last_pid, last_start_time, last_stop_time,
                    create_time, update_time
             FROM project WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        project.ok_or(AppError::ProjectNotFound(id))
    }

    /// 新建项目
    pub async fn create(
        pool: &sqlx::Pool<Sqlite>,
        input: ProjectInput,
    ) -> AppResult<Project> {
        // 校验 group_id 存在（若提供）
        if let Some(gid) = input.group_id {
            GroupService::get(pool, gid).await?;
        }

        let ports_json = serde_json::to_string(&input.expected_ports)?;
        let enabled_int: i32 = if input.enabled { 1 } else { 0 };

        let result = sqlx::query(
            r#"INSERT INTO project
               (name, group_id, type, path, workdir, start_cmd, build_cmd,
                expected_ports, enabled)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&input.name)
        .bind(input.group_id)
        .bind(input.r#type.as_str())
        .bind(&input.path)
        .bind(input.workdir.as_ref())
        .bind(&input.start_cmd)
        .bind(input.build_cmd.as_ref())
        .bind(ports_json)
        .bind(enabled_int)
        .execute(pool)
        .await
        .map_err(|e| db::map_unique_err(e, |_| AppError::ProjectNameExists(input.name.clone())))?;

        let id = result.last_insert_rowid();
        Self::get(pool, id).await
    }

    /// 更新项目（全量覆盖字段；id / create_time / last_* 不动）
    pub async fn update(
        pool: &sqlx::Pool<Sqlite>,
        id: i64,
        input: ProjectInput,
    ) -> AppResult<Project> {
        // 先校验存在
        Self::get(pool, id).await?;
        if let Some(gid) = input.group_id {
            GroupService::get(pool, gid).await?;
        }

        let ports_json = serde_json::to_string(&input.expected_ports)?;
        let enabled_int: i32 = if input.enabled { 1 } else { 0 };

        sqlx::query(
            r#"UPDATE project SET
                 name = ?, group_id = ?, type = ?, path = ?, workdir = ?,
                 start_cmd = ?, build_cmd = ?, expected_ports = ?, enabled = ?,
                 update_time = datetime('now')
               WHERE id = ?"#,
        )
        .bind(&input.name)
        .bind(input.group_id)
        .bind(input.r#type.as_str())
        .bind(&input.path)
        .bind(input.workdir.as_ref())
        .bind(&input.start_cmd)
        .bind(input.build_cmd.as_ref())
        .bind(ports_json)
        .bind(enabled_int)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| db::map_unique_err(e, |_| AppError::ProjectNameExists(input.name.clone())))?;

        Self::get(pool, id).await
    }

    /// 删除项目。
    ///
    /// TODO(阶段 2)：接入 ProcessRegistry 后，运行中拒绝删除（返回 AlreadyRunning）。
    pub async fn delete(pool: &sqlx::Pool<Sqlite>, id: i64) -> AppResult<()> {
        Self::get(pool, id).await?;
        sqlx::query("DELETE FROM project WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
fn sample_input(name: &str) -> ProjectInput {
    use crate::models::ProjectType;
    ProjectInput {
        name: name.into(),
        group_id: None,
        r#type: ProjectType::Node,
        path: "/tmp/proj".into(),
        workdir: None,
        start_cmd: "npm run dev".into(),
        build_cmd: Some("npm run build".into()),
        expected_ports: vec!["5173".into(), "3000".into()],
        enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ProjectType;

    async fn pool() -> sqlx::Pool<Sqlite> {
        crate::services::setup_pool().await
    }

    #[tokio::test]
    async fn create_get() {
        let pool = pool().await;
        let p = ProjectService::create(&pool, sample_input("前端"))
            .await
            .unwrap();
        assert_eq!(p.name, "前端");
        assert_eq!(p.r#type, ProjectType::Node);
        assert_eq!(p.expected_ports, vec!["5173".to_string(), "3000".into()]);
        assert!(p.enabled);

        let got = ProjectService::get(&pool, p.id).await.unwrap();
        assert_eq!(got.start_cmd, "npm run dev");
    }

    #[tokio::test]
    async fn list_filter_by_group() {
        let pool = pool().await;
        let g = GroupService::create(&pool, crate::models::GroupInput { name: "G".into() })
            .await
            .unwrap();

        let mut in_g = sample_input("ingroup");
        in_g.group_id = Some(g.id);
        let mut out_g = sample_input("outgroup");
        out_g.group_id = None;
        ProjectService::create(&pool, in_g).await.unwrap();
        ProjectService::create(&pool, out_g).await.unwrap();

        let all = ProjectService::list(&pool, None).await.unwrap();
        assert_eq!(all.len(), 2);

        let in_list = ProjectService::list(&pool, Some(g.id)).await.unwrap();
        assert_eq!(in_list.len(), 1);
        assert_eq!(in_list[0].name, "ingroup");
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let pool = pool().await;
        let p = ProjectService::create(&pool, sample_input("A"))
            .await
            .unwrap();

        let mut upd = sample_input("A2");
        upd.r#type = ProjectType::Springboot;
        upd.start_cmd = "mvn spring-boot:run".into();
        upd.expected_ports = vec!["8080".into()];
        let updated = ProjectService::update(&pool, p.id, upd).await.unwrap();

        assert_eq!(updated.name, "A2");
        assert_eq!(updated.r#type, ProjectType::Springboot);
        assert_eq!(updated.start_cmd, "mvn spring-boot:run");
        assert_eq!(updated.expected_ports, vec!["8080".to_string()]);
    }

    #[tokio::test]
    async fn create_with_invalid_group_errors() {
        let pool = pool().await;
        let mut input = sample_input("X");
        input.group_id = Some(9999);
        let err = ProjectService::create(&pool, input).await.unwrap_err();
        assert!(matches!(err, AppError::GroupNotFound(9999)));
    }

    #[tokio::test]
    async fn delete_removes_project() {
        let pool = pool().await;
        let p = ProjectService::create(&pool, sample_input("Del"))
            .await
            .unwrap();
        ProjectService::delete(&pool, p.id).await.unwrap();
        let err = ProjectService::get(&pool, p.id).await.unwrap_err();
        assert!(matches!(err, AppError::ProjectNotFound(_)));
    }

    #[tokio::test]
    async fn ports_json_roundtrip_empty() {
        let pool = pool().await;
        let mut input = sample_input("empty");
        input.expected_ports = vec![];
        let p = ProjectService::create(&pool, input).await.unwrap();
        assert!(p.expected_ports.is_empty());
    }
}
