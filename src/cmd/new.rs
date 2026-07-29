use std::cmp::Reverse;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use heck::{ToSnakeCase, ToUpperCamelCase};
use syn::spanned::Spanned;

use crate::include_template;
use crate::prelude::*;

const COMMAND_TEMPLATE: &str = include_template!("cli/command.rs");
const GROUP_TEMPLATE: &str = include_template!("cli/group.rs");
const COMMAND_TEMPLATE_NAME: &str = "command.rs";
const GROUP_TEMPLATE_NAME: &str = "group.rs";
const ROOT_COMMAND_TYPE: &str = "Command";
const DEFAULT_INDENT: &str = "    ";
const WINDOWS_LINE_ENDING: &str = "\r\n";
const UNIX_LINE_ENDING: &str = "\n";
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

#[derive(Debug)]
struct TextEdit {
    offset: usize,
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
        path: root_module.clone(),
        content: add_registration(
            &root_content,
            &root_module,
            ROOT_COMMAND_TYPE,
            &segments[0],
            segments.len() > 1,
        )?,
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
            add_registration(&current, &group_module, &group.command_type(), child, child_is_group)?
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
    context.insert(
        "child_variant",
        &render_variant(child, child_is_group, DEFAULT_INDENT, UNIX_LINE_ENDING),
    );
    engine
        .render(GROUP_TEMPLATE_NAME, &context)
        .context("failed to render the command group template")
}

fn add_registration(
    content: &str,
    path: &Path,
    command_type: &str,
    child: &Segment,
    child_is_group: bool,
) -> anyhow::Result<String> {
    let syntax = syn::parse_file(content)
        .with_context(|| format!("failed to parse {} as Rust source", path.display()))?;
    let command_enums = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident == command_type => Some(item_enum),
            _ => None,
        })
        .collect::<Vec<_>>();
    let command_enum = match command_enums.as_slice() {
        [command_enum] => *command_enum,
        command_enums => anyhow::bail!(
            "expected exactly one top-level enum `{command_type}` in {}, found {}",
            path.display(),
            command_enums.len()
        ),
    };

    let module_registered = syntax
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Mod(item_mod) if item_mod.ident == child.module));
    let variant_registered =
        command_enum.variants.iter().any(|variant| variant.ident == child.type_stem);
    if module_registered && variant_registered && child_is_group {
        return Ok(content.to_owned());
    }
    if module_registered != variant_registered {
        anyhow::bail!(
            "command `{}` has inconsistent module and variant registrations in {}",
            child.module,
            path.display()
        );
    }
    if module_registered {
        anyhow::bail!("command module `{}` is already registered", child.module);
    }
    if variant_registered {
        anyhow::bail!("command variant `{}` is already registered", child.type_stem);
    }

    let line_ending =
        if content.contains(WINDOWS_LINE_ENDING) { WINDOWS_LINE_ENDING } else { UNIX_LINE_ENDING };
    let mut edits = vec![
        module_edit(content, &syntax, child, line_ending)?,
        variant_edit(content, command_enum, child, child_is_group, line_ending)?,
    ];
    if let Some(comma_edit) = trailing_comma_edit(command_enum) {
        edits.push(comma_edit);
    }

    let updated = apply_text_edits(content, edits)?;
    syn::parse_file(&updated)
        .with_context(|| format!("generated invalid Rust source for {}", path.display()))?;
    Ok(updated)
}

fn module_edit(
    content: &str,
    syntax: &syn::File,
    child: &Segment,
    line_ending: &str,
) -> anyhow::Result<TextEdit> {
    let mut external_modules = syntax.items.iter().filter_map(|item| match item {
        syn::Item::Mod(item_mod) if item_mod.content.is_none() => Some(item_mod),
        _ => None,
    });

    if let Some(next_module) = external_modules
        .clone()
        .find(|item_mod| item_mod.ident.to_string().as_str() > child.module.as_str())
    {
        let item_start = checked_offset(next_module.span().byte_range().start, content)?;
        let offset = leading_line_comment_start(content, item_start);
        return Ok(TextEdit { offset, content: format!("mod {};{line_ending}", child.module) });
    }

    if let Some(item_mod) = external_modules.next_back() {
        let semi = item_mod.semi.context("external module declaration is missing a semicolon")?;
        let syntactic_end = checked_offset(semi.span().byte_range().end, content)?;
        if let Some(relative_line_end) = content[syntactic_end..].find('\n') {
            let offset = syntactic_end + relative_line_end + 1;
            return Ok(TextEdit { offset, content: format!("mod {};{line_ending}", child.module) });
        }
        return Ok(TextEdit {
            offset: content.len(),
            content: format!("{line_ending}mod {};", child.module),
        });
    }

    let offset = syntax.items.first().map_or(content.len(), |item| item.span().byte_range().start);
    Ok(TextEdit {
        offset: checked_offset(offset, content)?,
        content: format!("mod {};{line_ending}{line_ending}", child.module),
    })
}

