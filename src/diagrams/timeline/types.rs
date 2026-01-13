//! Timeline diagram types
//!
//! Timeline diagrams show events and tasks organized by time periods/sections.

/// A task/event in the timeline
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTask {
    /// Unique task identifier
    pub id: usize,
    /// Section this task belongs to
    pub section: String,
    /// Task name/period description
    pub task: String,
    /// Events associated with this task
    pub events: Vec<String>,
}

impl TimelineTask {
    /// Create a new timeline task
    pub fn new(id: usize, section: String, task: String) -> Self {
        Self {
            id,
            section,
            task,
            events: Vec::new(),
        }
    }

    /// Add an event to this task
    pub fn add_event(&mut self, event: String) {
        self.events.push(event);
    }
}

/// The Timeline database that stores all diagram data
#[derive(Debug, Clone, Default)]
pub struct TimelineDb {
    /// Diagram title
    pub title: String,
    /// All sections in order
    pub sections: Vec<String>,
    /// Current section name
    current_section: String,
    /// All tasks
    tasks: Vec<TimelineTask>,
    /// Counter for task IDs
    task_counter: usize,
}

impl TimelineDb {
    /// Create a new empty TimelineDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the diagram title
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Get the diagram title
    pub fn get_title(&self) -> &str {
        &self.title
    }

    /// Add a section
    pub fn add_section(&mut self, name: &str) {
        let name = name.trim().to_string();
        self.current_section = name.clone();
        if !self.sections.contains(&name) {
            self.sections.push(name);
        }
    }

    /// Get all sections
    pub fn get_sections(&self) -> &[String] {
        &self.sections
    }

    /// Add a task with optional events
    ///
    /// The task string can contain events separated by colons:
    /// - "task1" - just a task
    /// - "task1: event1" - task with one event
    /// - "task1: event1: event2" - task with multiple events
    pub fn add_task(&mut self, period: &str, events: &[&str]) {
        self.task_counter += 1;

        // Parse period - split on ": " to preserve URLs like http://
        // The first part before any ": " is the task name
        let parts: Vec<&str> = period.splitn(2, ": ").collect();
        let task_name = parts[0].trim().to_string();

        let mut task =
            TimelineTask::new(self.task_counter, self.current_section.clone(), task_name);

        // Add event from period if present
        if parts.len() > 1 {
            // The rest after first ": " may contain multiple events
            // Split on ": " to preserve URLs
            for event in parts[1].split(": ") {
                let event = event.trim();
                if !event.is_empty() {
                    task.add_event(event.to_string());
                }
            }
        }

        // Add additional events
        for event in events {
            let event = event.trim();
            if !event.is_empty() {
                task.add_event(event.to_string());
            }
        }

        self.tasks.push(task);
    }

    /// Add events to the most recent task
    pub fn add_event(&mut self, event: &str) {
        if let Some(task) = self.tasks.last_mut() {
            // Event may contain multiple events separated by ": "
            // Use ": " to preserve URLs
            for e in event.split(": ") {
                let e = e.trim();
                if !e.is_empty() {
                    task.add_event(e.to_string());
                }
            }
        }
    }

    /// Get all tasks
    pub fn get_tasks(&self) -> &[TimelineTask] {
        &self.tasks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // Section tests
    // ==================

    #[test]
    fn test_simple_section_definition() {
        let mut db = TimelineDb::new();
        db.add_section("abc-123");
        assert_eq!(db.get_sections(), &["abc-123"]);
    }

    #[test]
    fn test_section_and_two_tasks() {
        let mut db = TimelineDb::new();
        db.add_section("abc-123");
        db.add_task("task1", &[]);
        db.add_task("task2", &[]);

        let tasks = db.get_tasks();
        assert_eq!(tasks.len(), 2);

        for task in tasks {
            assert_eq!(task.section, "abc-123");
            assert!(task.task == "task1" || task.task == "task2");
        }
    }

    #[test]
    fn test_two_sections_with_tasks() {
        let mut db = TimelineDb::new();
        db.add_section("abc-123");
        db.add_task("task1", &[]);
        db.add_task("task2", &[]);
        db.add_section("abc-456");
        db.add_task("task3", &[]);
        db.add_task("task4", &[]);

        assert_eq!(db.get_sections(), &["abc-123", "abc-456"]);

        let tasks = db.get_tasks();
        assert_eq!(tasks.len(), 4);

        for task in tasks {
            if task.section == "abc-123" {
                assert!(task.task == "task1" || task.task == "task2");
            } else {
                assert_eq!(task.section, "abc-456");
                assert!(task.task == "task3" || task.task == "task4");
            }
        }
    }

    // ==================
    // Event tests
    // ==================

    #[test]
    fn test_task_with_single_event() {
        let mut db = TimelineDb::new();
        db.add_section("abc-123");
        db.add_task("task1: event1", &[]);

        let tasks = db.get_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task, "task1");
        assert_eq!(tasks[0].events, vec!["event1"]);
    }

