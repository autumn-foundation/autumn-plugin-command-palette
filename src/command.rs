//! Commands and the command registry.

use std::fmt;
use std::sync::Arc;

use autumn_web::reexports::http::request::Parts;

/// The longest command id, in bytes.
pub const MAX_ID_LEN: usize = 100;

/// The default group of a [`Command::route`].
pub const ROUTES_GROUP: &str = "Go to";

/// The default group of a [`Command::action`].
pub const ACTIONS_GROUP: &str = "Actions";

/// A command id. It is not empty, has [`MAX_ID_LEN`] bytes or fewer, and
/// has only ASCII letters, digits and `-`, `_`, `.`, `:`, `/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(String);

impl CommandId {
    /// Make an id.
    ///
    /// # Errors
    ///
    /// [`RegistryError::BadId`] when `id` breaks the rules above.
    pub fn new(id: impl Into<String>) -> Result<Self, RegistryError> {
        let id = id.into();
        let valid = !id.is_empty()
            && id.len() <= MAX_ID_LEN
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b));
        if valid {
            Ok(Self(id))
        } else {
            Err(RegistryError::BadId(id))
        }
    }

    /// The id text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A registry error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// The id breaks the [`CommandId`] rules.
    #[error("`{0}` is not a command id: use 1 to 100 ASCII letters, digits, - _ . : /")]
    BadId(String),
    /// Two commands have the same id.
    #[error("two commands have the id `{0}`")]
    DuplicateId(String),
    /// The command has an empty title.
    #[error("the command `{0}` has an empty title")]
    EmptyTitle(String),
}

/// What a command does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandTarget {
    /// Go to this URL with `GET`.
    Route(String),
    /// Send a `POST` to this path, with the CSRF token.
    Action(String),
}

/// A per-request filter. `true` shows the command.
pub type Visibility = Arc<dyn Fn(&Parts) -> bool + Send + Sync>;

/// One command in the palette.
///
/// ```
/// use autumn_command_palette_plugin::Command;
///
/// let new_post = Command::action("posts.new", "New post", "/posts")
///     .keywords(["create", "write"])
///     .shortcut(&["N"]);
/// assert_eq!(new_post.title(), "New post");
/// ```
#[derive(Clone)]
pub struct Command {
    id: String,
    title: String,
    group: String,
    target: CommandTarget,
    keywords: Vec<String>,
    shortcut: Vec<String>,
    hint: Option<String>,
    visible: Option<Visibility>,
}

impl Command {
    /// A command that goes to `href`. Its group is [`ROUTES_GROUP`].
    #[must_use]
    pub fn route(id: impl Into<String>, title: impl Into<String>, href: impl Into<String>) -> Self {
        Self::new(id, title, ROUTES_GROUP, CommandTarget::Route(href.into()))
    }

    /// A command that sends a `POST` to `path`. Its group is
    /// [`ACTIONS_GROUP`].
    #[must_use]
    pub fn action(
        id: impl Into<String>,
        title: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self::new(id, title, ACTIONS_GROUP, CommandTarget::Action(path.into()))
    }

    fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        group: &str,
        target: CommandTarget,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group: group.to_owned(),
            target,
            keywords: Vec::new(),
            shortcut: Vec::new(),
            hint: None,
            visible: None,
        }
    }

    /// Put the command in `group`. The palette shows groups in the order of
    /// their first command.
    #[must_use]
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Add search words that are not in the title.
    #[must_use]
    pub fn keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords.extend(keywords.into_iter().map(Into::into));
        self
    }

    /// Show a keyboard shortcut, for example `&["Ctrl", "N"]`. The palette
    /// shows it in `kbd`. The app binds the shortcut itself.
    #[must_use]
    pub fn shortcut(mut self, keys: &[&str]) -> Self {
        self.shortcut = keys.iter().map(|&key| key.to_owned()).collect();
        self
    }

    /// Show a short text after the title.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Show the command only when `visible` returns `true` for the request.
    #[must_use]
    pub fn visible_when<F>(self, visible: F) -> Self
    where
        F: Fn(&Parts) -> bool + Send + Sync + 'static,
    {
        self.visible_shared(Arc::new(visible))
    }

    /// Set a filter that other commands share.
    #[must_use]
    pub(crate) fn visible_shared(mut self, visible: Visibility) -> Self {
        self.visible = Some(visible);
        self
    }

    /// The id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The group.
    #[must_use]
    pub fn group_name(&self) -> &str {
        &self.group
    }

    /// The target.
    #[must_use]
    pub const fn target(&self) -> &CommandTarget {
        &self.target
    }

    /// The search words.
    #[must_use]
    pub fn keyword_list(&self) -> &[String] {
        &self.keywords
    }

    /// The shortcut keys.
    #[must_use]
    pub fn shortcut_keys(&self) -> &[String] {
        &self.shortcut
    }

    /// The hint.
    #[must_use]
    pub fn hint_text(&self) -> Option<&str> {
        self.hint.as_deref()
    }

    /// `true` when the command has a [`visible_when`](Self::visible_when)
    /// filter.
    #[must_use]
    pub const fn has_filter(&self) -> bool {
        self.visible.is_some()
    }

    /// `true` when the command shows for `parts`.
    #[must_use]
    pub fn is_visible(&self, parts: &Parts) -> bool {
        self.visible.as_ref().is_none_or(|visible| visible(parts))
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Command")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("group", &self.group)
            .field("target", &self.target)
            .field("keywords", &self.keywords)
            .field("shortcut", &self.shortcut)
            .field("hint", &self.hint)
            .field("visible_when", &self.visible.is_some())
            .finish()
    }
}

