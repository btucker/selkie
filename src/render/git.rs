//! Git graph diagram renderer

use std::collections::HashMap;

use crate::diagrams::git::{Commit, CommitType, DiagramOrientation, GitGraphDb};
use crate::error::Result;
use crate::layout::{CharacterSizeEstimator, SizeEstimator};
use crate::render::svg::color::{adjust, darken, invert, lighten, Color};
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement, Theme};

const LAYOUT_OFFSET: f64 = 10.0;
const COMMIT_STEP: f64 = 40.0;
const DEFAULT_POS: f64 = 30.0;
const PX: f64 = 4.0;
const PY: f64 = 2.0;
const THEME_COLOR_LIMIT: usize = 8;

#[derive(Debug, Clone)]
struct GitGraphConfig {
    show_commit_label: bool,
    show_branches: bool,
    rotate_commit_label: bool,
    parallel_commits: bool,
    diagram_padding: f64,
    title_top_margin: f64,
}

#[derive(Debug, Clone)]
struct GitTextContext<'a> {
    dir: DiagramOrientation,
    show_commit_label: bool,
    rotate_commit_label: bool,
    commit_label_font_size: f64,
    tag_label_font_size: f64,
    estimator: &'a CharacterSizeEstimator,
}

impl Default for GitGraphConfig {
    fn default() -> Self {
        Self {
            show_commit_label: true,
            show_branches: true,
            rotate_commit_label: true,
            parallel_commits: false,
            diagram_padding: 8.0,
            title_top_margin: 25.0,
        }
    }
}

#[derive(Debug, Clone)]
struct BranchPosition {
    pos: f64,
    index: usize,
}

#[derive(Debug, Clone, Copy)]
struct CommitPosition {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy)]
struct CommitPositionOffset {
    x: f64,
    y: f64,
    pos_with_offset: f64,
}

#[derive(Debug, Clone)]
struct GitPalette {
    git: Vec<String>,
    git_inv: Vec<String>,
    branch_label: Vec<String>,
    tag_label_color: String,
    tag_label_background: String,
    tag_label_border: String,
    tag_label_font_size: String,
    commit_label_color: String,
    commit_label_background: String,
    commit_label_font_size: String,
    line_color: String,
    text_color: String,
}

#[derive(Debug, Clone)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            min_x: f64::MAX,
            min_y: f64::MAX,
            max_x: f64::MIN,
            max_y: f64::MIN,
        }
    }
}

impl Bounds {
    fn is_valid(&self) -> bool {
        self.min_x.is_finite() && self.min_y.is_finite() && self.max_x.is_finite()
    }

    fn finish(&self) -> (f64, f64, f64, f64) {
        (self.min_x, self.min_y, self.max_x, self.max_y)
    }

    fn include_point(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn include_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.include_point(x, y);
        self.include_point(x + width, y + height);
    }

    fn include_commit_label(
        &mut self,
        commit: &Commit,
        pos: &CommitPosition,
        ctx: &GitTextContext<'_>,
    ) {
        let (bbox_w, bbox_h) = ctx
            .estimator
            .estimate_text_size(&commit.id, ctx.commit_label_font_size);
        let commit_pos_offset = commit_pos_with_offset(pos, ctx.dir);

        let mut rect_x = commit_pos_offset - bbox_w / 2.0 - PY;
        let mut rect_y = pos.y + 13.5;
        let rect_w = bbox_w + 2.0 * PY;
        let rect_h = bbox_h + 2.0 * PY;

        if matches!(
            ctx.dir,
            DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
        ) {
            rect_x = pos.x - (bbox_w + 4.0 * PX + 5.0);
            rect_y = pos.y - 12.0;
        }

        if ctx.rotate_commit_label {
            if matches!(
                ctx.dir,
                DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
            ) {
                self.include_rotated_rect(
                    rect_x, rect_y, rect_w, rect_h, -45.0, pos.x, pos.y, 0.0, 0.0,
                );
            } else {
                let r_x = -7.5 - ((bbox_w + 10.0) / 25.0) * 9.5;
                let r_y = 10.0 + (bbox_w / 25.0) * 8.5;
                self.include_rotated_rect(
                    rect_x,
                    rect_y,
                    rect_w,
                    rect_h,
                    -45.0,
                    commit_pos_offset,
                    pos.y,
                    r_x,
                    r_y,
                );
            }
        } else {
            self.include_rect(rect_x, rect_y, rect_w, rect_h);
        }
    }

