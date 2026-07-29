use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use heck::{ToSnakeCase, ToUpperCamelCase};

use crate::include_template;
use crate::prelude::*;

const COMMAND_TEMPLATE: &str = include_template!("cli/command.rs");
const GROUP_TEMPLATE: &str = include_template!("cli/group.rs");
const COMMAND_TEMPLATE_NAME: &str = "command.rs";
const GROUP_TEMPLATE_NAME: &str = "group.rs";
const MODULES_START: &str = "// <generated-command-modules>";
const MODULES_END: &str = "// </generated-command-modules>";
const VARIANTS_START: &str = "// <generated-command-variants>";
const VARIANTS_END: &str = "// </generated-command-variants>";
const RUST_KEYWORDS: [&str; 53] = [
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
    "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "union", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Generate and register a command.
#[derive(clap::Parser, Debug)]
pub struct NewCmd {
    /// Slash-separated command path, for example `admin/user-add`.
    path: String,
}

impl CliCommand for NewCmd {
    fn run(&self) -> CliResult {
        let current_directory =
            std::env::current_dir().context("failed to resolve the current directory")?;
        let project_root = find_project_root(&current_directory)?;
        generate_command(&project_root, &self.path)
    }
}

#[derive(Clone, Debug)]
struct Segment {
    module: String,
    type_stem: String,
}

impl Segment {
    fn parse(value: &str) -> anyhow::Result<Self> {
        let valid_characters = value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
        let starts_with_letter =
            value.chars().next().is_some_and(|character| character.is_ascii_alphabetic());
        if !starts_with_letter || !valid_characters {
            anyhow::bail!(
                "invalid command segment `{value}`; use ASCII letters, numbers, hyphens, or underscores and start with a letter"
            );
        }

        let module = value.to_snake_case();
        if RUST_KEYWORDS.contains(&module.as_str()) {
            anyhow::bail!("command segment `{value}` resolves to the Rust keyword `{module}`");
        }

        Ok(Self { type_stem: value.to_upper_camel_case(), module })
    }

    fn command_type(&self) -> String {
        format!("{}Cmd", self.type_stem)
    }
}

#[derive(Debug)]
struct PlannedWrite {
    path: PathBuf,
    content: String,
}

fn find_project_root(start: &Path) -> anyhow::Result<PathBuf> {
    for directory in start.ancestors() {
        if directory.join("Cargo.toml").is_file() && directory.join("src/cmd/mod.rs").is_file() {
            return Ok(directory.to_path_buf());
        }
    }
    anyhow::bail!(
        "failed to find a project root containing Cargo.toml and src/cmd/mod.rs from {}",
        start.display()
    )
}

fn parse_path(path: &str) -> anyhow::Result<Vec<Segment>> {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') {
        anyhow::bail!("command path must be a non-empty relative path separated by `/`");
    }

    path.split('/')
        .map(|segment| {
            if matches!(segment, "" | "." | "..") {
                anyhow::bail!("command path contains an invalid segment `{segment}`");
            }
            Segment::parse(segment)
        })
        .collect()
}

fn generate_command(project_root: &Path, path: &str) -> anyhow::Result<()> {
    let segments = parse_path(path)?;
    let cmd_root = project_root.join("src/cmd");
    let root_module = cmd_root.join("mod.rs");
    let mut engine = tera::Tera::default();
    engine.add_raw_template(COMMAND_TEMPLATE_NAME, COMMAND_TEMPLATE)?;
    engine.add_raw_template(GROUP_TEMPLATE_NAME, GROUP_TEMPLATE)?;

    validate_topology(&cmd_root, &segments)?;

    let mut writes = Vec::new();
    let root_content = fs::read_to_string(&root_module)
        .with_context(|| format!("failed to read {}", root_module.display()))?;
    writes.push(PlannedWrite {
        path: root_module,
        content: add_registration(&root_content, &segments[0], segments.len() > 1)?,
    });

    for group_index in 0..segments.len().saturating_sub(1) {
        let group = &segments[group_index];
        let child = &segments[group_index + 1];
        let child_is_group = group_index + 1 < segments.len() - 1;
        let group_directory = segments[..=group_index]
            .iter()
            .fold(cmd_root.clone(), |path, segment| path.join(&segment.module));
        let group_module = group_directory.join("mod.rs");

        let content = if group_module.exists() {
            let current = fs::read_to_string(&group_module)
                .with_context(|| format!("failed to read {}", group_module.display()))?;
            add_registration(&current, child, child_is_group)?
        } else {
            render_group(&engine, group, child, child_is_group)?
        };
        writes.push(PlannedWrite { path: group_module, content });
    }

    let leaf = segments.last().context("command path has no leaf")?;
    let leaf_directory = segments[..segments.len() - 1]
        .iter()
        .fold(cmd_root, |path, segment| path.join(&segment.module));
    let leaf_path = leaf_directory.join(format!("{}.rs", leaf.module));
    writes.push(PlannedWrite { path: leaf_path, content: render_command(&engine, leaf)? });

    apply_writes(&writes)
}