fn variant_edit(
    content: &str,
    command_enum: &syn::ItemEnum,
    child: &Segment,
    child_is_group: bool,
    line_ending: &str,
) -> anyhow::Result<TextEdit> {
    let close_offset =
        checked_offset(command_enum.brace_token.span.close().byte_range().start, content)?;
    let closing_indent = line_indent(content, close_offset);
    let variant_indent = format!("{closing_indent}{DEFAULT_INDENT}");
    let variant = render_variant(child, child_is_group, &variant_indent, line_ending);

    let Some(last_pair) = command_enum.variants.pairs().next_back() else {
        let prefix = if content[..close_offset].ends_with('\n') {
            String::new()
        } else {
            line_ending.to_owned()
        };
        return Ok(TextEdit { offset: close_offset, content: format!("{prefix}{variant}") });
    };

    let syntactic_end =
        last_pair.punct().map_or_else(|| last_pair.value().span(), Spanned::span).byte_range().end;
    let syntactic_end = checked_offset(syntactic_end, content)?;
    if let Some(relative_line_end) = content[syntactic_end..close_offset].find('\n') {
        let offset = syntactic_end + relative_line_end + 1;
        return Ok(TextEdit { offset, content: variant });
    }

    Ok(TextEdit {
        offset: close_offset,
        content: format!("{line_ending}{variant}{closing_indent}"),
    })
}

fn trailing_comma_edit(command_enum: &syn::ItemEnum) -> Option<TextEdit> {
    let last_variant = command_enum.variants.last()?;
    (!command_enum.variants.trailing_punct())
        .then(|| TextEdit { offset: last_variant.span().byte_range().end, content: ",".to_owned() })
}

fn render_variant(
    child: &Segment,
    child_is_group: bool,
    indent: &str,
    line_ending: &str,
) -> String {
    let attribute = if child_is_group {
        format!("{indent}#[command(subcommand)]{line_ending}")
    } else {
        String::new()
    };
    format!(
        "{attribute}{indent}{}({}::{}),{line_ending}",
        child.type_stem,
        child.module,
        child.command_type()
    )
}

fn line_indent(content: &str, offset: usize) -> &str {
    let line_start = content[..offset].rfind('\n').map_or(0, |position| position + 1);
    let indent = &content[line_start..offset];
    if indent.chars().all(|character| matches!(character, ' ' | '\t')) { indent } else { "" }
}

fn leading_line_comment_start(content: &str, offset: usize) -> usize {
    let mut start = content[..offset].rfind('\n').map_or(0, |position| position + 1);
    while start > 0 {
        let previous_end =
            if content[..start].ends_with(WINDOWS_LINE_ENDING) { start - 2 } else { start - 1 };
        let previous_start = content[..previous_end].rfind('\n').map_or(0, |position| position + 1);
        if !content[previous_start..previous_end].trim_start().starts_with("//") {
            break;
        }
        start = previous_start;
    }
    start
}

fn checked_offset(offset: usize, content: &str) -> anyhow::Result<usize> {
    if offset > content.len() || !content.is_char_boundary(offset) {
        anyhow::bail!("syntax span resolved to an invalid source offset {offset}");
    }
    Ok(offset)
}