    fn include_commit_tags(
        &mut self,
        commit: &Commit,
        pos: &CommitPosition,
        ctx: &GitTextContext<'_>,
    ) {
        let mut max_tag_bbox_width: f64 = 0.0;
        let mut max_tag_bbox_height: f64 = 0.0;
        for tag_value in commit.tags.iter().rev() {
            let (tag_w, tag_h) = ctx
                .estimator
                .estimate_text_size(tag_value, ctx.tag_label_font_size);
            max_tag_bbox_width = max_tag_bbox_width.max(tag_w);
            max_tag_bbox_height = max_tag_bbox_height.max(tag_h);
        }

        let mut y_cursor = 0.0;
        for tag_value in commit.tags.iter().rev() {
            let (tag_w, _tag_h) = ctx
                .estimator
                .estimate_text_size(tag_value, ctx.tag_label_font_size);
            let h2 = max_tag_bbox_height / 2.0;
            let pos_with_offset = commit_pos_with_offset(pos, ctx.dir);
            let ly = pos.y - 19.2 - y_cursor;

            let points = if matches!(
                ctx.dir,
                DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
            ) {
                let y_origin = pos_with_offset + y_cursor;
                let base = vec![
                    crate::layout::Point::new(pos.x, y_origin + 2.0),
                    crate::layout::Point::new(pos.x, y_origin - 2.0),
                    crate::layout::Point::new(pos.x + LAYOUT_OFFSET, y_origin - h2 - 2.0),
                    crate::layout::Point::new(
                        pos.x + LAYOUT_OFFSET + max_tag_bbox_width + 4.0,
                        y_origin - h2 - 2.0,
                    ),
                    crate::layout::Point::new(
                        pos.x + LAYOUT_OFFSET + max_tag_bbox_width + 4.0,
                        y_origin + h2 + 2.0,
                    ),
                    crate::layout::Point::new(pos.x + LAYOUT_OFFSET, y_origin + h2 + 2.0),
                ];
                let transform =
                    Transform::rotate_translate(45.0, pos.x, pos_with_offset, 12.0, 12.0);
                base.into_iter()
                    .map(|p| transform.apply(p))
                    .collect::<Vec<_>>()
            } else {
                vec![
                    crate::layout::Point::new(
                        pos_with_offset - max_tag_bbox_width / 2.0 - PX / 2.0,
                        ly + PY,
                    ),
                    crate::layout::Point::new(
                        pos_with_offset - max_tag_bbox_width / 2.0 - PX / 2.0,
                        ly - PY,
                    ),
                    crate::layout::Point::new(
                        pos_with_offset - max_tag_bbox_width / 2.0 - PX,
                        ly - h2 - PY,
                    ),
                    crate::layout::Point::new(
                        pos_with_offset + max_tag_bbox_width / 2.0 + PX,
                        ly - h2 - PY,
                    ),
                    crate::layout::Point::new(
                        pos_with_offset + max_tag_bbox_width / 2.0 + PX,
                        ly + h2 + PY,
                    ),
                    crate::layout::Point::new(
                        pos_with_offset - max_tag_bbox_width / 2.0 - PX,
                        ly + h2 + PY,
                    ),
                ]
            };

            for point in points {
                self.include_point(point.x, point.y);
            }
            self.include_rect(
                pos_with_offset - tag_w / 2.0,
                pos.y - 16.0 - y_cursor,
                tag_w,
                max_tag_bbox_height,
            );
            y_cursor += 20.0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn include_rotated_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        angle_deg: f64,
        cx: f64,
        cy: f64,
        translate_x: f64,
        translate_y: f64,
    ) {
        let transform = Transform::rotate_translate(angle_deg, cx, cy, translate_x, translate_y);
        let corners = [
            crate::layout::Point::new(x, y),
            crate::layout::Point::new(x + width, y),
            crate::layout::Point::new(x + width, y + height),
            crate::layout::Point::new(x, y + height),
        ];
        for corner in corners {
            let rotated = transform.apply(corner);
            self.include_point(rotated.x, rotated.y);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Transform {
    angle_rad: f64,
    cx: f64,
    cy: f64,
    translate_x: f64,
    translate_y: f64,
}

impl Transform {
    fn rotate_translate(
        angle_deg: f64,
        cx: f64,
        cy: f64,
        translate_x: f64,
        translate_y: f64,
    ) -> Self {
        Self {
            angle_rad: angle_deg.to_radians(),
            cx,
            cy,
            translate_x,
            translate_y,
        }
    }

    fn apply(&self, point: crate::layout::Point) -> crate::layout::Point {
        let mut x = point.x + self.translate_x;
        let mut y = point.y + self.translate_y;
        let cos = self.angle_rad.cos();
        let sin = self.angle_rad.sin();
        let dx = x - self.cx;
        let dy = y - self.cy;
        x = self.cx + dx * cos - dy * sin;
        y = self.cy + dx * sin + dy * cos;
        crate::layout::Point::new(x, y)
    }
}

pub fn render_git(db: &GitGraphDb, config: &RenderConfig) -> Result<String> {
    let git_config = GitGraphConfig::default();
    let mut doc = SvgDocument::new();
    let theme = &config.theme;

    if config.embed_css {
        let mut css = theme.generate_css();
        css.push_str(&generate_git_css(theme));
        if let Some(ref custom_css) = config.theme_css {
            let sanitized = crate::render::svg::sanitize_css(custom_css);
            if !sanitized.is_empty() {
                css.push_str("\n/* Custom CSS */\n");
                css.push_str(&sanitized);
            }
        }
        doc.add_style(&css);
    }

    let branches = db.get_branches_as_obj_array();
    let commits = db.get_commits();
    if branches.is_empty() || commits.is_empty() {
        doc.set_size(400.0, 200.0);
        return Ok(doc.to_string());
    }

    let dir = db.get_direction();
    let estimator = CharacterSizeEstimator {
        char_width_ratio: 0.6,
        line_height_ratio: 1.0,
    };
    let branch_font_size = parse_font_size(&theme.font_size);
    let commit_label_font_size = 10.0;
    let tag_label_font_size = 10.0;
    let text_ctx = GitTextContext {
        dir,
        show_commit_label: git_config.show_commit_label,
        rotate_commit_label: git_config.rotate_commit_label,
        commit_label_font_size,
        tag_label_font_size,
        estimator: &estimator,
    };

    let mut branch_pos: HashMap<String, BranchPosition> = HashMap::new();
    let mut pos = 0.0;
    for (index, branch) in branches.iter().enumerate() {
        let (bbox_w, _bbox_h) = estimator.estimate_text_size(&branch.name, branch_font_size);
        let rotate_offset = if git_config.rotate_commit_label {
            40.0
        } else {
            0.0
        };
        branch_pos.insert(branch.name.clone(), BranchPosition { pos, index });
        if matches!(
            dir,
            DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
        ) {
            pos += 50.0 + rotate_offset + bbox_w / 2.0;
        } else {
            pos += 50.0 + rotate_offset;
        }
    }

    let (commit_pos, max_pos) =
        compute_commit_positions(commits, &branch_pos, dir, git_config.parallel_commits);

    if git_config.show_branches {
        let branches_group = draw_branches(
            &branches,
            &branch_pos,
            dir,
            max_pos,
            branch_font_size,
            git_config.rotate_commit_label,
            &estimator,
        );
        doc.add_element(branches_group);
    }

    let arrows_group = draw_arrows(commits, &commit_pos, &branch_pos, dir);
    doc.add_element(arrows_group);

    let commits_group = draw_commits(
        commits,
        &commit_pos,
        &branch_pos,
        dir,
        git_config.show_commit_label,
        git_config.rotate_commit_label,
        commit_label_font_size,
        tag_label_font_size,
        &estimator,
    );
    doc.add_element(commits_group);

    if !db.diagram_title.is_empty() {
        let title = SvgElement::Text {
            x: (max_pos / 2.0).max(1.0),
            y: git_config.title_top_margin,
            content: db.diagram_title.clone(),
            attrs: Attrs::new().with_class("gitTitleText"),
        };
        doc.add_element(title);
    }

    let (min_x, min_y, width, height) = calculate_bounds(
        &branches,
        &branch_pos,
        &commit_pos,
        commits,
        max_pos,
        branch_font_size,
        &text_ctx,
        git_config.diagram_padding,
    );
    doc.set_size_with_origin(min_x, min_y, width, height);

    Ok(doc.to_string())
}

#[allow(clippy::too_many_arguments)]
fn calculate_bounds(
    branches: &[crate::diagrams::git::BranchConfig],
    branch_pos: &HashMap<String, BranchPosition>,
    commit_pos: &HashMap<String, CommitPosition>,
    commits: &HashMap<String, Commit>,
    max_pos: f64,
    branch_font_size: f64,
    text_ctx: &GitTextContext<'_>,
    padding: f64,
) -> (f64, f64, f64, f64) {
    let mut bounds = Bounds::default();

    for commit in commits.values() {
        if let Some(pos) = commit_pos.get(&commit.id) {
            let radius = if commit.commit_type == CommitType::Merge {
                9.0
            } else {
                10.0
            };
            bounds.include_rect(pos.x - radius, pos.y - radius, radius * 2.0, radius * 2.0);

            if should_draw_commit_label(commit, text_ctx.show_commit_label) {
                bounds.include_commit_label(commit, pos, text_ctx);
            }

            if !commit.tags.is_empty() {
                bounds.include_commit_tags(commit, pos, text_ctx);
            }
        }
    }

    for branch in branches {
        if let Some(pos) = branch_pos.get(&branch.name) {
            let (bbox_w, bbox_h) = text_ctx
                .estimator
                .estimate_text_size(&branch.name, branch_font_size);
            let label_bbox_h = bbox_h + 3.0;
            let label_height = label_bbox_h + 4.0;
            let rotate_offset = if text_ctx.rotate_commit_label {
                30.0
            } else {
                0.0
            };
            let (bkg_x, bkg_y) = match text_ctx.dir {
                DiagramOrientation::LeftToRight => {
                    let label_x = -bbox_w - 14.0 - rotate_offset;
                    let bkg_x = label_x - 5.0;
                    let bkg_y = pos.pos - label_bbox_h / 2.0 - 2.0;
                    (bkg_x, bkg_y)
                }
                DiagramOrientation::TopToBottom => {
                    let bkg_x = pos.pos - bbox_w / 2.0 - 10.0;
                    let bkg_y = 0.0;
                    (bkg_x, bkg_y)
                }
                DiagramOrientation::BottomToTop => {
                    let bkg_x = pos.pos - bbox_w / 2.0 - 10.0;
                    let bkg_y = max_pos;
                    (bkg_x, bkg_y)
                }
            };
            bounds.include_rect(bkg_x, bkg_y, bbox_w + 18.0, label_height);

            match text_ctx.dir {
                DiagramOrientation::LeftToRight => {
                    bounds.include_point(max_pos, pos.pos);
                }
                DiagramOrientation::TopToBottom => {
                    bounds.include_point(pos.pos, max_pos);
                }
                DiagramOrientation::BottomToTop => {
                    bounds.include_point(pos.pos, max_pos);
                }
            }
        }
    }

    if !bounds.is_valid() {
        return (0.0, 0.0, 400.0, 200.0);
    }

    let (min_x, min_y, max_x, max_y) = bounds.finish();
    let width = (max_x - min_x) + padding * 2.0;
    let height = (max_y - min_y) + padding * 2.0;
    (min_x - padding, min_y - padding, width, height)
}

fn compute_commit_positions(
    commits: &HashMap<String, Commit>,
    branch_pos: &HashMap<String, BranchPosition>,
    dir: DiagramOrientation,
    parallel_commits: bool,
) -> (HashMap<String, CommitPosition>, f64) {
    let mut commit_pos: HashMap<String, CommitPosition> = HashMap::new();
    let mut max_pos = 0.0;

    let mut keys: Vec<&String> = commits.keys().collect();
    keys.sort_by_key(|k| commits.get(*k).map(|c| c.seq).unwrap_or(0));

    let mut pos = if matches!(
        dir,
        DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
    ) {
        DEFAULT_POS
    } else {
        0.0
    };

    if dir == DiagramOrientation::BottomToTop && parallel_commits {
        let sorted_keys: Vec<String> = keys.iter().map(|k| (*k).clone()).collect();
        max_pos = set_parallel_bt_pos(&sorted_keys, commits, &mut commit_pos, branch_pos, pos);
    }

    let mut sorted_keys: Vec<String> = keys.iter().map(|k| (*k).clone()).collect();
    if dir == DiagramOrientation::BottomToTop {
        sorted_keys.reverse();
    }

    for key in sorted_keys {
        let commit = match commits.get(&key) {
            Some(commit) => commit,
            None => continue,
        };

        if parallel_commits {
            pos = calculate_position(commit, dir, pos, &commit_pos);
        }

        let commit_position = get_commit_position(commit, pos, parallel_commits, branch_pos, dir);
        if matches!(
            dir,
            DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
        ) {
            commit_pos.insert(
                commit.id.clone(),
                CommitPosition {
                    x: commit_position.x,
                    y: commit_position.pos_with_offset,
                },
            );
        } else {
            commit_pos.insert(
                commit.id.clone(),
                CommitPosition {
                    x: commit_position.pos_with_offset,
                    y: commit_position.y,
                },
            );
        }

        if dir == DiagramOrientation::BottomToTop && parallel_commits {
            pos += COMMIT_STEP;
        } else {
            pos += COMMIT_STEP + LAYOUT_OFFSET;
        }
        if pos > max_pos {
            max_pos = pos;
        }
    }

    (commit_pos, max_pos)
}

fn set_parallel_bt_pos(
    sorted_keys: &[String],
    commits: &HashMap<String, Commit>,
    commit_pos: &mut HashMap<String, CommitPosition>,
    branch_pos: &HashMap<String, BranchPosition>,
    default_pos: f64,
) -> f64 {
    let mut cur_pos = default_pos;
    let mut max_position = default_pos;
    let mut roots: Vec<&Commit> = Vec::new();

    for key in sorted_keys {
        let commit = match commits.get(key) {
            Some(commit) => commit,
            None => continue,
        };

        if !commit.parents.is_empty() {
            cur_pos = calculate_commit_position(commit, commit_pos);
            if cur_pos > max_position {
                max_position = cur_pos;
            }
        } else {
            roots.push(commit);
        }
        set_commit_position(commit, cur_pos, commit_pos, branch_pos);
    }

    cur_pos = max_position;
    for commit in roots {
        set_root_position(commit, cur_pos, default_pos, commit_pos, branch_pos);
    }

    for key in sorted_keys {
        let commit = match commits.get(key) {
            Some(commit) => commit,
            None => continue,
        };
        if !commit.parents.is_empty() {
            let closest_parent = find_closest_parent_bt(&commit.parents, commit_pos);
            if let Some(parent_id) = closest_parent {
                if let Some(parent_pos) = commit_pos.get(&parent_id) {
                    cur_pos = parent_pos.y - COMMIT_STEP;
                    if cur_pos <= max_position {
                        max_position = cur_pos;
                    }
                    if let Some(branch) = branch_pos.get(&commit.branch) {
                        commit_pos.insert(
                            commit.id.clone(),
                            CommitPosition {
                                x: branch.pos,
                                y: cur_pos - LAYOUT_OFFSET,
                            },
                        );
                    }
                }
            }
        }
    }

    max_position
}

fn find_closest_parent_bt(
    parents: &[String],
    commit_pos: &HashMap<String, CommitPosition>,
) -> Option<String> {
    let mut closest_parent = None;
    let mut max_position = f64::INFINITY;

    for parent in parents {
        if let Some(parent_position) = commit_pos.get(parent) {
            if parent_position.y <= max_position {
                closest_parent = Some(parent.clone());
                max_position = parent_position.y;
            }
        }
    }
    closest_parent
}

fn calculate_commit_position(commit: &Commit, commit_pos: &HashMap<String, CommitPosition>) -> f64 {
    let closest_parent_pos = find_closest_parent_pos(commit, commit_pos);
    closest_parent_pos + COMMIT_STEP
}

fn find_closest_parent_pos(commit: &Commit, commit_pos: &HashMap<String, CommitPosition>) -> f64 {
    let closest_parent =
        find_closest_parent(&commit.parents, commit_pos, DiagramOrientation::TopToBottom)
            .unwrap_or_else(|| commit.parents.first().cloned().unwrap_or_default());
    commit_pos
        .get(&closest_parent)
        .map(|pos| pos.y)
        .unwrap_or(DEFAULT_POS)
}

fn find_closest_parent(
    parents: &[String],
    commit_pos: &HashMap<String, CommitPosition>,
    dir: DiagramOrientation,
) -> Option<String> {
    let mut target = if dir == DiagramOrientation::BottomToTop {
        f64::INFINITY
    } else {
        0.0
    };

    let mut closest_parent = None;
    for parent in parents {
        let parent_position = if matches!(
            dir,
            DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
        ) {
            commit_pos.get(parent).map(|p| p.y)
        } else {
            commit_pos.get(parent).map(|p| p.x)
        };
        if let Some(pos) = parent_position {
            let should_replace = if dir == DiagramOrientation::BottomToTop {
                pos <= target
            } else {
                pos >= target
            };
            if should_replace {
                closest_parent = Some(parent.clone());
                target = pos;
            }
        }
    }
    closest_parent
}

fn calculate_position(
    commit: &Commit,
    dir: DiagramOrientation,
    pos: f64,
    commit_pos: &HashMap<String, CommitPosition>,
) -> f64 {
    if !commit.parents.is_empty() {
        if let Some(closest_parent) = find_closest_parent(&commit.parents, commit_pos, dir) {
            if let Some(parent_position) = commit_pos.get(&closest_parent) {
                return match dir {
                    DiagramOrientation::TopToBottom => parent_position.y + COMMIT_STEP,
                    DiagramOrientation::BottomToTop => parent_position.y - COMMIT_STEP,
                    DiagramOrientation::LeftToRight => parent_position.x + COMMIT_STEP,
                };
            }
        }
    } else if dir == DiagramOrientation::TopToBottom {
        return DEFAULT_POS;
    }

    pos
}

fn get_commit_position(
    commit: &Commit,
    pos: f64,
    parallel_commits: bool,
    branch_pos: &HashMap<String, BranchPosition>,
    dir: DiagramOrientation,
) -> CommitPositionOffset {
    let pos_with_offset = if dir == DiagramOrientation::BottomToTop && parallel_commits {
        pos
    } else {
        pos + LAYOUT_OFFSET
    };
    let (x, y) = if matches!(
        dir,
        DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
    ) {
        (
            branch_pos.get(&commit.branch).map(|b| b.pos).unwrap_or(0.0),
            pos_with_offset,
        )
    } else {
        (
            pos_with_offset,
            branch_pos.get(&commit.branch).map(|b| b.pos).unwrap_or(0.0),
        )
    };
    CommitPositionOffset {
        x,
        y,
        pos_with_offset,
    }
}

fn commit_pos_with_offset(pos: &CommitPosition, dir: DiagramOrientation) -> f64 {
    if matches!(
        dir,
        DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
    ) {
        pos.y
    } else {
        pos.x
    }
}

fn set_commit_position(
    commit: &Commit,
    cur_pos: f64,
    commit_pos: &mut HashMap<String, CommitPosition>,
    branch_pos: &HashMap<String, BranchPosition>,
) {
    if let Some(branch) = branch_pos.get(&commit.branch) {
        commit_pos.insert(
            commit.id.clone(),
            CommitPosition {
                x: branch.pos,
                y: cur_pos + LAYOUT_OFFSET,
            },
        );
    }
}

fn set_root_position(
    commit: &Commit,
    cur_pos: f64,
    default_pos: f64,
    commit_pos: &mut HashMap<String, CommitPosition>,
    branch_pos: &HashMap<String, BranchPosition>,
) {
    if let Some(branch) = branch_pos.get(&commit.branch) {
        commit_pos.insert(
            commit.id.clone(),
            CommitPosition {
                x: branch.pos,
                y: cur_pos + default_pos,
            },
        );
    }
}

fn draw_branches(
    branches: &[crate::diagrams::git::BranchConfig],
    branch_pos: &HashMap<String, BranchPosition>,
    dir: DiagramOrientation,
    max_pos: f64,
    branch_font_size: f64,
    rotate_commit_label: bool,
    estimator: &CharacterSizeEstimator,
) -> SvgElement {
    let mut elements = Vec::new();
    for branch in branches {
        let Some(position) = branch_pos.get(&branch.name) else {
            continue;
        };
        let adjust_index = position.index % THEME_COLOR_LIMIT;
        let mut line = SvgElement::Line {
            x1: 0.0,
            y1: position.pos,
            x2: max_pos,
            y2: position.pos,
            attrs: Attrs::new().with_class(&format!("branch branch{}", adjust_index)),
        };
        if dir == DiagramOrientation::TopToBottom {
            line = SvgElement::Line {
                x1: position.pos,
                y1: DEFAULT_POS,
                x2: position.pos,
                y2: max_pos,
                attrs: Attrs::new().with_class(&format!("branch branch{}", adjust_index)),
            };
        } else if dir == DiagramOrientation::BottomToTop {
            line = SvgElement::Line {
                x1: position.pos,
                y1: max_pos,
                x2: position.pos,
                y2: DEFAULT_POS,
                attrs: Attrs::new().with_class(&format!("branch branch{}", adjust_index)),
            };
        }
        elements.push(line);

        let (bbox_w, bbox_h) = estimator.estimate_text_size(&branch.name, branch_font_size);
        let label_bbox_h = bbox_h + 3.0;
        let label_height = label_bbox_h + 4.0;
        let rotate_offset = if rotate_commit_label { 30.0 } else { 0.0 };
        let (bkg_x, bkg_y, label_w) = match dir {
            DiagramOrientation::LeftToRight => {
                let label_x = -bbox_w - 14.0 - rotate_offset;
                let bkg_x = label_x - 5.0;
                let bkg_y = position.pos - label_bbox_h / 2.0 - 2.0;
                (bkg_x, bkg_y, bbox_w + 18.0)
            }
            DiagramOrientation::TopToBottom => {
                let bkg_x = position.pos - bbox_w / 2.0 - 10.0;
                let bkg_y = 0.0;
                (bkg_x, bkg_y, bbox_w + 18.0)
            }
            DiagramOrientation::BottomToTop => {
                let bkg_x = position.pos - bbox_w / 2.0 - 10.0;
                let bkg_y = max_pos;
                (bkg_x, bkg_y, bbox_w + 18.0)
            }
        };

        let rect = SvgElement::Rect {
            x: bkg_x,
            y: bkg_y,
            width: label_w,
            height: label_height,
            rx: Some(4.0),
            ry: Some(4.0),
            attrs: Attrs::new().with_class(&format!("branchLabelBkg label{}", adjust_index)),
        };
        let label_center_x = bkg_x + label_w / 2.0;
        let label_center_y = bkg_y + label_height / 2.0;
        let text = SvgElement::Text {
            x: label_center_x,
            y: label_center_y,
            content: branch.name.clone(),
            attrs: Attrs::new()
                .with_class(&format!("branch-label{}", adjust_index))
                .with_attr("text-anchor", "middle")
                .with_attr("dominant-baseline", "middle"),
        };
        elements.push(rect);
        elements.push(text);
    }

    SvgElement::group(elements).with_attrs(Attrs::new().with_class("branches"))
}

#[allow(clippy::too_many_arguments)]
fn draw_commits(
    commits: &HashMap<String, Commit>,
    commit_pos: &HashMap<String, CommitPosition>,
    branch_pos: &HashMap<String, BranchPosition>,
    dir: DiagramOrientation,
    show_commit_label: bool,
    rotate_commit_label: bool,
    commit_label_font_size: f64,
    tag_label_font_size: f64,
    estimator: &CharacterSizeEstimator,
) -> SvgElement {
    let mut bullets = Vec::new();
    let mut labels = Vec::new();

    let mut keys: Vec<&String> = commits.keys().collect();
    keys.sort_by_key(|k| commits.get(*k).map(|c| c.seq).unwrap_or(0));

    for key in keys {
        let Some(commit) = commits.get(key) else {
            continue;
        };
        let commit_position = match commit_pos.get(&commit.id) {
            Some(pos) => CommitPositionOffset {
                x: pos.x,
                y: pos.y,
                pos_with_offset: if matches!(
                    dir,
                    DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
                ) {
                    pos.y
                } else {
                    pos.x
                },
            },
            None => continue,
        };

        let type_class = get_commit_class_type(commit);
        let commit_symbol_type = commit.custom_type.unwrap_or(commit.commit_type);
        let branch_index = branch_pos.get(&commit.branch).map(|b| b.index).unwrap_or(0);

        bullets.extend(draw_commit_bullet(
            commit,
            commit_position,
            &type_class,
            branch_index,
            commit_symbol_type,
        ));

        if should_draw_commit_label(commit, show_commit_label) {
            labels.extend(draw_commit_label(
                commit,
                commit_position,
                commit_position.pos_with_offset,
                dir,
                rotate_commit_label,
                commit_label_font_size,
                estimator,
            ));
        }
        labels.extend(draw_commit_tags(
            commit,
            commit_position,
            commit_position.pos_with_offset,
            dir,
            tag_label_font_size,
            estimator,
        ));
    }

    let bullets_group =
        SvgElement::group(bullets).with_attrs(Attrs::new().with_class("commit-bullets"));
    let labels_group =
        SvgElement::group(labels).with_attrs(Attrs::new().with_class("commit-labels"));
    SvgElement::group(vec![bullets_group, labels_group])
        .with_attrs(Attrs::new().with_class("commits"))
}

fn should_draw_commit_label(commit: &Commit, show_commit_label: bool) -> bool {
    if !show_commit_label {
        return false;
    }
    if commit.commit_type == CommitType::CherryPick {
        return false;
    }
    if commit.commit_type == CommitType::Merge {
        return commit.custom_id;
    }
    true
}

fn draw_commit_bullet(
    commit: &Commit,
    commit_position: CommitPositionOffset,
    type_class: &str,
    branch_index: usize,
    commit_symbol_type: CommitType,
) -> Vec<SvgElement> {
    let mut elements = Vec::new();
    let commit_class = format!(
        "commit {} commit{}",
        sanitize_class(&commit.id),
        branch_index % THEME_COLOR_LIMIT
    );

    match commit_symbol_type {
        CommitType::Highlight => {
            let outer = SvgElement::Rect {
                x: commit_position.x - 10.0,
                y: commit_position.y - 10.0,
                width: 20.0,
                height: 20.0,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_class(&format!(
                    "commit {} commit-highlight{} {}-outer",
                    sanitize_class(&commit.id),
                    branch_index % THEME_COLOR_LIMIT,
                    type_class
                )),
            };
            let inner = SvgElement::Rect {
                x: commit_position.x - 6.0,
                y: commit_position.y - 6.0,
                width: 12.0,
                height: 12.0,
                rx: None,
                ry: None,
                attrs: Attrs::new().with_class(&format!(
                    "commit {} commit{} {}-inner",
                    sanitize_class(&commit.id),
                    branch_index % THEME_COLOR_LIMIT,
                    type_class
                )),
            };
            elements.push(outer);
            elements.push(inner);
        }
        CommitType::CherryPick => {
            let outer = SvgElement::Circle {
                cx: commit_position.x,
                cy: commit_position.y,
                r: 10.0,
                attrs: Attrs::new().with_class(&format!(
                    "commit {} {}",
                    sanitize_class(&commit.id),
                    type_class
                )),
            };
            let left_eye = SvgElement::Circle {
                cx: commit_position.x - 3.0,
                cy: commit_position.y + 2.0,
                r: 2.75,
                attrs: Attrs::new()
                    .with_class(&format!(
                        "commit {} {}",
                        sanitize_class(&commit.id),
                        type_class
                    ))
                    .with_fill("#fff"),
            };
            let right_eye = SvgElement::Circle {
                cx: commit_position.x + 3.0,
                cy: commit_position.y + 2.0,
                r: 2.75,
                attrs: Attrs::new()
                    .with_class(&format!(
                        "commit {} {}",
                        sanitize_class(&commit.id),
                        type_class
                    ))
                    .with_fill("#fff"),
            };
            let left_line = SvgElement::Line {
                x1: commit_position.x + 3.0,
                y1: commit_position.y + 1.0,
                x2: commit_position.x,
                y2: commit_position.y - 5.0,
                attrs: Attrs::new()
                    .with_class(&format!(
                        "commit {} {}",
                        sanitize_class(&commit.id),
                        type_class
                    ))
                    .with_stroke("#fff"),
            };
            let right_line = SvgElement::Line {
                x1: commit_position.x - 3.0,
                y1: commit_position.y + 1.0,
                x2: commit_position.x,
                y2: commit_position.y - 5.0,
                attrs: Attrs::new()
                    .with_class(&format!(
                        "commit {} {}",
                        sanitize_class(&commit.id),
                        type_class
                    ))
                    .with_stroke("#fff"),
            };
            elements.extend([outer, left_eye, right_eye, left_line, right_line]);
        }
        _ => {
            let circle = SvgElement::Circle {
                cx: commit_position.x,
                cy: commit_position.y,
                r: if commit_symbol_type == CommitType::Merge {
                    9.0
                } else {
                    10.0
                },
                attrs: Attrs::new().with_class(&commit_class),
            };
            elements.push(circle);

            if commit_symbol_type == CommitType::Merge {
                let inner = SvgElement::Circle {
                    cx: commit_position.x,
                    cy: commit_position.y,
                    r: 6.0,
                    attrs: Attrs::new().with_class(&format!(
                        "commit {} {} commit{}",
                        type_class,
                        sanitize_class(&commit.id),
                        branch_index % THEME_COLOR_LIMIT
                    )),
                };
                elements.push(inner);
            }

            if commit_symbol_type == CommitType::Reverse {
                let cross = SvgElement::Path {
                    d: format!(
                        "M {},{}L {},{}M {},{}L {},{}",
                        commit_position.x - 5.0,
                        commit_position.y - 5.0,
                        commit_position.x + 5.0,
                        commit_position.y + 5.0,
                        commit_position.x - 5.0,
                        commit_position.y + 5.0,
                        commit_position.x + 5.0,
                        commit_position.y - 5.0
                    ),
                    attrs: Attrs::new().with_class(&format!(
                        "commit {} {} commit{}",
                        type_class,
                        sanitize_class(&commit.id),
                        branch_index % THEME_COLOR_LIMIT
                    )),
                };
                elements.push(cross);
            }
        }
    }

    elements
}

fn draw_commit_label(
    commit: &Commit,
    commit_position: CommitPositionOffset,
    pos: f64,
    dir: DiagramOrientation,
    rotate_commit_label: bool,
    font_size: f64,
    estimator: &CharacterSizeEstimator,
) -> Vec<SvgElement> {
    let mut elements = Vec::new();
    let (bbox_w, bbox_h) = estimator.estimate_text_size(&commit.id, font_size);

    let mut rect_x = commit_position.pos_with_offset - bbox_w / 2.0 - PY;
    let mut rect_y = commit_position.y + 13.5;
    let rect_w = bbox_w + 2.0 * PY;
    let rect_h = bbox_h + 2.0 * PY;
    let mut text_x = commit_position.pos_with_offset - bbox_w / 2.0;
    let mut text_y = commit_position.y + 25.0;

    if matches!(
        dir,
        DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
    ) {
        rect_x = commit_position.x - (bbox_w + 4.0 * PX + 5.0);
        rect_y = commit_position.y - 12.0;
        text_x = commit_position.x - (bbox_w + 4.0 * PX);
        text_y = commit_position.y + bbox_h - 12.0;
    }

    let mut rect_attrs = Attrs::new().with_class("commit-label-bkg");
    let mut text_attrs = Attrs::new().with_class("commit-label");

    if rotate_commit_label {
        if matches!(
            dir,
            DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
        ) {
            let transform = format!("rotate(-45, {}, {})", commit_position.x, commit_position.y);
            rect_attrs = rect_attrs.with_transform(&transform);
            text_attrs = text_attrs.with_transform(&transform);
        } else {
            let r_x = -7.5 - ((bbox_w + 10.0) / 25.0) * 9.5;
            let r_y = 10.0 + (bbox_w / 25.0) * 8.5;
            let transform = format!(
                "translate({}, {}) rotate(-45, {}, {})",
                r_x, r_y, pos, commit_position.y
            );
            rect_attrs = rect_attrs.with_transform(&transform);
            text_attrs = text_attrs.with_transform(&transform);
        }
    }

    let rect = SvgElement::Rect {
        x: rect_x,
        y: rect_y,
        width: rect_w,
        height: rect_h,
        rx: None,
        ry: None,
        attrs: rect_attrs,
    };
    let text = SvgElement::Text {
        x: text_x,
        y: text_y,
        content: commit.id.clone(),
        attrs: text_attrs,
    };
    elements.push(rect);
    elements.push(text);
    elements
}

fn draw_commit_tags(
    commit: &Commit,
    commit_position: CommitPositionOffset,
    pos: f64,
    dir: DiagramOrientation,
    font_size: f64,
    estimator: &CharacterSizeEstimator,
) -> Vec<SvgElement> {
    if commit.tags.is_empty() {
        return Vec::new();
    }

    let mut elements = Vec::new();
    let mut y_offset = 0.0;
    let mut max_tag_bbox_width: f64 = 0.0;
    let mut max_tag_bbox_height: f64 = 0.0;

    let mut tag_layouts = Vec::new();

    for tag_value in commit.tags.iter().rev() {
        let (tag_w, tag_h) = estimator.estimate_text_size(tag_value, font_size);
        max_tag_bbox_width = max_tag_bbox_width.max(tag_w);
        max_tag_bbox_height = max_tag_bbox_height.max(tag_h);

        tag_layouts.push((tag_value.clone(), tag_w, tag_h, y_offset));
        y_offset += 20.0;
    }

    for (tag_value, tag_w, _tag_h, y_offset) in tag_layouts {
        let h2 = max_tag_bbox_height / 2.0;
        let ly = commit_position.y - 19.2 - y_offset;
        let mut points = vec![
            crate::layout::Point::new(pos - max_tag_bbox_width / 2.0 - PX / 2.0, ly + PY),
            crate::layout::Point::new(pos - max_tag_bbox_width / 2.0 - PX / 2.0, ly - PY),
            crate::layout::Point::new(
                commit_position.pos_with_offset - max_tag_bbox_width / 2.0 - PX,
                ly - h2 - PY,
            ),
            crate::layout::Point::new(
                commit_position.pos_with_offset + max_tag_bbox_width / 2.0 + PX,
                ly - h2 - PY,
            ),
            crate::layout::Point::new(
                commit_position.pos_with_offset + max_tag_bbox_width / 2.0 + PX,
                ly + h2 + PY,
            ),
            crate::layout::Point::new(
                commit_position.pos_with_offset - max_tag_bbox_width / 2.0 - PX,
                ly + h2 + PY,
            ),
        ];

        let mut polygon_attrs = Attrs::new().with_class("tag-label-bkg");
        let mut hole_attrs = Attrs::new().with_class("tag-hole");
        let mut text_attrs = Attrs::new().with_class("tag-label");
        let mut text_x = commit_position.pos_with_offset - tag_w / 2.0;
        let mut text_y = commit_position.y - 16.0 - y_offset;
        let mut hole_x = pos - max_tag_bbox_width / 2.0 + PX / 2.0;
        let mut hole_y = ly;

        if matches!(
            dir,
            DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
        ) {
            let y_origin = pos + y_offset;
            points = vec![
                crate::layout::Point::new(commit_position.x, y_origin + 2.0),
                crate::layout::Point::new(commit_position.x, y_origin - 2.0),
                crate::layout::Point::new(commit_position.x + LAYOUT_OFFSET, y_origin - h2 - 2.0),
                crate::layout::Point::new(
                    commit_position.x + LAYOUT_OFFSET + max_tag_bbox_width + 4.0,
                    y_origin - h2 - 2.0,
                ),
                crate::layout::Point::new(
                    commit_position.x + LAYOUT_OFFSET + max_tag_bbox_width + 4.0,
                    y_origin + h2 + 2.0,
                ),
                crate::layout::Point::new(commit_position.x + LAYOUT_OFFSET, y_origin + h2 + 2.0),
            ];
            let transform = format!(
                "translate(12,12) rotate(45, {}, {})",
                commit_position.x, pos
            );
            polygon_attrs = polygon_attrs.with_transform(&transform);
            hole_attrs = hole_attrs.with_transform(&transform);
            text_attrs = text_attrs.with_transform(&transform);
            hole_x = commit_position.x + PX / 2.0;
            hole_y = y_origin;
            text_x = commit_position.x + 5.0;
            text_y = y_origin + 3.0;
        }

        let polygon = SvgElement::Polygon {
            points,
            attrs: polygon_attrs,
        };
        let hole = SvgElement::Circle {
            cx: hole_x,
            cy: hole_y,
            r: 1.5,
            attrs: hole_attrs,
        };
        let text = SvgElement::Text {
            x: text_x,
            y: text_y,
            content: tag_value,
            attrs: text_attrs,
        };
        elements.extend([polygon, hole, text]);
    }

    elements
}

fn get_commit_class_type(commit: &Commit) -> String {
    let commit_symbol_type = commit.custom_type.unwrap_or(commit.commit_type);
    match commit_symbol_type {
        CommitType::Normal => "commit-normal",
        CommitType::Reverse => "commit-reverse",
        CommitType::Highlight => "commit-highlight",
        CommitType::Merge => "commit-merge",
        CommitType::CherryPick => "commit-cherry-pick",
    }
    .to_string()
}

fn draw_arrows(
    commits: &HashMap<String, Commit>,
    commit_pos: &HashMap<String, CommitPosition>,
    branch_pos: &HashMap<String, BranchPosition>,
    dir: DiagramOrientation,
) -> SvgElement {
    let mut elements = Vec::new();
    let mut lanes = Vec::new();

    for commit in commits.values() {
        for parent in &commit.parents {
            if let Some(parent_commit) = commits.get(parent) {
                if let Some(path) = draw_arrow(
                    parent_commit,
                    commit,
                    commits,
                    commit_pos,
                    branch_pos,
                    dir,
                    &mut lanes,
                ) {
                    elements.push(path);
                }
            }
        }
    }

    SvgElement::group(elements).with_attrs(Attrs::new().with_class("commit-arrows"))
}

fn draw_arrow(
    commit_a: &Commit,
    commit_b: &Commit,
    all_commits: &HashMap<String, Commit>,
    commit_pos: &HashMap<String, CommitPosition>,
    branch_pos: &HashMap<String, BranchPosition>,
    dir: DiagramOrientation,
    lanes: &mut Vec<f64>,
) -> Option<SvgElement> {
    let p1 = commit_pos.get(&commit_a.id)?;
    let p2 = commit_pos.get(&commit_b.id)?;
    let arrow_needs_rerouting = should_reroute_arrow(commit_a, commit_b, p1, p2, all_commits, dir);

    let mut color_class_num = branch_pos
        .get(&commit_b.branch)
        .map(|b| b.index)
        .unwrap_or(0);
    if commit_b.commit_type == CommitType::Merge && commit_a.id != commit_b.parents[0] {
        color_class_num = branch_pos
            .get(&commit_a.branch)
            .map(|b| b.index)
            .unwrap_or(0);
    }

    let line_def = if arrow_needs_rerouting {
        let arc = "A 10 10, 0, 0, 0,";
        let arc2 = "A 10 10, 0, 0, 1,";
        let radius = 10.0;
        let offset = 10.0;

        let line_y = if p1.y < p2.y {
            find_lane(p1.y, p2.y, lanes, 0)
        } else {
            find_lane(p2.y, p1.y, lanes, 0)
        };
        let line_x = if p1.x < p2.x {
            find_lane(p1.x, p2.x, lanes, 0)
        } else {
            find_lane(p2.x, p1.x, lanes, 0)
        };

        match dir {
            DiagramOrientation::TopToBottom => {
                if p1.x < p2.x {
                    format!(
                        "M {} {} L {} {} {} {} {} L {} {} {} {} {} L {} {}",
                        p1.x,
                        p1.y,
                        line_x - radius,
                        p1.y,
                        arc2,
                        line_x,
                        p1.y + offset,
                        line_x,
                        p2.y - radius,
                        arc,
                        line_x + offset,
                        p2.y,
                        p2.x,
                        p2.y
                    )
                } else {
                    color_class_num = branch_pos
                        .get(&commit_a.branch)
                        .map(|b| b.index)
                        .unwrap_or(0);
                    format!(
                        "M {} {} L {} {} {} {} {} L {} {} {} {} {} L {} {}",
                        p1.x,
                        p1.y,
                        line_x + radius,
                        p1.y,
                        arc,
                        line_x,
                        p1.y + offset,
                        line_x,
                        p2.y - radius,
                        arc2,
                        line_x - offset,
                        p2.y,
                        p2.x,
                        p2.y
                    )
                }
            }
            DiagramOrientation::BottomToTop => {
                if p1.x < p2.x {
                    format!(
                        "M {} {} L {} {} {} {} {} L {} {} {} {} {} L {} {}",
                        p1.x,
                        p1.y,
                        line_x - radius,
                        p1.y,
                        arc,
                        line_x,
                        p1.y - offset,
                        line_x,
                        p2.y + radius,
                        arc2,
                        line_x + offset,
                        p2.y,
                        p2.x,
                        p2.y
                    )
                } else {
                    color_class_num = branch_pos
                        .get(&commit_a.branch)
                        .map(|b| b.index)
                        .unwrap_or(0);
                    format!(
                        "M {} {} L {} {} {} {} {} L {} {} {} {} {} L {} {}",
                        p1.x,
                        p1.y,
                        line_x + radius,
                        p1.y,
                        arc2,
                        line_x,
                        p1.y - offset,
                        line_x,
                        p2.y + radius,
                        arc,
                        line_x - offset,
                        p2.y,
                        p2.x,
                        p2.y
                    )
                }
            }
            DiagramOrientation::LeftToRight => {
                if p1.y < p2.y {
                    format!(
                        "M {} {} L {} {} {} {} {} L {} {} {} {} {} L {} {}",
                        p1.x,
                        p1.y,
                        p1.x,
                        line_y - radius,
                        arc,
                        p1.x + offset,
                        line_y,
                        p2.x - radius,
                        line_y,
                        arc2,
                        p2.x,
                        line_y + offset,
                        p2.x,
                        p2.y
                    )
                } else {
                    color_class_num = branch_pos
                        .get(&commit_a.branch)
                        .map(|b| b.index)
                        .unwrap_or(0);
                    format!(
                        "M {} {} L {} {} {} {} {} L {} {} {} {} {} L {} {}",
                        p1.x,
                        p1.y,
                        p1.x,
                        line_y + radius,
                        arc2,
                        p1.x + offset,
                        line_y,
                        p2.x - radius,
                        line_y,
                        arc,
                        p2.x,
                        line_y - offset,
                        p2.x,
                        p2.y
                    )
                }
            }
        }
    } else {
        let arc = "A 20 20, 0, 0, 0,";
        let arc2 = "A 20 20, 0, 0, 1,";
        let radius = 20.0;
        let offset = 20.0;

        match dir {
            DiagramOrientation::TopToBottom => {
                if p1.x < p2.x {
                    if commit_b.commit_type == CommitType::Merge
                        && commit_a.id != commit_b.parents[0]
                    {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p1.x,
                            p2.y - radius,
                            arc,
                            p1.x + offset,
                            p2.y,
                            p2.x,
                            p2.y
                        )
                    } else {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p2.x - radius,
                            p1.y,
                            arc2,
                            p2.x,
                            p1.y + offset,
                            p2.x,
                            p2.y
                        )
                    }
                } else if p1.x > p2.x {
                    if commit_b.commit_type == CommitType::Merge
                        && commit_a.id != commit_b.parents[0]
                    {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p1.x,
                            p2.y - radius,
                            arc2,
                            p1.x - offset,
                            p2.y,
                            p2.x,
                            p2.y
                        )
                    } else {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p2.x + radius,
                            p1.y,
                            arc,
                            p2.x,
                            p1.y + offset,
                            p2.x,
                            p2.y
                        )
                    }
                } else {
                    format!("M {} {} L {} {}", p1.x, p1.y, p2.x, p2.y)
                }
            }
            DiagramOrientation::BottomToTop => {
                if p1.x < p2.x {
                    if commit_b.commit_type == CommitType::Merge
                        && commit_a.id != commit_b.parents[0]
                    {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p1.x,
                            p2.y + radius,
                            arc2,
                            p1.x + offset,
                            p2.y,
                            p2.x,
                            p2.y
                        )
                    } else {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p2.x - radius,
                            p1.y,
                            arc,
                            p2.x,
                            p1.y - offset,
                            p2.x,
                            p2.y
                        )
                    }
                } else if p1.x > p2.x {
                    if commit_b.commit_type == CommitType::Merge
                        && commit_a.id != commit_b.parents[0]
                    {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p1.x,
                            p2.y + radius,
                            arc,
                            p1.x - offset,
                            p2.y,
                            p2.x,
                            p2.y
                        )
                    } else {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p2.x - radius,
                            p1.y,
                            arc,
                            p2.x,
                            p1.y - offset,
                            p2.x,
                            p2.y
                        )
                    }
                } else {
                    format!("M {} {} L {} {}", p1.x, p1.y, p2.x, p2.y)
                }
            }
            DiagramOrientation::LeftToRight => {
                if p1.y < p2.y {
                    if commit_b.commit_type == CommitType::Merge
                        && commit_a.id != commit_b.parents[0]
                    {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p2.x - radius,
                            p1.y,
                            arc2,
                            p2.x,
                            p1.y + offset,
                            p2.x,
                            p2.y
                        )
                    } else {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p1.x,
                            p2.y - radius,
                            arc,
                            p1.x + offset,
                            p2.y,
                            p2.x,
                            p2.y
                        )
                    }
                } else if p1.y > p2.y {
                    if commit_b.commit_type == CommitType::Merge
                        && commit_a.id != commit_b.parents[0]
                    {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p2.x - radius,
                            p1.y,
                            arc,
                            p2.x,
                            p1.y - offset,
                            p2.x,
                            p2.y
                        )
                    } else {
                        format!(
                            "M {} {} L {} {} {} {} {} L {} {}",
                            p1.x,
                            p1.y,
                            p1.x,
                            p2.y + radius,
                            arc2,
                            p1.x + offset,
                            p2.y,
                            p2.x,
                            p2.y
                        )
                    }
                } else {
                    format!("M {} {} L {} {}", p1.x, p1.y, p2.x, p2.y)
                }
            }
        }
    };

    Some(SvgElement::Path {
        d: line_def,
        attrs: Attrs::new().with_class(&format!(
            "arrow arrow{}",
            color_class_num % THEME_COLOR_LIMIT
        )),
    })
}

