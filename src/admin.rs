//! Commands for the resources of `autumn-admin-plugin` (feature `admin`).
//!
//! Each model gives two commands: its list (`{prefix}/{slug}`) and "New"
//! (`{prefix}/{slug}/new`). They are in the [`ADMIN_GROUP`] group.
//!
//! The admin pages check the role. The palette does not. Thus give a
//! `visible` filter. It decides who sees the commands. To show them to all
//! users, give `|_| true`.
//!
//! ```ignore
//! use autumn_command_palette_plugin::admin::admin_commands;
//!
//! let commands = admin_commands("/admin", &[&ProjectAdmin], |parts| is_admin(parts));
//! let registry = CommandRegistry::new().extend(commands);
//! ```

use std::sync::Arc;

use autumn_admin_plugin::AdminModel;
use autumn_web::reexports::http::request::Parts;

use crate::command::{Command, Visibility};

/// The group of the admin commands.
pub const ADMIN_GROUP: &str = "Admin";

/// The commands of `models`, mounted at `prefix` (for example `/admin`).
/// `visible` runs for each request.
#[must_use]
pub fn admin_commands<F>(prefix: &str, models: &[&dyn AdminModel], visible: F) -> Vec<Command>
where
    F: Fn(&Parts) -> bool + Send + Sync + 'static,
{
    let prefix = prefix.trim_end_matches('/');
    let visible: Visibility = Arc::new(visible);
    models
        .iter()
        .flat_map(|model| {
            let slug = model.slug();
            [
                Command::route(
                    format!("admin.{slug}"),
                    model.display_name_plural(),
                    format!("{prefix}/{slug}"),
                )
                .group(ADMIN_GROUP)
                .keywords([slug, "admin"])
                .visible_shared(Arc::clone(&visible)),
                Command::route(
                    format!("admin.{slug}.new"),
                    format!("New {}", model.display_name()),
                    format!("{prefix}/{slug}/new"),
                )
                .group(ADMIN_GROUP)
                .keywords([slug, "admin", "create"])
                .visible_shared(Arc::clone(&visible)),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandTarget;
    use autumn_admin_plugin::{AdminField, AdminFuture, ListParams, ListResult};
    use autumn_web::RuntimeConnection;
    use autumn_web::reexports::diesel_async::pooled_connection::deadpool::Pool;
    use autumn_web::reexports::http::Request;
    use serde_json::Value;

    struct Projects;

    impl AdminModel for Projects {
        fn slug(&self) -> &'static str {
            "projects"
        }
        fn display_name(&self) -> &'static str {
            "Project"
        }
        fn display_name_plural(&self) -> &'static str {
            "Projects"
        }
        fn fields(&self) -> Vec<AdminField> {
            Vec::new()
        }
        fn list(&self, _: &Pool<RuntimeConnection>, _: ListParams) -> AdminFuture<'_, ListResult> {
            unimplemented!("not called")
        }
        fn get(&self, _: &Pool<RuntimeConnection>, _: i64) -> AdminFuture<'_, Option<Value>> {
            unimplemented!("not called")
        }
        fn create(&self, _: &Pool<RuntimeConnection>, _: Value) -> AdminFuture<'_, Value> {
            unimplemented!("not called")
        }
        fn update(&self, _: &Pool<RuntimeConnection>, _: i64, _: Value) -> AdminFuture<'_, Value> {
            unimplemented!("not called")
        }
        fn delete(&self, _: &Pool<RuntimeConnection>, _: i64) -> AdminFuture<'_, ()> {
            unimplemented!("not called")
        }
    }

    fn parts(admin: bool) -> Parts {
        let mut request = Request::get("/").body(()).expect("request");
        if admin {
            request
                .headers_mut()
                .insert("x-admin", "1".parse().expect("header"));
        }
        request.into_parts().0
    }

    #[test]
    fn each_model_gives_a_list_and_a_new_command() {
        let commands = admin_commands("/admin/", &[&Projects], |parts| {
            parts.headers.contains_key("x-admin")
        });
        let ids: Vec<&str> = commands.iter().map(Command::id).collect();
        assert_eq!(ids, ["admin.projects", "admin.projects.new"]);
        assert_eq!(commands[0].title(), "Projects");
        assert_eq!(
            commands[0].target(),
            &CommandTarget::Route("/admin/projects".into())
        );
        assert_eq!(commands[1].title(), "New Project");
        assert_eq!(
            commands[1].target(),
            &CommandTarget::Route("/admin/projects/new".into())
        );
        assert!(commands.iter().all(|c| c.group_name() == ADMIN_GROUP));
        assert!(commands[0].keyword_list().iter().any(|k| k == "projects"));
    }

    #[test]
    fn the_filter_decides_who_sees_them() {
        let commands = admin_commands("/admin", &[&Projects], |parts| {
            parts.headers.contains_key("x-admin")
        });
        assert!(commands.iter().all(|c| c.is_visible(&parts(true))));
        assert!(commands.iter().all(|c| !c.is_visible(&parts(false))));
    }

    #[test]
    fn the_commands_go_in_a_registry() {
        let registry =
            crate::CommandRegistry::new().extend(admin_commands("/admin", &[&Projects], |_| true));
        assert_eq!(registry.len(), 2);
    }
}
