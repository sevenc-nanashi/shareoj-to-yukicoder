mod log_writer;
use anyhow::{Context, Result, bail, ensure};
use base64::Engine;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(usage_rs::Cli)]
#[usage(version)]
struct Cli {
    #[usage(default = ".")]
    directory: PathBuf,
    #[usage(long)]
    force: bool,
    #[usage(long)]
    no_color: bool,
}
#[derive(Deserialize)]
struct Problem {
    statement: PathBuf,
    tests: Tests,
    interactor: Option<toml::Value>,
    checker: Option<toml::Value>,
}
#[derive(Deserialize)]
struct Tests {
    directory: PathBuf,
    output_suffix: String,
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn convert(source: &str, base: &Path) -> Result<String> {
    let mut edits = Vec::new();
    let mut heading = None;
    let mut title = String::new();
    let mut sections: Vec<(std::ops::Range<usize>, String, bool, bool)> = Vec::new();
    let mut protected = Vec::new();
    let mut sample_section = false;
    let mut image_start = None;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                heading = Some(range);
                title.clear();
            }
            Event::Text(ref t) | Event::Code(ref t) if heading.is_some() => title.push_str(t),
            Event::End(TagEnd::Heading(level)) => {
                if level == pulldown_cmark::HeadingLevel::H2 {
                    sample_section = title.strip_prefix("サンプル").is_some_and(|n| {
                        !n.trim().is_empty() && n.trim().chars().all(|c| c.is_ascii_digit())
                    });
                }

                sections.push((
                    heading.take().context("Missing heading")?,
                    title.clone(),
                    false,
                    false,
                ));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                protected.push(range.clone());
                if sample_section
                    && let CodeBlockKind::Fenced(info) = &kind
                    && !info.is_empty()
                    && info.as_ref() != "input"
                {
                    let line = source[range.clone()]
                        .lines()
                        .next()
                        .context("Empty fence")?;
                    let start =
                        range.start + line.find(info.as_ref()).context("Missing fence info")?;
                    edits.push((start..range.start + line.len(), String::new()));
                }

                if let Some(section) = sections.last_mut() {
                    if matches!(&kind, CodeBlockKind::Fenced(s) if s.as_ref() == "input") {
                        section.2 = true;
                        let line = &source[range.clone()]
                            .lines()
                            .next()
                            .context("Empty fence")?;
                        let start =
                            range.start + line.find("input").context("Missing input marker")?;
                        edits.push((start..start + 5, String::new()));
                    } else {
                        section.3 = true;
                    }
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                image_start = Some((range, dest_url.to_string(), String::new()));
            }
            Event::Text(t) if image_start.is_some() => image_start.as_mut().unwrap().2.push_str(&t),
            Event::End(TagEnd::Image) => {
                let (range, url, alt) = image_start.take().context("Missing image")?;
                if url.starts_with("data:")
                    || (url.starts_with("http") && !url.contains("share-oj.net/"))
                {
                    continue;
                }
                let (data, mime) = if url.starts_with("/api/images/") || url.starts_with("https://")
                {
                    let url = if url.starts_with('/') {
                        format!("https://www.share-oj.net{url}")
                    } else {
                        url
                    };
                    let response = client
                        .get(&url)
                        .send()
                        .with_context(|| format!("Fetching {url}"))?
                        .error_for_status()?;
                    let mime = response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .context("Missing image Content-Type")?
                        .to_str()?
                        .split(';')
                        .next()
                        .context("Missing MIME")?
                        .to_owned();
                    ensure!(mime.starts_with("image/"), "Not an image: {url}");
                    (response.bytes()?.to_vec(), mime)
                } else {
                    let path = base.join(&url);
                    let mime = match path.extension().and_then(|x| x.to_str()) {
                        Some("png") => "image/png",
                        Some("jpg" | "jpeg") => "image/jpeg",
                        Some("webp") => "image/webp",
                        Some("gif") => "image/gif",
                        Some("svg") => "image/svg+xml",
                        _ => bail!("Unsupported image: {url}"),
                    };
                    (
                        fs::read(&path).with_context(|| format!("Reading {}", path.display()))?,
                        mime.to_owned(),
                    )
                };
                edits.push((
                    range,
                    format!(
                        "<img src=\"data:{mime};base64,{}\" alt=\"{}\">",
                        base64::engine::general_purpose::STANDARD.encode(data),
                        escape(&alt)
                    ),
                ));
            }
            _ => {}
        }
    }
    let mut in_sample = false;
    let mut samples_started = false;
    for (range, title, input, other) in sections {
        let original = source[range.clone()].trim_end();
        let body = original.trim_start_matches('#').trim();
        let sample = title
            .strip_prefix("サンプル")
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()));
        let replacement = if let Some(n) = sample {
            in_sample = true;
            let prefix = if samples_started {
                ""
            } else {
                "## @samples サンプル\n\n"
            };
            samples_started = true;
            format!(
                "{prefix}@sample {}",
                body.replace(&format!("サンプル {n}"), &format!("サンプル{n}"))
            )
        } else if in_sample && (title == "入力" || title == "出力") && original.starts_with("### ")
        {
            format!("@{} {title}", if title == "入力" { "in" } else { "out" })
        } else {
            if original.starts_with("## ") {
                in_sample = false;
            }
            if input {
                let marks = original.len() - original.trim_start_matches('#').len();
                format!("{} @input {body}", "#".repeat(marks))
            } else {
                original.to_owned()
            }
        };
        if input && other {
            tracing::warn!("Mixed code fences in section: {title}");
        }
        edits.push((range, format!("{replacement}\n")));
    }
    let mut offset = 0;
    let mut depth = 0;
    for line in source.split_inclusive('\n') {
        if !protected.iter().any(|r| r.contains(&offset)) {
            let t = line.trim();
            if let Some(title) = t.strip_prefix(":::details ") {
                depth += 1;
                edits.push((offset..offset + line.len(), format!("@details {title}\n")));
            } else if t == ":::" {
                ensure!(depth > 0, "Unmatched details terminator");
                depth -= 1;
                edits.push((offset..offset + line.len(), "@enddetails\n".into()));
            }
        }
        offset += line.len();
    }
    ensure!(depth == 0, "Unclosed details");
    edits.sort_by_key(|(r, _)| r.start);
    let mut result = source.to_owned();
    for (range, text) in edits.into_iter().rev() {
        result.replace_range(range, &text);
    }
    Ok(result)
}
fn run(cli: Cli) -> Result<()> {
    let root = cli.directory;
    let problem: Problem = toml::from_str(&fs::read_to_string(root.join("problem.toml"))?)?;
    let destinations = [root.join("yukicoder.md"), root.join("yukicoder_tests")];
    ensure!(
        cli.force || destinations.iter().all(|p| !p.exists()),
        "Output exists; use --force to replace it"
    );
    if problem.interactor.is_some() || problem.checker.is_some() {
        tracing::warn!("Judge code requires manual migration");
    }
    let statement = root.join(problem.statement);
    let converted = convert(
        &fs::read_to_string(&statement)?,
        statement.parent().context("Missing statement parent")?,
    )?;
    ensure!(
        problem.tests.output_suffix != ".in" && !problem.tests.output_suffix.is_empty(),
        "Invalid output suffix"
    );
    let tests = root.join(problem.tests.directory);
    let stage = tempfile::tempdir_in(&root)?;
    fs::write(stage.path().join("yukicoder.md"), converted)?;
    for name in ["inputs", "outputs"] {
        fs::create_dir_all(stage.path().join("yukicoder_tests").join(name))?;
    }
    let mut count = 0;
    for entry in fs::read_dir(&tests)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .context("Missing filename")?
            .to_str()
            .context("Non-UTF8 filename")?;
        if let Some(stem) = name.strip_suffix(&problem.tests.output_suffix) {
            ensure!(
                tests.join(format!("{stem}.in")).is_file(),
                "Missing input for {name}"
            );
        }
        if let Some(stem) = name.strip_suffix(".in") {
            for (folder, from) in [
                ("inputs", path.clone()),
                (
                    "outputs",
                    tests.join(format!("{stem}{}", problem.tests.output_suffix)),
                ),
            ] {
                fs::copy(
                    &from,
                    stage
                        .path()
                        .join("yukicoder_tests")
                        .join(folder)
                        .join(format!("{stem}.txt")),
                )
                .with_context(|| format!("Copying {}", from.display()))?;
            }
            count += 1;
        }
    }
    if count == 0 {
        tracing::warn!("No test cases found");
    }
    let mut installed = Vec::new();
    let mut backups = Vec::new();
    let commit: Result<()> = (|| {
        for (index, dest) in destinations.iter().enumerate() {
            let backup = stage.path().join(format!("backup-{index}"));
            if dest.symlink_metadata().is_ok() {
                fs::rename(dest, &backup)?;
                backups.push((dest.clone(), backup));
            }
            fs::rename(
                stage.path().join(dest.file_name().context("Missing name")?),
                dest,
            )?;
            installed.push(dest.clone());
        }
        Ok(())
    })();
    if let Err(error) = commit {
        let rollback: Result<()> = (|| {
            for dest in installed.iter().rev() {
                if dest.is_dir() {
                    fs::remove_dir_all(dest)?;
                } else {
                    fs::remove_file(dest)?;
                }
            }
            for (dest, backup) in backups.iter().rev() {
                fs::rename(backup, dest)?;
            }
            Ok(())
        })();
        if let Err(restore) = rollback {
            let recovery = stage.keep();
            bail!(
                "{error:#}; rollback failed: {restore:#}; backups: {}",
                recovery.display()
            );
        }
        return Err(error);
    }
    tracing::info!("Converted problem and {count} test cases");
    Ok(())
}
fn main() {
    let cli = Cli::parse();
    if let Err(e) = log_writer::init(cli.no_color) {
        eprintln!("{e:#}");
        std::process::exit(2);
    }
    if let Err(e) = run(cli) {
        tracing::error!("{e:#}");
        std::process::exit(2);
    }
}