fn should_reroute_arrow(
    commit_a: &Commit,
    commit_b: &Commit,
    p1: &CommitPosition,
    p2: &CommitPosition,
    all_commits: &HashMap<String, Commit>,
    dir: DiagramOrientation,
) -> bool {
    let commit_b_is_furthest = if matches!(
        dir,
        DiagramOrientation::TopToBottom | DiagramOrientation::BottomToTop
    ) {
        p1.x < p2.x
    } else {
        p1.y < p2.y
    };
    let branch_to_get_curve = if commit_b_is_furthest {
        &commit_b.branch
    } else {
        &commit_a.branch
    };
    all_commits.values().any(|commit_x| {
        commit_x.seq > commit_a.seq
            && commit_x.seq < commit_b.seq
            && commit_x.branch == *branch_to_get_curve
    })
}

fn find_lane(y1: f64, y2: f64, lanes: &mut Vec<f64>, depth: usize) -> f64 {
    let candidate = y1 + (y1 - y2).abs() / 2.0;
    if depth > 5 {
        return candidate;
    }
    let ok = lanes.iter().all(|lane| (*lane - candidate).abs() >= 10.0);
    if ok {
        lanes.push(candidate);
        return candidate;
    }
    let diff = (y1 - y2).abs();
    find_lane(y1, y2 - diff / 5.0, lanes, depth + 1)
}

