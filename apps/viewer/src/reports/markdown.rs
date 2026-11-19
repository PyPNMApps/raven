/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Validate native Markdown and expose internal links as themed navigation controls.
use super::{failure, ReportResult};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::Serialize;
use std::{collections::BTreeSet, ops::Range};
#[derive(Serialize)]
pub(super) struct Navigation {
    label: String,
    target: String,
}
pub(super) struct ApprovedMarkdown {
    pub display_content: String,
    pub links: Vec<String>,
    pub navigation: Vec<Navigation>,
}
/// Escapes locally supplied prose before embedding it in native Markdown.
///
/// # Arguments
/// * `text` - Literal narrative, never executable markup.
///
/// # Returns
/// Markdown text retaining literal punctuation and angle brackets.
pub(super) fn literal(text: &str) -> String {
    let mut result = String::new();
    for character in text.chars() {
        match character {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '[' | ']' | '!' | '#' | '|' => {
                result.push('\\');
                result.push(character);
            }
            _ => result.push(character),
        }
    }
    result
}
/// Validates bounded Markdown and separates approved internal navigation.
///
/// # Arguments
/// * `content` - Report text limited to 16 KiB and 16 nesting levels.
/// * `ids` - Current report block identifiers allowed as link destinations.
///
/// # Returns
/// Original prose with link markup removed and themed navigation metadata.
///
/// # Errors
/// Rejects HTML, images, external or malformed links, and exceeded budgets.
pub(super) fn validate(content: &str, ids: &BTreeSet<String>) -> ReportResult<ApprovedMarkdown> {
    if content.len() > 16_384 || content.contains('<') || content.contains('\0') {
        return Err(failure(
            "ReportMarkdown",
            "Markdown exceeds its bounds or contains HTML markup.",
        ));
    }
    let mut depth = 0usize;
    let mut links = Vec::new();
    let mut navigation = Vec::new();
    let mut replacements = Vec::new();
    let mut current: Option<(Range<usize>, String, String)> = None;
    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Html(_) | Event::InlineHtml(_) | Event::Start(Tag::Image { .. }) => {
                return Err(failure(
                    "ReportMarkdown",
                    "HTML and Markdown images are not permitted.",
                ))
            }
            Event::Start(tag) => {
                depth += 1;
                if depth > 16 {
                    return Err(failure(
                        "ReportDepth",
                        "Markdown nesting exceeds 16 levels.",
                    ));
                }
                if let Tag::Link { dest_url, .. } = tag {
                    let id = dest_url
                        .strip_prefix('#')
                        .filter(|id| ids.contains(*id))
                        .ok_or_else(|| {
                            failure(
                                "ReportLink",
                                "Only links to existing report block IDs are permitted.",
                            )
                        })?;
                    if current.is_some() {
                        return Err(failure("ReportLink", "Nested links are not supported."));
                    }
                    current = Some((range, format!("#{id}"), String::new()));
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, label)) = &mut current {
                    label.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, label)) = &mut current {
                    label.push(' ');
                }
            }
            Event::End(tag) => {
                depth = depth.saturating_sub(1);
                if tag == TagEnd::Link {
                    let (mut span, target, label) = current
                        .take()
                        .ok_or_else(|| failure("ReportLink", "Link structure is invalid."))?;
                    span.end = range.end;
                    if !links.contains(&target) {
                        if links.len() >= 8 {
                            return Err(failure(
                                "ReportLink",
                                "Markdown exceeds eight unique internal link targets.",
                            ));
                        }
                        links.push(target.clone());
                        navigation.push(Navigation {
                            label: if label.is_empty() {
                                target.clone()
                            } else {
                                label.clone()
                            },
                            target,
                        });
                    }
                    replacements.push((span, literal(&label)));
                }
            }
            _ => {}
        }
    }
    let mut display = content.to_owned();
    for (range, label) in replacements.into_iter().rev() {
        display.replace_range(range, &label);
    }
    Ok(ApprovedMarkdown {
        display_content: display,
        links,
        navigation,
    })
}
