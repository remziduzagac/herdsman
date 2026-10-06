//! Named worktree groups in the spaces sidebar: inside a repository's group,
//! linked worktrees that share a `worktree_group` token sit together under a
//! header row that collapses like the repository itself.
//!
//! herdsman's own module. `sidebar.rs`, `endpoint_sidebar.rs` and `mouse.rs`
//! reach it through hooks marked `// fork: worktree groups`.
//!
//! The sidebar's entry list stays one entry per workspace, so keyboard
//! navigation, numbering and scrolling are unchanged: [`arrange`] only orders
//! and hides entries. Header rows are drawn as extra lines of a neighbouring
//! entry, which [`SidebarGroups`] works out for the renderers.

use std::collections::{HashMap, HashSet};

use super::*;
use crate::api::schema::{AgentStatus, WORKTREE_GROUP_TOKEN};

/// The collapse key of a group, kept beside the repository keys in the
/// client's saved collapse state.
pub(in crate::client::shell) fn collapse_key(repo_key: &str, group: &str) -> String {
    format!("worktree-group:{repo_key}:{group}")
}

fn group_of(workspace: &ClientShellWorkspace) -> Option<&str> {
    workspace
        .worktree
        .as_ref()
        .filter(|worktree| worktree.is_linked_worktree)?;
    workspace
        .tokens
        .iter()
        .find(|(key, _)| key == WORKTREE_GROUP_TOKEN)
        .map(|(_, group)| group.as_str())
        .filter(|group| !group.is_empty())
}

fn repo_key_of(workspace: &ClientShellWorkspace) -> Option<&str> {
    workspace
        .worktree
        .as_ref()
        .map(|worktree| worktree.key.as_str())
}

/// Order a repository's children by group, groups first in the order their
/// first member appears, then the ungrouped ones, and hide the members of a
/// collapsed group except a focused one. `workspace_entries` runs every list
/// through this.
pub(in crate::client::shell) fn arrange(
    snapshot: &ClientShellSnapshot,
    entries: Vec<WorkspaceEntry>,
    collapsed: &HashSet<String>,
) -> Vec<WorkspaceEntry> {
    let mut arranged = Vec::with_capacity(entries.len());
    let mut position = 0;
    while position < entries.len() {
        if !entries[position].indented {
            arranged.push(entries[position]);
            position += 1;
            continue;
        }
        let end = entries[position..]
            .iter()
            .position(|entry| !entry.indented)
            .map_or(entries.len(), |offset| position + offset);
        arranged.extend(arrange_children(
            snapshot,
            &entries[position..end],
            collapsed,
        ));
        position = end;
    }
    arranged
}

fn arrange_children(
    snapshot: &ClientShellSnapshot,
    children: &[WorkspaceEntry],
    collapsed: &HashSet<String>,
) -> Vec<WorkspaceEntry> {
    let workspace = |entry: &WorkspaceEntry| snapshot.workspaces.get(entry.index);
    let Some(repo_key) = children.first().and_then(&workspace).and_then(repo_key_of) else {
        return children.to_vec();
    };
    if collapsed.contains(repo_key) {
        return children.to_vec();
    }
    let mut groups: Vec<(&str, Vec<WorkspaceEntry>)> = Vec::new();
    let mut loose = Vec::new();
    for entry in children {
        match workspace(entry).and_then(group_of) {
            Some(group) => match groups.iter_mut().find(|(name, _)| *name == group) {
                Some((_, members)) => members.push(*entry),
                None => groups.push((group, vec![*entry])),
            },
            None => loose.push(*entry),
        }
    }
    let mark_last = |entries: Vec<WorkspaceEntry>| {
        let count = entries.len();
        entries
            .into_iter()
            .enumerate()
            .map(move |(position, mut entry)| {
                entry.last_child = position + 1 == count;
                entry
            })
    };
    let mut arranged = Vec::with_capacity(children.len());
    for (group, members) in groups {
        let visible = if collapsed.contains(&collapse_key(repo_key, group)) {
            members
                .into_iter()
                .filter(|entry| workspace(entry).is_some_and(|workspace| workspace.focused))
                .collect()
        } else {
            members
        };
        arranged.extend(mark_last(visible));
    }
    arranged.extend(mark_last(loose));
    arranged
}

/// A group's header row.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Header {
    name: String,
    key: String,
    collapsed: bool,
    /// The strongest status among the members, shown while collapsed.
    status: AgentStatus,
    /// Nothing follows the group in its repository, so it closes the tree.
    last: bool,
}

#[derive(Debug, Default)]
struct Decor {
    before: Vec<Header>,
    after: Vec<Header>,
    /// Drawn one level deeper, as a group member; true draws the tree's rail
    /// beside it because more of the repository follows the group.
    member: Option<bool>,
}

/// Where a renderer draws group headers and members, by workspace index.
#[derive(Debug, Default)]
pub(in crate::client::shell) struct SidebarGroups {
    decor: HashMap<usize, Decor>,
}