fn parse_font_size(size: &str) -> f64 {
    let trimmed = size.trim().trim_end_matches("px");
    trimmed.parse().unwrap_or(16.0)
}

fn sanitize_class(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn generate_git_css(theme: &Theme) -> String {
    let palette = compute_git_palette(theme);
    let mut css = String::new();
    css.push_str(
        r#"
  .commit-id,
  .commit-msg,
  .branch-label {
    fill: lightgrey;
    color: lightgrey;
    font-family: var(--mermaid-font-family);
  }
"#,
    );

    for i in 0..THEME_COLOR_LIMIT {
        let git = palette
            .git
            .get(i)
            .cloned()
            .unwrap_or_else(|| theme.primary_color.clone());
        let git_inv = palette
            .git_inv
            .get(i)
            .cloned()
            .unwrap_or_else(|| theme.primary_border_color.clone());
        let branch_label = palette
            .branch_label
            .get(i)
            .cloned()
            .unwrap_or_else(|| palette.text_color.clone());
        css.push_str(&format!(
            r#"
        .branch-label{idx} {{ fill: {branch_label}; }}
        .commit{idx} {{ stroke: {git}; fill: {git}; }}
        .commit-highlight{idx} {{ stroke: {git_inv}; fill: {git_inv}; }}
        .label{idx}  {{ fill: {git}; }}
        .arrow{idx} {{ stroke: {git}; }}
"#,
            idx = i,
            branch_label = branch_label,
            git = git,
            git_inv = git_inv,
        ));
    }

    css.push_str(&format!(
        r#"
  .branch {{
    stroke-width: 1;
    stroke: {line_color};
    stroke-dasharray: 2;
  }}
  .commit-label {{ font-size: {commit_label_font_size}; fill: {commit_label_color}; }}
  .commit-label-bkg {{ font-size: {commit_label_font_size}; fill: {commit_label_background}; opacity: 0.5; }}
  .tag-label {{ font-size: {tag_label_font_size}; fill: {tag_label_color}; }}
  .tag-label-bkg {{ fill: {tag_label_background}; stroke: {tag_label_border}; }}
  .tag-hole {{ fill: {text_color}; }}
  .commit-merge {{
    stroke: {primary_color};
    fill: {primary_color};
  }}
  .commit-reverse {{
    stroke: {primary_color};
    fill: {primary_color};
    stroke-width: 3;
  }}
  .commit-highlight-inner {{
    stroke: {primary_color};
    fill: {primary_color};
  }}
  .arrow {{ stroke-width: 8; stroke-linecap: round; fill: none }}
  .gitTitleText {{
    text-anchor: middle;
    font-size: 18px;
    fill: {text_color};
  }}
"#,
        line_color = palette.line_color,
        commit_label_font_size = palette.commit_label_font_size,
        commit_label_color = palette.commit_label_color,
        commit_label_background = palette.commit_label_background,
        tag_label_font_size = palette.tag_label_font_size,
        tag_label_color = palette.tag_label_color,
        tag_label_background = palette.tag_label_background,
        tag_label_border = palette.tag_label_border,
        text_color = palette.text_color,
        primary_color = theme.primary_color,
    ));

    css
}

fn compute_git_palette(theme: &Theme) -> GitPalette {
    let primary = Color::parse(&theme.primary_color).unwrap_or(Color::rgb(236, 236, 255));
    let secondary = Color::parse(&theme.secondary_color).unwrap_or(Color::rgb(255, 255, 222));
    let tertiary = if theme.tertiary_color.trim().eq_ignore_ascii_case("#fafafa") {
        adjust(&primary, -160.0, 0.0, 0.0)
    } else {
        Color::parse(&theme.tertiary_color).unwrap_or_else(|| adjust(&primary, -160.0, 0.0, 0.0))
    };
    let dark_mode = Color::parse(&theme.background)
        .map(|c| c.is_dark())
        .unwrap_or(false);

    let mut git_colors = vec![
        primary.clone(),
        secondary.clone(),
        tertiary.clone(),
        adjust(&primary, -30.0, 0.0, 0.0),
        adjust(&primary, -60.0, 0.0, 0.0),
        adjust(&primary, -90.0, 0.0, 0.0),
        adjust(&primary, 60.0, 0.0, 0.0),
        adjust(&primary, 120.0, 0.0, 0.0),
    ];

    for color in &mut git_colors {
        if dark_mode {
            *color = lighten(color, 50.0);
        } else {
            *color = darken(color, 50.0);
        }
    }

    let git_inv = git_colors
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            if idx == 0 {
                darken(&invert(c), 50.0).to_hex()
            } else {
                invert(c).to_hex()
            }
        })
        .collect::<Vec<_>>();

    let label_text = crate::render::svg::color::contrasting_text(&primary);
    let label_inv = invert(&label_text);
    let branch_label = vec![
        label_inv.to_hex(),
        label_text.to_hex(),
        label_text.to_hex(),
        label_inv.to_hex(),
        label_text.to_hex(),
        label_text.to_hex(),
        label_text.to_hex(),
        label_text.to_hex(),
    ];

    let secondary_text = invert(&secondary);
    let tag_label_color = invert(&primary);
    let tag_border = crate::render::svg::color::mk_border(&primary, dark_mode);

    GitPalette {
        git: git_colors.iter().map(|c| c.to_hex()).collect(),
        git_inv,
        branch_label,
        tag_label_color: tag_label_color.to_hex(),
        tag_label_background: theme.primary_color.clone(),
        tag_label_border: tag_border.to_hex(),
        tag_label_font_size: "10px".to_string(),
        commit_label_color: secondary_text.to_hex(),
        commit_label_background: theme.secondary_color.clone(),
        commit_label_font_size: "10px".to_string(),
        line_color: theme.line_color.clone(),
        text_color: theme.primary_text_color.clone(),
    }
}
