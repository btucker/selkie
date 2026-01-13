//! Git graph diagram support
//!
//! The git graph diagram visualizes git repository history with
//! commits, branches, merges, and cherry-picks.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_tests {
        use super::*;

        #[test]
        fn should_initialize_with_main_branch() {
            let db = GitGraphDb::new();
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_direction(), DiagramOrientation::LeftToRight);
            assert_eq!(db.get_branches().len(), 1);
            assert!(db.get_branches().contains_key("main"));
        }

        #[test]
        fn should_handle_single_commit() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Normal, vec![]);

            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_direction(), DiagramOrientation::LeftToRight);
            assert_eq!(db.get_branches().len(), 1);
        }

        #[test]
        fn should_handle_direction_tb() {
            let mut db = GitGraphDb::new();
            db.set_direction(DiagramOrientation::TopToBottom);
            db.commit(None, String::new(), CommitType::Normal, vec![]);

            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_direction(), DiagramOrientation::TopToBottom);
        }

        #[test]
        fn should_handle_direction_bt() {
            let mut db = GitGraphDb::new();
            db.set_direction(DiagramOrientation::BottomToTop);
            db.commit(None, String::new(), CommitType::Normal, vec![]);

            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_direction(), DiagramOrientation::BottomToTop);
        }

        #[test]
        fn should_checkout_branch() {
            let mut db = GitGraphDb::new();
            db.branch("new".to_string(), None);
            db.checkout("new");

            assert_eq!(db.get_commits().len(), 0);
            assert_eq!(db.get_current_branch(), "new");
        }

        #[test]
        fn should_add_commits_to_checked_out_branch() {
            let mut db = GitGraphDb::new();
            db.branch("new".to_string(), None);
            db.checkout("new");
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.commit(None, String::new(), CommitType::Normal, vec![]);

            assert_eq!(db.get_commits().len(), 2);
            assert_eq!(db.get_current_branch(), "new");

            let branch_commit = db.get_branches().get("new").cloned().flatten();
            assert!(branch_commit.is_some());
        }

        #[test]
        fn should_handle_commit_with_message() {
            let mut db = GitGraphDb::new();
            db.commit(None, "a commit".to_string(), CommitType::Normal, vec![]);

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.message, "a commit");
            assert_eq!(db.get_current_branch(), "main");
        }

        #[test]
        fn should_generate_branches_array() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.branch("b1".to_string(), None);
            db.checkout("b1");
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.branch("b2".to_string(), None);

            // Note: main branch config isn't automatically added
            let branches = db.get_branches();
            assert_eq!(branches.len(), 3); // main, b1, b2
        }
    }

    mod advanced_tests {
        use super::*;

        #[test]
        fn should_handle_commit_with_no_params_auto_id() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Normal, vec![]);

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.message, "");
            assert!(!commit.id.is_empty());
            assert!(commit.tags.is_empty());
            assert_eq!(commit.commit_type, CommitType::Normal);
        }

        #[test]
        fn should_handle_commit_with_custom_id() {
            let mut db = GitGraphDb::new();
            db.commit(
                Some("1111".to_string()),
                String::new(),
                CommitType::Normal,
                vec![],
            );

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.get("1111").unwrap();
            assert_eq!(commit.message, "");
            assert_eq!(commit.id, "1111");
            assert!(commit.tags.is_empty());
            assert_eq!(commit.commit_type, CommitType::Normal);
        }

        #[test]
        fn should_handle_commit_with_tag() {
            let mut db = GitGraphDb::new();
            db.commit(
                None,
                String::new(),
                CommitType::Normal,
                vec!["test".to_string()],
            );

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.message, "");
            assert_eq!(commit.tags, vec!["test"]);
            assert_eq!(commit.commit_type, CommitType::Normal);
        }

        #[test]
        fn should_handle_commit_type_highlight() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Highlight, vec![]);

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.commit_type, CommitType::Highlight);
            assert_eq!(commit.commit_type.as_num(), 2);
        }

        #[test]
        fn should_handle_commit_type_reverse() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Reverse, vec![]);

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.commit_type, CommitType::Reverse);
            assert_eq!(commit.commit_type.as_num(), 1);
        }

        #[test]
        fn should_handle_commit_type_normal() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Normal, vec![]);

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.commit_type, CommitType::Normal);
            assert_eq!(commit.commit_type.as_num(), 0);
        }

        #[test]
        fn should_handle_commit_with_msg() {
            let mut db = GitGraphDb::new();
            db.commit(None, "test commit".to_string(), CommitType::Normal, vec![]);

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.message, "test commit");
        }

        #[test]
        fn should_handle_commit_with_id_and_tag() {
            let mut db = GitGraphDb::new();
            db.commit(
                Some("1111".to_string()),
                String::new(),
                CommitType::Normal,
                vec!["test tag".to_string()],
            );

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.get("1111").unwrap();
            assert_eq!(commit.message, "");
            assert_eq!(commit.id, "1111");
            assert_eq!(commit.tags, vec!["test tag"]);
            assert_eq!(commit.commit_type, CommitType::Normal);
        }

        #[test]
        fn should_handle_commit_with_type_and_tag() {
            let mut db = GitGraphDb::new();
            db.commit(
                None,
                String::new(),
                CommitType::Highlight,
                vec!["test tag".to_string()],
            );

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.values().next().unwrap();
            assert_eq!(commit.tags, vec!["test tag"]);
            assert_eq!(commit.commit_type, CommitType::Highlight);
        }

        #[test]
        fn should_handle_commit_with_all_params() {
            let mut db = GitGraphDb::new();
            db.commit(
                Some("1111".to_string()),
                "test msg".to_string(),
                CommitType::Reverse,
                vec!["test tag".to_string()],
            );

            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);

            let commit = commits.get("1111").unwrap();
            assert_eq!(commit.message, "test msg");
            assert_eq!(commit.id, "1111");
            assert_eq!(commit.tags, vec!["test tag"]);
            assert_eq!(commit.commit_type, CommitType::Reverse);
        }

        #[test]
        fn should_handle_3_straight_commits() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.commit(None, String::new(), CommitType::Normal, vec![]);

            assert_eq!(db.get_commits().len(), 3);
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_branches().len(), 1);
        }

        #[test]
        fn should_handle_new_branch_creation() {
            let mut db = GitGraphDb::new();
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.branch("testBranch".to_string(), None);

            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_branches().len(), 2);
            assert!(db.get_branches().contains_key("testBranch"));
        }
    }

    mod merge_tests {
        use super::*;

        #[test]
        fn should_handle_merge_when_noop() {
            let mut db = GitGraphDb::new();
            // commit on main
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            // create and switch to new branch
            db.branch("newbranch".to_string(), None);
            db.checkout("newbranch");
            // commits on new branch
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            // merge main into newbranch (noop)
            db.merge("main", None, CommitType::Normal, vec![]);

            assert_eq!(db.get_commits().len(), 4); // 1 + 2 + merge commit
            assert_eq!(db.get_current_branch(), "newbranch");
            // After merge, branches point to different commits
            assert_ne!(
                db.get_branches().get("newbranch"),
                db.get_branches().get("main")
            );
        }

        #[test]
        fn should_handle_merge_with_2_parents() {
            let mut db = GitGraphDb::new();
            // commit on main
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            // create and switch to new branch
            db.branch("newbranch".to_string(), None);
            db.checkout("newbranch");
            // commits on new branch
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            // switch back to main and commit
            db.checkout("main");
            db.commit(None, String::new(), CommitType::Normal, vec![]);
            // merge newbranch into main
            db.merge("newbranch", None, CommitType::Normal, vec![]);

            assert_eq!(db.get_commits().len(), 5);
            assert_eq!(db.get_current_branch(), "main");

            // Get the merge commit
            let main_head = db.get_branches().get("main").cloned().flatten().unwrap();
            let merge_commit = db.get_commits().get(&main_head).unwrap();
            assert_eq!(merge_commit.parents.len(), 2);
        }
    }

    mod commit_type_tests {
        use super::*;

        #[test]
        fn commit_type_from_str() {
            assert_eq!(CommitType::from_str("NORMAL"), CommitType::Normal);
            assert_eq!(CommitType::from_str("REVERSE"), CommitType::Reverse);
            assert_eq!(CommitType::from_str("HIGHLIGHT"), CommitType::Highlight);
            assert_eq!(CommitType::from_str("unknown"), CommitType::Normal);
        }

        #[test]
        fn commit_type_as_num() {
            assert_eq!(CommitType::Normal.as_num(), 0);
            assert_eq!(CommitType::Reverse.as_num(), 1);
            assert_eq!(CommitType::Highlight.as_num(), 2);
            assert_eq!(CommitType::Merge.as_num(), 3);
            assert_eq!(CommitType::CherryPick.as_num(), 4);
        }
    }

    mod orientation_tests {
        use super::*;

        #[test]
        fn orientation_from_str() {
            assert_eq!(
                DiagramOrientation::from_str("LR"),
                DiagramOrientation::LeftToRight
            );
            assert_eq!(
                DiagramOrientation::from_str("TB"),
                DiagramOrientation::TopToBottom
            );
            assert_eq!(
                DiagramOrientation::from_str("TD"),
                DiagramOrientation::TopToBottom
            );
            assert_eq!(
                DiagramOrientation::from_str("BT"),
                DiagramOrientation::BottomToTop
            );
        }

        #[test]
        fn orientation_as_str() {
            assert_eq!(DiagramOrientation::LeftToRight.as_str(), "LR");
            assert_eq!(DiagramOrientation::TopToBottom.as_str(), "TB");
            assert_eq!(DiagramOrientation::BottomToTop.as_str(), "BT");
        }
    }

    mod clear_tests {
        use super::*;

        #[test]
        fn should_clear_state() {
            let mut db = GitGraphDb::new();
            db.commit(None, "test".to_string(), CommitType::Normal, vec![]);
            db.branch("test".to_string(), None);
            db.set_direction(DiagramOrientation::TopToBottom);

            db.clear();

            assert_eq!(db.get_commits().len(), 0);
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_direction(), DiagramOrientation::LeftToRight);
            assert_eq!(db.get_branches().len(), 1);
            assert!(db.get_branches().contains_key("main"));
        }
    }
}