    #[test]
    fn test_task_with_multiple_events() {
        let mut db = TimelineDb::new();
        db.add_section("abc-123");
        db.add_task("task1: event1", &[]);
        db.add_task("task2: event2: event3", &[]);

        let tasks = db.get_tasks();
        assert_eq!(tasks.len(), 2);

        assert_eq!(tasks[0].task, "task1");
        assert_eq!(tasks[0].events, vec!["event1"]);

        assert_eq!(tasks[1].task, "task2");
        assert_eq!(tasks[1].events, vec!["event2", "event3"]);
    }

    #[test]
    fn test_task_with_markdown_link_event() {
        let mut db = TimelineDb::new();
        db.add_section("abc-123");
        db.add_task("task1: [event1](http://example.com)", &[]);
        db.add_task("task2: event2: event3", &[]);

        let tasks = db.get_tasks();

        assert_eq!(tasks[0].task, "task1");
        assert_eq!(tasks[0].events, vec!["[event1](http://example.com)"]);

        assert_eq!(tasks[1].task, "task2");
        assert_eq!(tasks[1].events, vec!["event2", "event3"]);
    }

    #[test]
    fn test_multiline_events() {
        let mut db = TimelineDb::new();
        db.add_section("abc-123");
        db.add_task("task1: event1", &[]);
        db.add_task("task2: event2: event3", &[]);
        // Continuation line adds more events to task2
        db.add_event("event4: event5");

        let tasks = db.get_tasks();

        assert_eq!(tasks[0].task, "task1");
        assert_eq!(tasks[0].events, vec!["event1"]);

        assert_eq!(tasks[1].task, "task2");
        assert_eq!(
            tasks[1].events,
            vec!["event2", "event3", "event4", "event5"]
        );
    }

    // ==================
    // Special character tests
    // ==================

    #[test]
    fn test_title_with_semicolons() {
        let mut db = TimelineDb::new();
        db.set_title(";my;title;");
        assert_eq!(db.get_title(), ";my;title;");
    }

    #[test]
    fn test_section_with_semicolons() {
        let mut db = TimelineDb::new();
        db.add_section(";a;bc-123;");
        assert_eq!(db.get_sections(), &[";a;bc-123;"]);
    }

    #[test]
    fn test_events_with_semicolons() {
        let mut db = TimelineDb::new();
        db.add_section(";a;bc-123;");
        // Manually add task with events containing semicolons
        let mut task = TimelineTask::new(1, ";a;bc-123;".to_string(), ";ta;sk1;".to_string());
        task.add_event(";ev;ent1; ".to_string());
        task.add_event(";ev;ent2; ".to_string());
        task.add_event(";ev;ent3;".to_string());
        db.tasks.push(task);
        db.task_counter = 1;

        let tasks = db.get_tasks();
        assert_eq!(
            tasks[0].events,
            vec![";ev;ent1; ", ";ev;ent2; ", ";ev;ent3;"]
        );
    }

    #[test]
    fn test_title_with_hashtags() {
        let mut db = TimelineDb::new();
        db.set_title("#my#title#");
        assert_eq!(db.get_title(), "#my#title#");
    }

    #[test]
    fn test_section_with_hashtags() {
        let mut db = TimelineDb::new();
        db.add_section("#a#bc-123#");
        assert_eq!(db.get_sections(), &["#a#bc-123#"]);
    }

    #[test]
    fn test_events_with_hashtags() {
        let mut db = TimelineDb::new();
        db.add_section("#a#bc-123#");
        // The task parsing handles colons, but hashtags in content are preserved
        let mut task = TimelineTask::new(1, "#a#bc-123#".to_string(), "task1".to_string());
        task.add_event("#ev#ent1# ".to_string());
        task.add_event("#ev#ent2# ".to_string());
        task.add_event("#ev#ent3#".to_string());
        db.tasks.push(task);
        db.task_counter = 1;

        let tasks = db.get_tasks();
        assert_eq!(tasks[0].task, "task1");
        assert_eq!(
            tasks[0].events,
            vec!["#ev#ent1# ", "#ev#ent2# ", "#ev#ent3#"]
        );
    }

    // ==================
    // Clear tests
    // ==================

    #[test]
    fn test_clear() {
        let mut db = TimelineDb::new();
        db.set_title("test");
        db.add_section("section1");
        db.add_task("task1", &[]);
        db.clear();

        assert_eq!(db.get_title(), "");
        assert!(db.get_sections().is_empty());
        assert!(db.get_tasks().is_empty());
    }
}
