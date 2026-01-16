//! Git graph diagram types

use std::collections::HashMap;

/// Commit types for visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitType {
    #[default]
    Normal = 0,
    Reverse = 1,
    Highlight = 2,
    Merge = 3,
    CherryPick = 4,
}

impl CommitType {
    /// Parse a commit type from a string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "REVERSE" => Self::Reverse,
            "HIGHLIGHT" => Self::Highlight,
            "MERGE" => Self::Merge,
            "CHERRY_PICK" | "CHERRY-PICK" => Self::CherryPick,
            _ => Self::Normal,
        }
    }

    /// Get the numeric value for this commit type
    pub fn as_num(&self) -> i32 {
        *self as i32
    }
}

/// Diagram orientation/direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramOrientation {
    #[default]
    LeftToRight, // LR
    TopToBottom, // TB
    BottomToTop, // BT
}

impl DiagramOrientation {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TB" | "TD" => Self::TopToBottom,
            "BT" => Self::BottomToTop,
            _ => Self::LeftToRight,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeftToRight => "LR",
            Self::TopToBottom => "TB",
            Self::BottomToTop => "BT",
        }
    }
}

/// A commit in the git graph
#[derive(Debug, Clone)]
pub struct Commit {
    /// Unique commit identifier
    pub id: String,
    /// Commit message
    pub message: String,
    /// Sequence number (order of creation)
    pub seq: usize,
    /// Type of commit for visualization
    pub commit_type: CommitType,
    /// Tags associated with this commit
    pub tags: Vec<String>,
    /// Parent commit IDs
    pub parents: Vec<String>,
    /// Branch this commit belongs to
    pub branch: String,
    /// Whether this commit has a custom type
    pub custom_type: Option<CommitType>,
    /// Whether this commit has a custom ID
    pub custom_id: bool,
}

impl Commit {
    pub fn new(id: String, branch: String, seq: usize) -> Self {
        Self {
            id,
            message: String::new(),
            seq,
            commit_type: CommitType::Normal,
            tags: Vec::new(),
            parents: Vec::new(),
            branch,
            custom_type: None,
            custom_id: false,
        }
    }
}

/// Branch configuration
#[derive(Debug, Clone)]
pub struct BranchConfig {
    pub name: String,
    pub order: Option<i32>,
    pub created_seq: usize,
}

/// The git graph database
#[derive(Debug, Clone)]
pub struct GitGraphDb {
    /// All commits by ID
    commits: HashMap<String, Commit>,
    /// Current HEAD commit
    head: Option<String>,
    /// Branch configurations
    branch_config: HashMap<String, BranchConfig>,
    /// Branch name -> HEAD commit ID mapping
    branches: HashMap<String, Option<String>>,
    /// Current branch name
    current_branch: String,
    /// Diagram direction
    direction: DiagramOrientation,
    /// Commit sequence counter
    seq: usize,
    /// Branch sequence counter (for deterministic ordering)
    branch_seq: usize,
    /// Accessibility title
    pub acc_title: String,
    /// Accessibility description
    pub acc_descr: String,
    /// Diagram title
    pub diagram_title: String,
}

impl Default for GitGraphDb {
    fn default() -> Self {
        Self::new()
    }
}

impl GitGraphDb {
    pub fn new() -> Self {
        let mut db = Self {
            commits: HashMap::new(),
            head: None,
            branch_config: HashMap::new(),
            branches: HashMap::new(),
            current_branch: "main".to_string(),
            direction: DiagramOrientation::LeftToRight,
            seq: 0,
            branch_seq: 0,
            acc_title: String::new(),
            acc_descr: String::new(),
            diagram_title: String::new(),
        };
        // Initialize main branch
        db.branches.insert("main".to_string(), None);
        db.branch_config.insert(
            "main".to_string(),
            BranchConfig {
                name: "main".to_string(),
                order: Some(0),
                created_seq: db.branch_seq,
            },
        );
        db.branch_seq += 1;
        db
    }

    pub fn clear(&mut self) {
        self.commits.clear();
        self.head = None;
        self.branch_config.clear();
        self.branches.clear();
        self.current_branch = "main".to_string();
        self.direction = DiagramOrientation::LeftToRight;
        self.seq = 0;
        self.branch_seq = 0;
        self.acc_title.clear();
        self.acc_descr.clear();
        self.diagram_title.clear();
        // Re-initialize main branch
        self.branches.insert("main".to_string(), None);
        self.branch_config.insert(
            "main".to_string(),
            BranchConfig {
                name: "main".to_string(),
                order: Some(0),
                created_seq: self.branch_seq,
            },
        );
        self.branch_seq += 1;
    }

    /// Set the diagram direction
    pub fn set_direction(&mut self, dir: DiagramOrientation) {
        self.direction = dir;
    }

    /// Get the diagram direction
    pub fn get_direction(&self) -> DiagramOrientation {
        self.direction
    }

    /// Get the current branch
    pub fn get_current_branch(&self) -> &str {
        &self.current_branch
    }

    /// Get all commits
    pub fn get_commits(&self) -> &HashMap<String, Commit> {
        &self.commits
    }

    /// Get all branches
    pub fn get_branches(&self) -> &HashMap<String, Option<String>> {
        &self.branches
    }

