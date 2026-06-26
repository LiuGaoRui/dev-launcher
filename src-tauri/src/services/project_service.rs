//! 项目服务（CRUD）
//!
//! 纯业务层，不依赖 Tauri，接收 `&Pool<Sqlite>`。
//! `expected_ports` 以 JSON 字符串存入 TEXT 列，读取时由 sqlx `Json` 解包。
//! delete：阶段 2 实现进程 registry 前，先不校验运行态（仅按 enabled 软判断不阻断）。

use sqlx::Sqlite;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::{Project, ProjectInput};

pub struct ProjectService;

impl ProjectService {
    /// 列出全部项目，按 sort_order ASC, id ASC 排序。
    pub async fn list(pool: &sqlx::Pool<Sqlite>) -> AppResult<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            "SELECT id, name, type, path, workdir, scan_root, start_cmd, build_cmd,
                    expected_ports, enabled, last_pid, last_start_time, last_stop_time,
                    create_time, update_time, sort_order
             FROM project
             ORDER BY sort_order ASC, id ASC",
        )
        .fetch_all(pool)
        .await?;
        Ok(projects)
    }

    /// 按 id 取项目
    pub async fn get(pool: &sqlx::Pool<Sqlite>, id: i64) -> AppResult<Project> {
        let project = sqlx::query_as::<_, Project>(
            "SELECT id, name, type, path, workdir, scan_root, start_cmd, build_cmd,
                    expected_ports, enabled, last_pid, last_start_time, last_stop_time,
                    create_time, update_time, sort_order
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
        let ports_json = serde_json::to_string(&input.expected_ports)?;
        let enabled_int: i32 = if input.enabled { 1 } else { 0 };

        // 新项目排到末尾：sort_order = 当前最大值 + 1（无项目时为 0）
        let next_order: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM project")
                .fetch_one(pool)
                .await?;

        let result = sqlx::query(
            r#"INSERT INTO project
               (name, type, path, workdir, scan_root, start_cmd, build_cmd,
                expected_ports, enabled, sort_order)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&input.name)
        .bind(input.r#type.as_str())
        .bind(&input.path)
        .bind(input.workdir.as_ref())
        .bind(input.scan_root.as_ref())
        .bind(&input.start_cmd)
        .bind(input.build_cmd.as_ref())
        .bind(ports_json)
        .bind(enabled_int)
        .bind(next_order)
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

        let ports_json = serde_json::to_string(&input.expected_ports)?;
        let enabled_int: i32 = if input.enabled { 1 } else { 0 };

        sqlx::query(
            r#"UPDATE project SET
                 name = ?, type = ?, path = ?, workdir = ?, scan_root = ?,
                 start_cmd = ?, build_cmd = ?, expected_ports = ?, enabled = ?,
                 update_time = datetime('now')
               WHERE id = ?"#,
        )
        .bind(&input.name)
        .bind(input.r#type.as_str())
        .bind(&input.path)
        .bind(input.workdir.as_ref())
        .bind(input.scan_root.as_ref())
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
    pub async fn delete(pool: &sqlx::Pool<Sqlite>, id: i64) -> AppResult<()> {
        Self::get(pool, id).await?;
        sqlx::query("DELETE FROM project WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 按 ids 顺序批量重排项目 sort_order。
    ///
    /// ids 的下标即新的 sort_order。用 CASE WHEN 一条 SQL 完成全部更新，
    /// 避免逐条 UPDATE 的 N 次往返。仅更新命中的 id，未传入的项目 sort_order 不变。
    pub async fn reorder(pool: &sqlx::Pool<Sqlite>, ids: Vec<i64>) -> AppResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        // 构造 `WHEN id = ? THEN ? ...` 片段
        let cases: String = ids
            .iter()
            .map(|_| "WHEN id = ? THEN ?")
            .collect::<Vec<_>>()
            .join(" ");
        let sql = format!("UPDATE project SET sort_order = CASE {cases} ELSE sort_order END");

        let mut query = sqlx::query(&sql);
        for (i, id) in ids.iter().enumerate() {
            query = query.bind(id).bind(i as i64);
        }
        query.execute(pool).await?;
        Ok(())
    }
}

#[cfg(test)]
fn sample_input(name: &str) -> ProjectInput {
    use crate::models::ProjectType;
    ProjectInput {
        name: name.into(),
        r#type: ProjectType::Node,
        path: "/tmp/proj".into(),
        workdir: None,
        scan_root: None,
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
    async fn list_returns_all() {
        let pool = pool().await;
        ProjectService::create(&pool, sample_input("A"))
            .await
            .unwrap();
        ProjectService::create(&pool, sample_input("B"))
            .await
            .unwrap();

        let all = ProjectService::list(&pool).await.unwrap();
        assert_eq!(all.len(), 2);
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

    #[tokio::test]
    async fn scan_root_persists() {
        let pool = pool().await;
        let mut input = sample_input("scanned");
        input.scan_root = Some("D:/code/repo".into());
        let p = ProjectService::create(&pool, input).await.unwrap();
        assert_eq!(p.scan_root.as_deref(), Some("D:/code/repo"));
    }

    #[tokio::test]
    async fn create_appends_to_sort_order() {
        let pool = pool().await;
        let a = ProjectService::create(&pool, sample_input("A"))
            .await
            .unwrap();
        let b = ProjectService::create(&pool, sample_input("B"))
            .await
            .unwrap();
        // 后创建的 sort_order 更大（排到末尾）
        assert!(b.sort_order > a.sort_order);
    }

    #[tokio::test]
    async fn reorder_changes_sort_order() {
        let pool = pool().await;
        let a = ProjectService::create(&pool, sample_input("A"))
            .await
            .unwrap();
        let b = ProjectService::create(&pool, sample_input("B"))
            .await
            .unwrap();
        let c = ProjectService::create(&pool, sample_input("C"))
            .await
            .unwrap();
        // 原序 A B C，重排为 C B A
        ProjectService::reorder(&pool, vec![c.id, b.id, a.id])
            .await
            .unwrap();
        let list = ProjectService::list(&pool).await.unwrap();
        assert_eq!(
            list.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
            vec!["C", "B", "A"]
        );
    }
}