fn apply_text_edits(content: &str, mut edits: Vec<TextEdit>) -> anyhow::Result<String> {
    edits.sort_by_key(|edit| Reverse(edit.offset));
    let additional_capacity: usize = edits.iter().map(|edit| edit.content.len()).sum();
    let mut updated = String::with_capacity(content.len() + additional_capacity);
    updated.push_str(content);
    for edit in edits {
        let offset = checked_offset(edit.offset, &updated)?;
        updated.insert_str(offset, &edit.content);
    }
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
            Self::with_root(concat!(
                "mod existing;\n",
                "\n",
                "enum Command {\n",
                "    Existing(existing::ExistingCmd),\n",
                "}\n",
            ))
        }

        fn with_root(content: &str) -> anyhow::Result<Self> {
            let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("cli-new-test-{}-{id}", std::process::id()));
            fs::create_dir_all(root.join("src/cmd"))?;
            fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\n")?;
            fs::write(root.join("src/cmd/mod.rs"), content)?;
            Ok(Self(root))
        }

        fn root_module(&self) -> PathBuf {
            self.0.join("src/cmd/mod.rs")
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
        generate_command(&project.0, "admin/audit-log")?;

        let root_module = fs::read_to_string(project.root_module())?;
        assert!(root_module.starts_with("mod admin;\nmod existing;"));
        assert!(root_module.contains("mod admin;"));
        assert!(root_module.contains("#[command(subcommand)]\n    Admin(admin::AdminCmd),"));
        assert_eq!(root_module.matches("mod admin;").count(), 1);
        assert_eq!(root_module.matches("Admin(admin::AdminCmd),").count(), 1);

        let group_module = fs::read_to_string(project.0.join("src/cmd/admin/mod.rs"))?;
        assert!(group_module.contains("mod user_add;"));
        assert!(group_module.contains("mod audit_log;"));
        assert!(group_module.contains("UserAdd(user_add::UserAddCmd),"));
        assert!(group_module.contains("AuditLog(audit_log::AuditLogCmd),"));
        assert!(project.0.join("src/cmd/admin/user_add.rs").is_file());
        assert!(project.0.join("src/cmd/admin/audit_log.rs").is_file());
        Ok(())
    }

    #[test]
    fn preserves_comments_attributes_and_missing_trailing_comma() -> anyhow::Result<()> {
        let root = concat!(
            "// Keep the leading module comment.\n",
            "mod existing; // Keep the inline module comment.\n",
            "\n",
            "use crate::prelude::*;\n",
            "\n",
            "#[derive(Debug)]\n",
            "enum Command {\n",
            "    #[cfg(feature = \"existing\")]\n",
            "    Existing(existing::ExistingCmd) // Keep the variant comment.\n",
            "    // Keep the footer comment.\n",
            "}\n",
        );
        let project = TestProject::with_root(root)?;
        generate_command(&project.0, "alpha")?;

        let updated = fs::read_to_string(project.root_module())?;
        assert!(updated.starts_with(concat!(
            "mod alpha;\n",
            "// Keep the leading module comment.\n",
            "mod existing; // Keep the inline module comment.\n",
        )));
        assert!(updated.contains(concat!(
            "    Existing(existing::ExistingCmd), // Keep the variant comment.\n",
            "    Alpha(alpha::AlphaCmd),\n",
            "    // Keep the footer comment.",
        )));
        assert!(syn::parse_file(&updated).is_ok());
        Ok(())
    }

    #[test]
    fn registers_a_command_when_no_module_declarations_exist() -> anyhow::Result<()> {
        let project = TestProject::with_root(concat!(
            "use crate::prelude::*;\n",
            "\n",
            "enum Command {\n",
            "    Existing(existing::ExistingCmd),\n",
            "}\n",
        ))?;
        generate_command(&project.0, "status")?;

        let updated = fs::read_to_string(project.root_module())?;
        assert!(updated.starts_with("mod status;\n\nuse crate::prelude::*;"));
        assert!(updated.contains("Status(status::StatusCmd),"));
        Ok(())
    }

    #[test]
    fn inserts_before_an_attributed_module() -> anyhow::Result<()> {
        let project = TestProject::with_root(concat!(
            "mod alpha;\n",
            "#[cfg(feature = \"omega\")]\n",
            "mod omega;\n",
            "\n",
            "enum Command {\n",
            "    Existing(existing::ExistingCmd),\n",
            "}\n",
        ))?;
        generate_command(&project.0, "beta")?;

        let updated = fs::read_to_string(project.root_module())?;
        assert!(updated.starts_with(concat!(
            "mod alpha;\n",
            "mod beta;\n",
            "#[cfg(feature = \"omega\")]\n",
            "mod omega;\n",
        )));
        Ok(())
    }

    #[test]
    fn preserves_windows_line_endings() -> anyhow::Result<()> {
        let project = TestProject::with_root(concat!(
            "mod existing;\r\n",
            "\r\n",
            "enum Command {\r\n",
            "    Existing(existing::ExistingCmd),\r\n",
            "}\r\n",
        ))?;
        generate_command(&project.0, "status")?;

        let updated = fs::read_to_string(project.root_module())?;
        assert!(!updated.replace("\r\n", "").contains('\n'));
        assert!(updated.contains("mod existing;\r\nmod status;\r\n"));
        assert!(updated.contains("    Status(status::StatusCmd),\r\n"));
        Ok(())
    }

    #[test]
    fn rejects_invalid_source_without_partial_writes() -> anyhow::Result<()> {
        for root in [
            "mod existing;\n\nenum Other {}\n",
            "enum Command {}\nenum Command {}\n",
            "enum Command {\n",
        ] {
            let project = TestProject::with_root(root)?;
            let before = fs::read_to_string(project.root_module())?;
            assert!(generate_command(&project.0, "status").is_err());
            assert_eq!(fs::read_to_string(project.root_module())?, before);
            assert!(!project.0.join("src/cmd/status.rs").exists());
        }
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