    /// Get branches as array of objects (for compatibility)
    pub fn get_branches_as_obj_array(&self) -> Vec<BranchConfig> {
        let mut branches: Vec<BranchConfig> = self.branch_config.values().cloned().collect();
        branches.sort_by(|a, b| {
            let order_a = a
                .order
                .map(|o| o as f64)
                .unwrap_or(0.0 + (a.created_seq as f64 / 1000.0));
            let order_b = b
                .order
                .map(|o| o as f64)
                .unwrap_or(0.0 + (b.created_seq as f64 / 1000.0));
            order_a
                .partial_cmp(&order_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        branches
    }

    /// Get the HEAD commit
    pub fn get_head(&self) -> Option<&Commit> {
        self.head.as_ref().and_then(|id| self.commits.get(id))
    }

    /// Create a new commit
    pub fn commit(
        &mut self,
        id: Option<String>,
        message: String,
        commit_type: CommitType,
        tags: Vec<String>,
    ) {
        let custom_id = id.is_some();
        let commit_id = id.unwrap_or_else(|| self.generate_commit_id());

        let parent = self.branches.get(&self.current_branch).cloned().flatten();
        let parents = parent.map(|p| vec![p]).unwrap_or_default();

        let mut commit = Commit::new(commit_id.clone(), self.current_branch.clone(), self.seq);
        commit.message = message;
        commit.commit_type = commit_type;
        commit.tags = tags;
        commit.parents = parents;
        commit.custom_id = custom_id;
        if commit_type != CommitType::Normal {
            commit.custom_type = Some(commit_type);
        }

        self.seq += 1;
        self.head = Some(commit_id.clone());
        self.branches
            .insert(self.current_branch.clone(), Some(commit_id.clone()));
        self.commits.insert(commit_id, commit);
    }

    /// Create a new branch
    pub fn branch(&mut self, name: String, order: Option<i32>) {
        // New branch points to the same commit as current branch
        let current_head = self.branches.get(&self.current_branch).cloned().flatten();
        self.branches.insert(name.clone(), current_head);
        self.branch_config.insert(
            name.clone(),
            BranchConfig {
                name,
                order,
                created_seq: self.branch_seq,
            },
        );
        self.branch_seq += 1;
    }

    /// Checkout/switch to a branch
    pub fn checkout(&mut self, branch: &str) {
        if self.branches.contains_key(branch) {
            self.current_branch = branch.to_string();
            self.head = self.branches.get(branch).cloned().flatten();
        }
    }

    /// Merge a branch into the current branch
    pub fn merge(
        &mut self,
        branch: &str,
        id: Option<String>,
        commit_type: CommitType,
        tags: Vec<String>,
    ) {
        let source_head = self.branches.get(branch).cloned().flatten();
        let current_head = self.branches.get(&self.current_branch).cloned().flatten();

        let custom_id = id.is_some();
        let commit_id = id.unwrap_or_else(|| self.generate_commit_id());

        let mut parents = Vec::new();
        if let Some(p) = current_head {
            parents.push(p);
        }
        if let Some(p) = source_head {
            parents.push(p);
        }

        let mut commit = Commit::new(commit_id.clone(), self.current_branch.clone(), self.seq);
        commit.commit_type = if commit_type == CommitType::Normal {
            CommitType::Merge
        } else {
            commit_type
        };
        commit.tags = tags;
        commit.parents = parents;
        commit.custom_id = custom_id;

        self.seq += 1;
        self.head = Some(commit_id.clone());
        self.branches
            .insert(self.current_branch.clone(), Some(commit_id.clone()));
        self.commits.insert(commit_id, commit);
    }

    /// Cherry-pick a commit
    pub fn cherry_pick(
        &mut self,
        source_id: &str,
        id: Option<String>,
        parent: Option<String>,
        tags: Vec<String>,
    ) {
        if !self.commits.contains_key(source_id) {
            return;
        }

        let custom_id = id.is_some();
        let commit_id = id.unwrap_or_else(|| self.generate_commit_id());

        let parent_id =
            parent.or_else(|| self.branches.get(&self.current_branch).cloned().flatten());

        let mut commit = Commit::new(commit_id.clone(), self.current_branch.clone(), self.seq);
        commit.commit_type = CommitType::CherryPick;
        commit.tags = tags;
        commit.parents = parent_id.map(|p| vec![p]).unwrap_or_default();
        commit.custom_id = custom_id;

        self.seq += 1;
        self.head = Some(commit_id.clone());
        self.branches
            .insert(self.current_branch.clone(), Some(commit_id.clone()));
        self.commits.insert(commit_id, commit);
    }

    /// Check if commit_a is an ancestor of commit_b
    #[allow(dead_code)]
    fn is_ancestor(&self, commit_a: &str, commit_b: &str) -> bool {
        if commit_a == commit_b {
            return true;
        }

        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![commit_b.to_string()];

        while let Some(current) = queue.pop() {
            if current == commit_a {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(commit) = self.commits.get(&current) {
                for parent in &commit.parents {
                    queue.push(parent.clone());
                }
            }
        }

        false
    }

    /// Generate a unique commit ID
    fn generate_commit_id(&self) -> String {
        format!("{}-{}", self.seq, uuid_v4_short())
    }
}

/// Generate a short UUID-like string
fn uuid_v4_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now % 0xFFFFFFFF)
}