/// The commands of the app, in order.
///
/// ```
/// use autumn_command_palette_plugin::{Command, CommandRegistry};
///
/// let registry = CommandRegistry::new()
///     .add(Command::route("home", "Dashboard", "/"))
///     .add(Command::action("sign-out", "Sign out", "/logout"));
/// assert_eq!(registry.len(), 2);
/// ```
#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    /// An empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Add a command.
    ///
    /// # Panics
    ///
    /// Panics when [`try_add`](Self::try_add) fails. A bad registry is a
    /// start-up bug, as a duplicate story slug is.
    #[must_use]
    #[track_caller]
    #[allow(
        clippy::should_implement_trait,
        reason = "a builder step, as `StoryGallery` has; not `std::ops::Add`"
    )]
    pub fn add(self, command: Command) -> Self {
        match self.try_add(command) {
            Ok(registry) => registry,
            Err(err) => panic!("autumn-command-palette: {err}"),
        }
    }

    /// Add a command.
    ///
    /// # Errors
    ///
    /// [`RegistryError`] when the id is bad or is in the registry, or the
    /// title is empty.
    pub fn try_add(mut self, command: Command) -> Result<Self, RegistryError> {
        let id = CommandId::new(command.id.as_str())?;
        if self.get(id.as_str()).is_some() {
            return Err(RegistryError::DuplicateId(id.0));
        }
        if command.title.trim().is_empty() {
            return Err(RegistryError::EmptyTitle(id.0));
        }
        self.commands.push(command);
        Ok(self)
    }

    /// Add each command.
    ///
    /// # Panics
    ///
    /// Panics as [`add`](Self::add) does.
    #[must_use]
    #[track_caller]
    pub fn extend(self, commands: impl IntoIterator<Item = Command>) -> Self {
        commands.into_iter().fold(self, Self::add)
    }

    /// The command with `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Command> {
        self.commands.iter().find(|command| command.id == id)
    }

    /// The commands, in order.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// The number of commands.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.commands.len()
    }

    /// `true` when there are no commands.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autumn_web::reexports::http::Request;

    fn parts(path: &str) -> Parts {
        Request::get(path).body(()).expect("request").into_parts().0
    }

    #[test]
    fn route_and_action_have_default_groups() {
        let route = Command::route("home", "Dashboard", "/");
        assert_eq!(route.group_name(), ROUTES_GROUP);
        assert_eq!(route.target(), &CommandTarget::Route("/".into()));
        let action = Command::action("out", "Sign out", "/logout");
        assert_eq!(action.group_name(), ACTIONS_GROUP);
        assert_eq!(action.target(), &CommandTarget::Action("/logout".into()));
    }

    #[test]
    fn builder_sets_each_field() {
        let command = Command::route("posts", "Posts", "/posts")
            .group("Content")
            .keywords(["articles", "blog"])
            .keywords(vec![String::from("news")])
            .shortcut(&["G", "P"])
            .hint("All posts");
        assert_eq!(command.id(), "posts");
        assert_eq!(command.title(), "Posts");
        assert_eq!(command.group_name(), "Content");
        assert_eq!(command.keyword_list(), ["articles", "blog", "news"]);
        assert_eq!(command.shortcut_keys(), ["G", "P"]);
        assert_eq!(command.hint_text(), Some("All posts"));
        assert!(!command.has_filter());
        assert!(command.is_visible(&parts("/")));
        let debug = format!("{command:?}");
        assert!(debug.contains("visible_when: false"), "{debug}");
    }

    #[test]
    fn visible_when_filters_by_request() {
        let command = Command::route("admin", "Admin", "/admin")
            .visible_when(|parts| parts.uri.path().starts_with("/admin"));
        assert!(command.has_filter());
        assert!(command.is_visible(&parts("/admin/x")));
        assert!(!command.is_visible(&parts("/")));
    }

    #[test]
    fn command_id_rules() {
        assert_eq!(
            CommandId::new("posts.new").expect("valid").as_str(),
            "posts.new"
        );
        assert_eq!(
            CommandId::new("a:b/c_d-e").expect("valid").to_string(),
            "a:b/c_d-e"
        );
        for bad in ["", "has space", "a,b", "é", "<x>"] {
            assert_eq!(
                CommandId::new(bad),
                Err(RegistryError::BadId(bad.into())),
                "{bad}"
            );
        }
        assert!(CommandId::new("x".repeat(MAX_ID_LEN)).is_ok());
        assert!(CommandId::new("x".repeat(MAX_ID_LEN + 1)).is_err());
    }

    #[test]
    fn registry_keeps_order_and_finds_by_id() {
        let registry = CommandRegistry::new()
            .add(Command::route("b", "Bravo", "/b"))
            .extend([Command::route("a", "Alpha", "/a")]);
        let ids: Vec<&str> = registry.commands().iter().map(Command::id).collect();
        assert_eq!(ids, ["b", "a"]);
        assert_eq!(registry.get("a").map(Command::title), Some("Alpha"));
        assert!(registry.get("zz").is_none());
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
        assert!(CommandRegistry::new().is_empty());
    }

    #[test]
    fn try_add_rejects_bad_commands() {
        let registry = CommandRegistry::new().add(Command::route("a", "Alpha", "/a"));
        assert_eq!(
            registry
                .clone()
                .try_add(Command::route("a", "Again", "/b"))
                .err(),
            Some(RegistryError::DuplicateId("a".into()))
        );
        assert_eq!(
            registry
                .clone()
                .try_add(Command::route("b c", "Bad", "/b"))
                .err(),
            Some(RegistryError::BadId("b c".into()))
        );
        assert_eq!(
            registry.try_add(Command::route("e", "  ", "/e")).err(),
            Some(RegistryError::EmptyTitle("e".into()))
        );
    }

    #[test]
    #[should_panic(expected = "autumn-command-palette: two commands have the id `a`")]
    fn add_panics_on_a_duplicate_id() {
        let _ = CommandRegistry::new()
            .add(Command::route("a", "Alpha", "/a"))
            .add(Command::route("a", "Again", "/a"));
    }
}
