//! User journey diagram types
//!
//! User journey diagrams visualize user experiences as a series of tasks
//! with scores and actors involved.

/// A task in the user journey
#[derive(Debug, Clone, PartialEq)]
pub struct JourneyTask {
    /// Task name/description
    pub task: String,
    /// Score (1-5 typically representing user satisfaction)
    pub score: i32,
    /// People/actors involved in this task
    pub people: Vec<String>,
    /// Section this task belongs to
    pub section: String,
    /// Type (usually same as section)
    pub task_type: String,
}

impl JourneyTask {
    /// Create a new journey task
    pub fn new(task: String, section: String) -> Self {
        Self {
            task,
            score: 0,
            people: Vec::new(),
            section: section.clone(),
            task_type: section,
        }
    }
}

/// The User Journey database that stores all diagram data
#[derive(Debug, Clone, Default)]
pub struct JourneyDb {
    /// Diagram title
    pub title: String,
    /// Accessibility title
    pub acc_title: String,
    /// Accessibility description
    pub acc_description: String,
    /// All sections in order
    pub sections: Vec<String>,
    /// Current section name
    current_section: String,
    /// All tasks
    tasks: Vec<JourneyTask>,
}

impl JourneyDb {
    /// Create a new empty JourneyDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the accessibility title
    pub fn set_acc_title(&mut self, title: &str) {
        self.acc_title = title.to_string();
    }

    /// Get the accessibility title
    pub fn get_acc_title(&self) -> &str {
        &self.acc_title
    }

    /// Set the accessibility description
    pub fn set_acc_description(&mut self, desc: &str) {
        self.acc_description = desc.to_string();
    }