impl SidebarGroups {
    pub(in crate::client::shell) fn new(
        snapshot: &ClientShellSnapshot,
        collapsed: &HashSet<String>,
    ) -> Self {
        let entries = super::workspace_entries(snapshot, collapsed);
        let mut groups = Self::default();
        for (position, entry) in entries.iter().enumerate() {
            if entry.indented {
                continue;
            }
            let Some(repo_key) = parent_repo_key(snapshot, entry.index) else {
                continue;
            };
            if collapsed.contains(repo_key) {
                continue;
            }
            let children = entries[position + 1..]
                .iter()
                .take_while(|child| child.indented)
                .map(|child| child.index)
                .collect::<Vec<_>>();
            groups.place(snapshot, collapsed, repo_key, entry.index, &children);
        }
        groups
    }

    /// Decide where each of a repository's group headers goes: above the
    /// group's first visible member, else above whatever follows the group in
    /// the repository, else below the repository's last row.
    fn place(
        &mut self,
        snapshot: &ClientShellSnapshot,
        collapsed: &HashSet<String>,
        repo_key: &str,
        parent: usize,
        children: &[usize],
    ) {
        let mut order: Vec<&str> = Vec::new();
        for workspace in &snapshot.workspaces {
            if repo_key_of(workspace) == Some(repo_key) {
                if let Some(group) = group_of(workspace) {
                    if !order.contains(&group) {
                        order.push(group);
                    }
                }
            }
        }
        if order.is_empty() {
            return;
        }
        let group_at = |index: usize| snapshot.workspaces.get(index).and_then(group_of);
        let has_loose = children.iter().any(|index| group_at(*index).is_none());
        let mut pending = Vec::new();
        for (position, group) in order.iter().enumerate() {
            let key = collapse_key(repo_key, group);
            let status = snapshot
                .workspaces
                .iter()
                .filter(|workspace| {
                    repo_key_of(workspace) == Some(repo_key) && group_of(workspace) == Some(group)
                })
                .map(|workspace| workspace.agent_status)
                .max_by_key(|status| crate::client::shell::status_priority(*status))
                .unwrap_or(AgentStatus::Unknown);
            let last = position + 1 == order.len() && !has_loose;
            pending.push(Header {
                name: (*group).to_owned(),
                collapsed: collapsed.contains(&key),
                key,
                status,
                last,
            });
            let members = children
                .iter()
                .copied()
                .filter(|index| group_at(*index) == Some(group))
                .collect::<Vec<_>>();
            if let Some(first) = members.first() {
                self.decor
                    .entry(*first)
                    .or_default()
                    .before
                    .append(&mut pending);
            }
            for member in members {
                self.decor.entry(member).or_default().member = Some(!last);
            }
        }
        if pending.is_empty() {
            return;
        }
        match children.iter().find(|index| group_at(**index).is_none()) {
            Some(first_loose) => self
                .decor
                .entry(*first_loose)
                .or_default()
                .before
                .append(&mut pending),
            None => self
                .decor
                .entry(children.last().copied().unwrap_or(parent))
                .or_default()
                .after
                .append(&mut pending),
        }
    }

    /// Header lines drawn with the workspace at `index`, for row heights.
    pub(in crate::client::shell) fn header_rows(&self, index: usize) -> u16 {
        self.decor.get(&index).map_or(0, |decor| {
            u16::try_from(decor.before.len() + decor.after.len()).unwrap_or(u16::MAX)
        })
    }
}

fn parent_repo_key(snapshot: &ClientShellSnapshot, index: usize) -> Option<&str> {
    let workspace = snapshot.workspaces.get(index)?;
    let worktree = workspace.worktree.as_ref()?;
    (!worktree.is_linked_worktree
        && snapshot.workspaces.iter().any(|candidate| {
            candidate.worktree.as_ref().is_some_and(|candidate| {
                candidate.key == worktree.key && candidate.is_linked_worktree
            })
        }))
    .then_some(worktree.key.as_str())
}

/// Which headers of a workspace to draw: those above it or those below it.
#[derive(Debug, Clone, Copy)]
pub(in crate::client::shell) enum Placement {
    Before,
    After,
}

/// What the renderers pass in to draw headers.
pub(in crate::client::shell) struct HeaderCanvas<'a> {
    pub(in crate::client::shell) buffer: &'a mut Buffer,
    /// The row's columns, from `y` down to the bottom of the list.
    pub(in crate::client::shell) area: Rect,
    pub(in crate::client::shell) endpoint_id: &'a ClientEndpointId,
    pub(in crate::client::shell) hits: &'a mut ShellHitMap,
    pub(in crate::client::shell) indicators: crate::config::StatusIndicatorStyle,
    pub(in crate::client::shell) palette: &'a Palette,
}