fn validate_topology(cmd_root: &Path, segments: &[Segment]) -> anyhow::Result<()> {
    let mut parent = cmd_root.to_path_buf();
    for segment in &segments[..segments.len() - 1] {
        let conflicting_leaf = parent.join(format!("{}.rs", segment.module));
        if conflicting_leaf.exists() {
            anyhow::bail!(
                "cannot create command group because a leaf command already exists: {}",
                conflicting_leaf.display()
            );
        }
        parent.push(&segment.module);
    }

    let leaf = segments.last().context("command path has no leaf")?;
    let leaf_file = parent.join(format!("{}.rs", leaf.module));
    let leaf_group = parent.join(&leaf.module);
    if leaf_file.exists() {
        anyhow::bail!("command already exists: {}", leaf_file.display());
    }
    if leaf_group.exists() {
        anyhow::bail!(
            "cannot create leaf command because a command group already exists: {}",
            leaf_group.display()
        );
    }
    Ok(())
}

fn render_command(engine: &tera::Tera, command: &Segment) -> anyhow::Result<String> {
    let mut context = tera::Context::new();
    context.insert("command_type", &command.command_type());
    engine.render(COMMAND_TEMPLATE_NAME, &context).context("failed to render the command template")
}

fn render_group(
    engine: &tera::Tera,
    group: &Segment,
    child: &Segment,
    child_is_group: bool,
) -> anyhow::Result<String> {
    let mut context = tera::Context::new();
    context.insert("group_type", &group.command_type());
    context.insert("child_module", &child.module);
    context.insert("child_variant", &render_variant(child, child_is_group));
    context.insert("modules_start", MODULES_START);
    context.insert("modules_end", MODULES_END);
    context.insert("variants_start", VARIANTS_START);
    context.insert("variants_end", VARIANTS_END);
    engine
        .render(GROUP_TEMPLATE_NAME, &context)
        .context("failed to render the command group template")
}

fn add_registration(
    content: &str,
    child: &Segment,
    child_is_group: bool,
) -> anyhow::Result<String> {
    let module_declaration = format!("mod {};", child.module);
    if content.lines().any(|line| line.trim() == module_declaration) {
        anyhow::bail!("command module `{}` is already registered", child.module);
    }

    let content = insert_before_marker(
        content,
        MODULES_START,
        MODULES_END,
        &format!("{module_declaration}\n"),
    )?;
    insert_before_marker(
        &content,
        VARIANTS_START,
        VARIANTS_END,
        &render_variant(child, child_is_group),
    )
}

fn render_variant(child: &Segment, child_is_group: bool) -> String {
    let attribute = if child_is_group { "    #[command(subcommand)]\n" } else { "" };
    format!("{attribute}    {}({}::{}),\n", child.type_stem, child.module, child.command_type())
}

fn insert_before_marker(
    content: &str,
    start_marker: &str,
    end_marker: &str,
    addition: &str,
) -> anyhow::Result<String> {
    let start = content
        .find(start_marker)
        .with_context(|| format!("missing generator marker `{start_marker}`"))?;
    let end = content
        .find(end_marker)
        .with_context(|| format!("missing generator marker `{end_marker}`"))?;
    if start >= end {
        anyhow::bail!("generator markers `{start_marker}` and `{end_marker}` are out of order");
    }

    let line_start = content[..end].rfind('\n').map_or(0, |position| position + 1);
    let mut updated = String::with_capacity(content.len() + addition.len());
    updated.push_str(&content[..line_start]);
    updated.push_str(addition);
    updated.push_str(&content[line_start..]);
    Ok(updated)
}