    /// Get the accessibility description
    pub fn get_acc_description(&self) -> &str {
        &self.acc_description
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

    /// Add a task with data
    ///
    /// Task data format: `:score:actor1, actor2, ...`
    /// Example: `:5:Dad` or `:3:Dad, Mum, Child#1`
    pub fn add_task(&mut self, task_name: &str, task_data: &str) {
        let mut task = JourneyTask::new(task_name.trim().to_string(), self.current_section.clone());

        // Parse task data format: :score:actor1, actor2
        // Or: score: actor1, actor2 (without leading colon)
        let data = task_data.trim();

        // Split by colon to get score and actors
        let parts: Vec<&str> = data.split(':').collect();

        // Find the score and people parts
        for part in parts.iter() {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Try to parse as score first
            if let Ok(score) = part.parse::<i32>() {
                task.score = score;
            } else {
                // Must be actors - comma separated
                for actor in part.split(',') {
                    let actor = actor.trim();
                    if !actor.is_empty() {
                        task.people.push(actor.to_string());
                    }
                }
            }
        }

        self.tasks.push(task);
    }

    /// Get all tasks
    pub fn get_tasks(&self) -> &[JourneyTask] {
        &self.tasks
    }

    /// Get all actors (sorted, unique)
    pub fn get_actors(&self) -> Vec<String> {
        let mut actors: Vec<String> = self
            .tasks
            .iter()
            .flat_map(|t| t.people.iter().cloned())
            .collect();

        actors.sort();
        actors.dedup();
        actors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // Clear function tests
    // ==================

    #[test]
    fn test_clear_resets_tasks() {
        let mut db = JourneyDb::new();
        db.add_section("weekends skip test");
        db.add_task("test1", "4: id1, id3");
        db.add_task("test2", "2: id2");
        db.clear();
        assert!(db.get_tasks().is_empty());
    }

    #[test]
    fn test_clear_resets_acc_title() {
        let mut db = JourneyDb::new();
        db.set_acc_title("Test Title");
        db.clear();
        assert_eq!(db.get_acc_title(), "");
    }

    #[test]
    fn test_clear_resets_acc_description() {
        let mut db = JourneyDb::new();
        db.set_acc_description("Test Description");
        db.clear();
        assert_eq!(db.get_acc_description(), "");
    }

    #[test]
    fn test_clear_resets_sections() {
        let mut db = JourneyDb::new();
        db.add_section("weekends skip test");
        db.clear();
        assert!(db.get_sections().is_empty());
    }

    #[test]
    fn test_clear_resets_actors() {
        let mut db = JourneyDb::new();
        db.add_section("weekends skip test");
        db.add_task("test1", "3: id1, id3");
        db.add_task("test2", "1: id2");
        db.clear();
        assert!(db.get_actors().is_empty());
    }

    // ==================
    // Full journey test
    // ==================

    #[test]
    fn test_tasks_and_actors_added() {
        let mut db = JourneyDb::new();
        db.set_acc_title("Shopping");
        db.set_acc_description("A user journey for family shopping");
        db.add_section("Journey to the shops");
        db.add_task("Get car keys", ":5:Dad");
        db.add_task("Go to car", ":3:Dad, Mum, Child#1, Child#2");
        db.add_task("Drive to supermarket", ":4:Dad");
        db.add_section("Do shopping");
        db.add_task("Go shopping", ":5:Mum");

        assert_eq!(db.get_acc_title(), "Shopping");
        assert_eq!(
            db.get_acc_description(),
            "A user journey for family shopping"
        );

        let tasks = db.get_tasks();
        assert_eq!(tasks.len(), 4);

        // Check first task
        assert_eq!(tasks[0].score, 5);
        assert_eq!(tasks[0].people, vec!["Dad"]);
        assert_eq!(tasks[0].section, "Journey to the shops");
        assert_eq!(tasks[0].task, "Get car keys");
        assert_eq!(tasks[0].task_type, "Journey to the shops");

        // Check second task
        assert_eq!(tasks[1].score, 3);
        assert_eq!(tasks[1].people, vec!["Dad", "Mum", "Child#1", "Child#2"]);
        assert_eq!(tasks[1].section, "Journey to the shops");
        assert_eq!(tasks[1].task, "Go to car");

        // Check third task
        assert_eq!(tasks[2].score, 4);
        assert_eq!(tasks[2].people, vec!["Dad"]);
        assert_eq!(tasks[2].section, "Journey to the shops");
        assert_eq!(tasks[2].task, "Drive to supermarket");

        // Check fourth task
        assert_eq!(tasks[3].score, 5);
        assert_eq!(tasks[3].people, vec!["Mum"]);
        assert_eq!(tasks[3].section, "Do shopping");
        assert_eq!(tasks[3].task, "Go shopping");
        assert_eq!(tasks[3].task_type, "Do shopping");

        // Check actors are sorted
        assert_eq!(db.get_actors(), vec!["Child#1", "Child#2", "Dad", "Mum"]);

        // Check sections
        assert_eq!(db.get_sections(), &["Journey to the shops", "Do shopping"]);
    }

    // ==================
    // Additional tests
    // ==================

    #[test]
    fn test_task_data_without_leading_colon() {
        let mut db = JourneyDb::new();
        db.add_section("Test");
        db.add_task("test1", "4: id1, id3");
        db.add_task("test2", "2: id2");

        let tasks = db.get_tasks();
        assert_eq!(tasks[0].score, 4);
        assert_eq!(tasks[0].people, vec!["id1", "id3"]);
        assert_eq!(tasks[1].score, 2);
        assert_eq!(tasks[1].people, vec!["id2"]);
    }

    #[test]
    fn test_duplicate_actors_are_deduplicated() {
        let mut db = JourneyDb::new();
        db.add_section("Test");
        db.add_task("task1", ":5:Alice, Bob");
        db.add_task("task2", ":4:Bob, Charlie");
        db.add_task("task3", ":3:Alice");

        assert_eq!(db.get_actors(), vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_empty_section() {
        let mut db = JourneyDb::new();
        db.add_section("Empty Section");
        // No tasks added

        assert_eq!(db.get_sections(), &["Empty Section"]);
        assert!(db.get_tasks().is_empty());
        assert!(db.get_actors().is_empty());
    }

    #[test]
    fn test_multiple_sections_with_same_actors() {
        let mut db = JourneyDb::new();
        db.add_section("Section 1");
        db.add_task("Task A", ":5:User1");
        db.add_section("Section 2");
        db.add_task("Task B", ":3:User1, User2");
        db.add_section("Section 3");
        db.add_task("Task C", ":4:User2");

        assert_eq!(db.get_actors(), vec!["User1", "User2"]);
        assert_eq!(db.get_sections(), &["Section 1", "Section 2", "Section 3"]);
    }
}
