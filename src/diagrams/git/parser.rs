//! Git graph diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::{CommitType, DiagramOrientation, GitGraphDb};

#[derive(Parser)]
#[grammar = "diagrams/git/git.pest"]
pub struct GitGraphParser;

/// Parse a git graph diagram and return the populated database
pub fn parse(input: &str) -> Result<GitGraphDb, String> {
    let mut db = GitGraphDb::new();

    let pairs =
        GitGraphParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::direction => {
                        let dir = DiagramOrientation::from_str(inner.as_str());
                        db.set_direction(dir);
                    }
                    Rule::document => {
                        process_document(&mut db, inner)?;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(db)
}

fn process_document(db: &mut GitGraphDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(db: &mut GitGraphDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::acc_title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::line_content {
                    db.acc_title = inner.as_str().trim().to_string();
                }
            }
        }
        Rule::acc_descr_stmt => {
            process_acc_descr(db, pair)?;
        }
        Rule::commit_stmt => {
            process_commit(db, pair)?;
        }
        Rule::branch_stmt => {
            process_branch(db, pair)?;
        }
        Rule::checkout_stmt => {
            process_checkout(db, pair)?;
        }
        Rule::merge_stmt => {
            process_merge(db, pair)?;
        }
        Rule::cherry_pick_stmt => {
            process_cherry_pick(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_acc_descr(db: &mut GitGraphDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::acc_descr_single => {
                for content in inner.into_inner() {
                    if content.as_rule() == Rule::line_content {
                        db.acc_descr = content.as_str().trim().to_string();
                    }
                }
            }
            Rule::acc_descr_multi => {
                for content in inner.into_inner() {
                    if content.as_rule() == Rule::multiline_content {
                        let text = content.as_str().trim();
                        let cleaned: String = text
                            .lines()
                            .map(|l| l.trim())
                            .collect::<Vec<_>>()
                            .join("\n");
                        db.acc_descr = cleaned;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn process_commit(db: &mut GitGraphDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut id: Option<String> = None;
    let mut message = String::new();
    let mut commit_type = CommitType::Normal;
    let mut tags: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::commit_options {
            for opt in inner.into_inner() {
                match opt.as_rule() {
                    Rule::commit_id => {
                        for id_inner in opt.into_inner() {
                            if id_inner.as_rule() == Rule::quoted_string {
                                id = Some(unquote(id_inner.as_str()));
                            }
                        }
                    }
                    Rule::commit_tag => {
                        for tag_inner in opt.into_inner() {
                            if tag_inner.as_rule() == Rule::quoted_string {
                                tags.push(unquote(tag_inner.as_str()));
                            }
                        }
                    }
                    Rule::commit_type => {
                        for type_inner in opt.into_inner() {
                            if type_inner.as_rule() == Rule::type_value {
                                commit_type = CommitType::from_str(type_inner.as_str());
                            }
                        }
                    }
                    Rule::commit_msg => {
                        for msg_inner in opt.into_inner() {
                            if msg_inner.as_rule() == Rule::quoted_string {
                                message = unquote(msg_inner.as_str());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    db.commit(id, message, commit_type, tags);
    Ok(())
}

fn process_branch(db: &mut GitGraphDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut name = String::new();
    let mut order: Option<i32> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::branch_name => {
                name = inner.as_str().to_string();
            }
            Rule::branch_options => {
                for opt in inner.into_inner() {
                    if opt.as_rule() == Rule::branch_order {
                        for order_inner in opt.into_inner() {
                            if order_inner.as_rule() == Rule::integer {
                                order = order_inner.as_str().parse().ok();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    db.branch(name, order);
    Ok(())
}

fn process_checkout(db: &mut GitGraphDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::branch_name {
            db.checkout(inner.as_str());
        }
    }
    Ok(())
}

fn process_merge(db: &mut GitGraphDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut branch = String::new();
    let mut id: Option<String> = None;
    let mut commit_type = CommitType::Normal;
    let mut tags: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::branch_name => {
                branch = inner.as_str().to_string();
            }
            Rule::merge_options => {
                for opt in inner.into_inner() {
                    match opt.as_rule() {
                        Rule::merge_id => {
                            for id_inner in opt.into_inner() {
                                if id_inner.as_rule() == Rule::quoted_string {
                                    id = Some(unquote(id_inner.as_str()));
                                }
                            }
                        }
                        Rule::merge_tag => {
                            for tag_inner in opt.into_inner() {
                                if tag_inner.as_rule() == Rule::quoted_string {
                                    tags.push(unquote(tag_inner.as_str()));
                                }
                            }
                        }
                        Rule::merge_type => {
                            for type_inner in opt.into_inner() {
                                if type_inner.as_rule() == Rule::type_value {
                                    commit_type = CommitType::from_str(type_inner.as_str());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    db.merge(&branch, id, commit_type, tags);
    Ok(())
}

fn process_cherry_pick(
    db: &mut GitGraphDb,
    pair: pest::iterators::Pair<Rule>,
) -> Result<(), String> {
    let mut source_id = String::new();
    let mut id: Option<String> = None;
    let mut parent: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::cherry_pick_options {
            for opt in inner.into_inner() {
                match opt.as_rule() {
                    Rule::cherry_pick_id => {
                        for id_inner in opt.into_inner() {
                            if id_inner.as_rule() == Rule::quoted_string {
                                // First id is source, second would be new commit id
                                if source_id.is_empty() {
                                    source_id = unquote(id_inner.as_str());
                                } else {
                                    id = Some(unquote(id_inner.as_str()));
                                }
                            }
                        }
                    }
                    Rule::cherry_pick_parent => {
                        for parent_inner in opt.into_inner() {
                            if parent_inner.as_rule() == Rule::quoted_string {
                                parent = Some(unquote(parent_inner.as_str()));
                            }
                        }
                    }
                    Rule::cherry_pick_tag => {
                        for tag_inner in opt.into_inner() {
                            if tag_inner.as_rule() == Rule::quoted_string {
                                tags.push(unquote(tag_inner.as_str()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    db.cherry_pick(&source_id, id, parent, tags);
    Ok(())
}

/// Remove surrounding quotes from a string
fn unquote(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_gitgraph_definition() {
            let result = parse("gitGraph:\ncommit\n");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_current_branch(), "main");
            assert_eq!(db.get_direction(), DiagramOrientation::LeftToRight);
            assert_eq!(db.get_branches().len(), 1);
        }

        #[test]
        fn should_handle_direction_tb() {
            let result = parse("gitGraph TB:\ncommit\n");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_direction(), DiagramOrientation::TopToBottom);
        }

        #[test]
        fn should_handle_direction_bt() {
            let result = parse("gitGraph BT:\ncommit\n");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_direction(), DiagramOrientation::BottomToTop);
        }

        #[test]
        fn should_checkout_branch() {
            let result = parse("gitGraph:\nbranch new\ncheckout new\n");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 0);
            assert_eq!(db.get_current_branch(), "new");
        }

        #[test]
        fn should_switch_branch() {
            let result = parse("gitGraph:\nbranch new\nswitch new\n");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 0);
            assert_eq!(db.get_current_branch(), "new");
        }

        #[test]
        fn should_add_commits_to_checked_out_branch() {
            let result = parse("gitGraph:\nbranch new\ncheckout new\ncommit\ncommit\n");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 2);
            assert_eq!(db.get_current_branch(), "new");
        }

        #[test]
        fn should_handle_commit_with_message() {
            let result = parse("gitGraph:\ncommit \"a commit\"\n");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);
            let commit = commits.values().next().unwrap();
            assert_eq!(commit.message, "a commit");
        }
    }

    mod advanced_commits {
        use super::*;

        #[test]
        fn should_handle_commit_with_auto_id() {
            let result = parse("gitGraph:\ncommit\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let commits = db.get_commits();
            assert_eq!(commits.len(), 1);
            let commit = commits.values().next().unwrap();
            assert!(!commit.id.is_empty());
            assert!(commit.tags.is_empty());
            assert_eq!(commit.commit_type, CommitType::Normal);
        }

        #[test]
        fn should_handle_commit_with_custom_id() {
            let result = parse("gitGraph:\ncommit id:\"1111\"\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let commit = db.get_commits().get("1111").unwrap();
            assert_eq!(commit.id, "1111");
            assert_eq!(commit.commit_type, CommitType::Normal);
        }

        #[test]
        fn should_handle_commit_with_tag() {
            let result = parse("gitGraph:\ncommit tag:\"test\"\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let commit = db.get_commits().values().next().unwrap();
            assert_eq!(commit.tags, vec!["test"]);
        }

        #[test]
        fn should_handle_commit_type_highlight() {
            let result = parse("gitGraph:\ncommit type: HIGHLIGHT\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let commit = db.get_commits().values().next().unwrap();
            assert_eq!(commit.commit_type, CommitType::Highlight);
            assert_eq!(commit.commit_type.as_num(), 2);
        }

        #[test]
        fn should_handle_commit_type_reverse() {
            let result = parse("gitGraph:\ncommit type: REVERSE\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let commit = db.get_commits().values().next().unwrap();
            assert_eq!(commit.commit_type, CommitType::Reverse);
            assert_eq!(commit.commit_type.as_num(), 1);
        }

        #[test]
        fn should_handle_commit_with_msg_key() {
            let result = parse("gitGraph:\ncommit msg: \"test commit\"\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let commit = db.get_commits().values().next().unwrap();
            assert_eq!(commit.message, "test commit");
        }

        #[test]
        fn should_handle_commit_with_id_and_tag() {
            let result = parse("gitGraph:\ncommit id:\"1111\" tag: \"test tag\"\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let commit = db.get_commits().get("1111").unwrap();
            assert_eq!(commit.id, "1111");
            assert_eq!(commit.tags, vec!["test tag"]);
        }

        #[test]
        fn should_handle_commit_with_all_params() {
            let result = parse(
                "gitGraph:\ncommit id:\"1111\" type: REVERSE tag:\"test tag\" msg:\"test msg\"\n",
            );
            assert!(result.is_ok());
            let db = result.unwrap();
            let commit = db.get_commits().get("1111").unwrap();
            assert_eq!(commit.id, "1111");
            assert_eq!(commit.message, "test msg");
            assert_eq!(commit.tags, vec!["test tag"]);
            assert_eq!(commit.commit_type, CommitType::Reverse);
        }

        #[test]
        fn should_handle_three_straight_commits() {
            let result = parse("gitGraph:\ncommit\ncommit\ncommit\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 3);
            assert_eq!(db.get_branches().len(), 1);
        }
    }

    mod branch_tests {
        use super::*;

        #[test]
        fn should_create_new_branch() {
            let result = parse("gitGraph:\ncommit\nbranch testBranch\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 1);
            assert_eq!(db.get_branches().len(), 2);
            assert!(db.get_branches().contains_key("testBranch"));
        }

        #[test]
        fn should_generate_branches_array() {
            let result =
                parse("gitGraph:\ncommit\nbranch b1\ncheckout b1\ncommit\ncommit\nbranch b2\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            let branches = db.get_branches();
            assert_eq!(branches.len(), 3); // main, b1, b2
        }

        #[test]
        fn should_handle_branch_with_order() {
            let result = parse("gitGraph:\nbranch develop order: 1\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.get_branches().contains_key("develop"));
            let branches = db.get_branches_as_obj_array();
            assert!(!branches.is_empty());
        }
    }

    mod merge_tests {
        use super::*;

        #[test]
        fn should_handle_merge_noop() {
            let result = parse("gitGraph:\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\ncommit\nmerge main\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 4); // 1 + 2 + merge
            assert_eq!(db.get_current_branch(), "newbranch");
        }

        #[test]
        fn should_handle_merge_with_two_parents() {
            let result = parse("gitGraph:\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\ncommit\ncheckout main\ncommit\nmerge newbranch\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 5);
            assert_eq!(db.get_current_branch(), "main");

            // Get the merge commit
            let main_head = db.get_branches().get("main").cloned().flatten().unwrap();
            let merge_commit = db.get_commits().get(&main_head).unwrap();
            assert_eq!(merge_commit.parents.len(), 2);
        }
    }

    mod cherry_pick_tests {
        use super::*;

        #[test]
        fn should_parse_cherry_pick() {
            let result = parse("gitGraph:\ncommit id:\"abc123\"\nbranch feature\ncheckout feature\ncherry-pick id:\"abc123\"\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 2); // original + cherry-pick
        }
    }

    mod comment_tests {
        use super::*;

        #[test]
        fn should_handle_comments() {
            let result = parse("gitGraph:\n%% This is a comment\ncommit\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.get_commits().len(), 1);
        }

        #[test]
        fn should_handle_comments_before_diagram() {
            let result = parse("%% Comment before\ngitGraph:\ncommit\n");
            assert!(result.is_ok());
        }
    }

    mod accessibility_tests {
        use super::*;

        #[test]
        fn should_parse_acc_title() {
            let result = parse("gitGraph:\naccTitle: Git History\ncommit\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_title, "Git History");
        }

        #[test]
        fn should_parse_acc_descr() {
            let result = parse("gitGraph:\naccDescr: A description\ncommit\n");
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.acc_descr, "A description");
        }
    }

    mod complex_diagrams {
        use super::*;

        #[test]
        fn should_parse_complex_diagram() {
            let input = r#"gitGraph TB:
    commit
    commit id:"feature-start" tag:"v1.0"
    branch feature
    checkout feature
    commit type: HIGHLIGHT msg:"Add feature"
    commit
    checkout main
    commit
    merge feature id:"merge-commit" tag:"v1.1"
"#;
            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.get_direction(), DiagramOrientation::TopToBottom);
            assert!(db.get_commits().len() >= 5);
            assert!(db.get_branches().contains_key("feature"));
        }
    }
}