/// Draw the headers placed with the workspace at `index`, one line each from
/// the top of the canvas, and return how many lines they took.
pub(in crate::client::shell) fn render_headers(
    groups: Option<&SidebarGroups>,
    placement: Placement,
    index: usize,
    canvas: HeaderCanvas<'_>,
) -> u16 {
    let Some(decor) = groups.and_then(|groups| groups.decor.get(&index)) else {
        return 0;
    };
    let headers = match placement {
        Placement::Before => &decor.before,
        Placement::After => &decor.after,
    };
    let HeaderCanvas {
        buffer,
        area,
        endpoint_id,
        hits,
        indicators,
        palette,
    } = canvas;
    let mut drawn = 0;
    for header in headers {
        let y = area.y.saturating_add(drawn);
        if y >= area.bottom() {
            break;
        }
        let row = Rect::new(area.x, y, area.width, 1);
        let mut x = put_segment(
            buffer,
            row.x,
            y,
            row.right(),
            if header.last {
                "   └─ "
            } else {
                "   ├─ "
            },
            Style::default().fg(palette.overlay0),
        );
        x = put_segment(
            buffer,
            x,
            y,
            row.right(),
            if header.collapsed { "▸ " } else { "▾ " },
            Style::default().fg(palette.accent),
        );
        x = put_segment(
            buffer,
            x,
            y,
            row.right().saturating_sub(2),
            &header.name,
            Style::default().fg(palette.subtext0),
        );
        if header.collapsed {
            put_segment(
                buffer,
                x.saturating_add(1),
                y,
                row.right().saturating_sub(2),
                status_icon(header.status, indicators),
                Style::default().fg(status_color(header.status, palette)),
            );
        }
        hits.fork.worktree_group_headers.push(GroupHeaderHit {
            rect: row,
            endpoint_id: endpoint_id.clone(),
            key: header.key.clone(),
        });
        drawn += 1;
    }
    drawn
}

/// The rect to draw a workspace's rows in: a group member moves one level
/// deeper.
pub(in crate::client::shell) fn rows_rect(
    groups: Option<&SidebarGroups>,
    index: usize,
    rect: Rect,
) -> Rect {
    if groups
        .and_then(|groups| groups.decor.get(&index))
        .and_then(|decor| decor.member)
        .is_none()
    {
        return rect;
    }
    Rect::new(
        rect.x.saturating_add(3),
        rect.y,
        rect.width.saturating_sub(3),
        rect.height,
    )
}

/// After a group member's rows are drawn, the repository's rail beside them
/// while more of the repository follows the group. It sits where the
/// member's own row prefix began, so it is drawn last.
pub(in crate::client::shell) fn render_rail(
    groups: Option<&SidebarGroups>,
    buffer: &mut Buffer,
    index: usize,
    rect: Rect,
    palette: &Palette,
) {
    let rail = groups
        .and_then(|groups| groups.decor.get(&index))
        .and_then(|decor| decor.member);
    if rail != Some(true) {
        return;
    }
    for y in rect.y..rect.bottom() {
        put_text(
            buffer,
            rect.x.saturating_add(3),
            y,
            1,
            "│",
            Style::default().fg(palette.overlay0),
        );
    }
}

/// A group header's place on screen, to collapse it on a click.
#[derive(Debug, Clone)]
pub(in crate::client::shell) struct GroupHeaderHit {
    pub(in crate::client::shell) rect: Rect,
    pub(in crate::client::shell) endpoint_id: ClientEndpointId,
    pub(in crate::client::shell) key: String,
}

impl ClientShellState {
    /// Collapse or expand the group whose header is under `point`; false when
    /// no header is there.
    pub(in crate::client::shell) fn toggle_worktree_group_at(
        &mut self,
        point: (u16, u16),
        outcome: &mut ClientShellInput,
    ) -> bool {
        let Some(hit) = self
            .hits
            .fork
            .worktree_group_headers
            .iter()
            .find(|hit| crate::client::shell::contains(hit.rect, point))
            .cloned()
        else {
            return false;
        };
        self.toggle_collapsed_group(&hit.endpoint_id, hit.key);
        outcome.repaint = true;
        self.persist_chrome_preferences(outcome);
        true
    }
}

impl ClientShellState {
    /// Ask for the group of a linked worktree's workspace, starting from its
    /// current one; an empty name takes it out of its group.
    pub(in crate::client::shell) fn open_worktree_group_prompt(&mut self, workspace_id: String) {
        let current = self
            .snapshot
            .as_deref()
            .and_then(|snapshot| {
                snapshot
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.workspace_id == workspace_id)
            })
            .and_then(group_of)
            .unwrap_or_default()
            .to_owned();
        self.overlay = Some(ClientShellOverlay::Rename(ClientRenameOverlay {
            title: "worktree group (empty to remove)",
            input: TextEditor::new(&current, false),
            target: ClientRenameTarget::WorktreeGroup { workspace_id },
        }));
    }
}

/// The method the group prompt sends: join `group`, or leave when empty.
pub(in crate::client::shell) fn set_group_method(
    workspace_id: String,
    group: &str,
) -> crate::api::schema::Method {
    crate::api::schema::Method::WorkspaceSetWorktreeGroup(
        crate::api::schema::WorkspaceSetWorktreeGroupParams {
            workspace_id,
            group: (!group.is_empty()).then(|| group.to_owned()),
        },
    )
}