fn apply_writes(writes: &[PlannedWrite]) -> anyhow::Result<()> {
    let process_id = std::process::id();
    let mut created_directories = Vec::new();
    let mut temporary_paths = Vec::new();

    for write in writes {
        create_missing_directories(&write.path, &mut created_directories)?;
    }

    for (index, write) in writes.iter().enumerate() {
        let temporary = sibling_path(&write.path, "tmp", process_id, index)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .with_context(|| format!("failed to create {}", temporary.display()))?;
        if let Err(error) = file.write_all(write.content.as_bytes()) {
            cleanup_files(&temporary_paths);
            cleanup_directories(&created_directories);
            return Err(error).with_context(|| format!("failed to write {}", temporary.display()));
        }
        temporary_paths.push(temporary);
    }

    let mut committed: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    for (index, (write, temporary)) in writes.iter().zip(&temporary_paths).enumerate() {
        let backup = if write.path.exists() {
            let backup = sibling_path(&write.path, "backup", process_id, index)?;
            if let Err(error) = fs::rename(&write.path, &backup) {
                rollback(&committed, &temporary_paths, &created_directories);
                return Err(error)
                    .with_context(|| format!("failed to back up {}", write.path.display()));
            }
            Some(backup)
        } else {
            None
        };

        if let Err(error) = fs::rename(temporary, &write.path) {
            if let Some(backup_path) = &backup {
                let _ = fs::rename(backup_path, &write.path);
            }
            rollback(&committed, &temporary_paths, &created_directories);
            return Err(error)
                .with_context(|| format!("failed to commit {}", write.path.display()));
        }
        committed.push((write.path.clone(), backup));
    }

    for (_, backup) in &committed {
        if let Some(path) = backup {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

fn create_missing_directories(
    target: &Path,
    created_directories: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let mut missing = Vec::new();
    let mut current = target.parent();
    while let Some(directory) = current {
        if directory.exists() {
            break;
        }
        missing.push(directory.to_path_buf());
        current = directory.parent();
    }

    for directory in missing.into_iter().rev() {
        fs::create_dir(&directory)
            .with_context(|| format!("failed to create {}", directory.display()))?;
        created_directories.push(directory);
    }
    Ok(())
}

fn sibling_path(path: &Path, kind: &str, process_id: u32, index: usize) -> anyhow::Result<PathBuf> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("target filename is not valid UTF-8")?;
    Ok(path.with_file_name(format!(".{name}.{kind}-{process_id}-{index}")))
}

fn rollback(
    committed: &[(PathBuf, Option<PathBuf>)],
    temporary_paths: &[PathBuf],
    created_directories: &[PathBuf],
) {
    for (path, backup) in committed.iter().rev() {
        let _ = fs::remove_file(path);
        if let Some(backup_path) = backup {
            let _ = fs::rename(backup_path, path);
        }
    }
    cleanup_files(temporary_paths);
    cleanup_directories(created_directories);
}

fn cleanup_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn cleanup_directories(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        let _ = fs::remove_dir(path);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    struct TestProject(PathBuf);

    impl TestProject {
        fn new() -> anyhow::Result<Self> {
            let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("cli-new-test-{}-{id}", std::process::id()));
            fs::create_dir_all(root.join("src/cmd"))?;
            fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\n")?;
            fs::write(
                root.join("src/cmd/mod.rs"),
                format!(
                    "{MODULES_START}\n{MODULES_END}\n\nenum Command {{\n    {VARIANTS_START}\n    {VARIANTS_END}\n}}\n"
                ),
            )?;
            Ok(Self(root))
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn generates_and_registers_a_nested_command() -> anyhow::Result<()> {
        let project = TestProject::new()?;
        generate_command(&project.0, "admin/user-add")?;

        let root_module = fs::read_to_string(project.0.join("src/cmd/mod.rs"))?;
        assert!(root_module.contains("mod admin;"));
        assert!(root_module.contains("#[command(subcommand)]\n    Admin(admin::AdminCmd),"));

        let group_module = fs::read_to_string(project.0.join("src/cmd/admin/mod.rs"))?;
        assert!(group_module.contains("mod user_add;"));
        assert!(group_module.contains("UserAdd(user_add::UserAddCmd),"));
        assert!(project.0.join("src/cmd/admin/user_add.rs").is_file());
        Ok(())
    }

    #[test]
    fn rejects_traversal_and_existing_commands() -> anyhow::Result<()> {
        let project = TestProject::new()?;
        assert!(generate_command(&project.0, "../escape").is_err());

        generate_command(&project.0, "status")?;
        let before = fs::read_to_string(project.0.join("src/cmd/status.rs"))?;
        assert!(generate_command(&project.0, "status").is_err());
        assert_eq!(fs::read_to_string(project.0.join("src/cmd/status.rs"))?, before);
        Ok(())
    }
}
